use crate::{model::{now, ProviderState, Settings, Snapshot, PROVIDERS}, providers};
use std::{path::PathBuf, sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}}};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder, image::Image,
    menu::{Menu, MenuItem}, tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent}};

struct Inner {
    snapshot: Mutex<Snapshot>,
    gate: tokio::sync::Mutex<()>,
    data_dir: PathBuf,
    last_attempt: AtomicU64,
    failures: AtomicU64,
    mini: crate::minibar::MiniBar,
}
type Shared = Arc<Inner>;

fn name(id: &str) -> &str {
    match id { "claude" => "Claude", "codex" => "Codex", "antigravity" => "Antigravity", "grok" => "Grok", _ => "AI Token" }
}

pub(crate) fn service_icon(id: &str, dark: bool) -> Image<'static> {
    static ICONS: std::sync::OnceLock<Vec<Image<'static>>> = std::sync::OnceLock::new();
    let icons = ICONS.get_or_init(|| [
        include_bytes!("../../public/brands/claude.png").as_slice(),
        include_bytes!("../../public/brands/codex.png").as_slice(),
        include_bytes!("../../public/brands/antigravity.png").as_slice(),
        include_bytes!("../../public/brands/grok.png").as_slice(),
        include_bytes!("../../public/brands/codex-dark.png").as_slice(),
    ].iter().map(|bytes| Image::from_bytes(bytes).expect("서비스 아이콘 PNG")).collect());
    let index = match id { "claude"=>0, "codex" if dark=>4, "codex"=>1, "antigravity"=>2, _=>3 };
    icons[index].clone()
}


pub(crate) fn show_dashboard(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("dashboard") { let _ = window.show(); let _ = window.set_focus(); return; }
    if let Ok(window) = WebviewWindowBuilder::new(app, "dashboard", WebviewUrl::App("index.html".into()))
        .title("AI Token").inner_size(780.0, 760.0).min_inner_size(620.0, 560.0).build() {
        let _ = window.set_focus();
    }
}

fn tray_tooltip(snapshot: &Snapshot, time: u64) -> String {
    let mut text = String::from("AI Token");
    for id in &snapshot.settings.mini_providers {
        if let Some(p) = snapshot.providers.iter().find(|p| &p.id==id) {
            let summary = p.summary(time);
            text.push_str(&format!("\n{} · {}",name(id),if summary.is_empty() { "최신 데이터 없음" } else { &summary }));
        }
    }
    // Windows notification-area tooltips are limited to 127 UTF-16 code units.
    let mut units=0;
    text.chars().take_while(|c| { units+=c.len_utf16(); units<=127 }).collect()
}

fn publish(app: &tauri::AppHandle, state: &Shared) {
    let snapshot = state.snapshot.lock().unwrap().clone();
    let time = now();
    if let Some(tray) = app.tray_by_id("ai-token") {
        let _ = tray.set_tooltip(Some(tray_tooltip(&snapshot,time)));
    }
    state.mini.update(&snapshot);
    let _ = app.emit("quota-updated", &snapshot);
}

async fn refresh(app: tauri::AppHandle, state: Shared, force: bool) {
    let Ok(_guard) = state.gate.try_lock() else { return };
    let time = now();
    if !force && time.saturating_sub(state.last_attempt.load(Ordering::Relaxed)) < 10 { return; }
    state.last_attempt.store(time, Ordering::Relaxed);
    let settings = {
        let mut data = state.snapshot.lock().unwrap(); data.refreshing = true; data.settings.clone()
    };
    publish(&app, &state);
    let mut codex = if settings.codex_enabled { providers::fetch_codex().await } else { ProviderState::empty("codex", "disabled", "설정에서 Codex 연결을 켜세요.") };
    let claude = if settings.claude_enabled { providers::fetch_claude(&state.data_dir.join("claude-usage.json")) } else { ProviderState::empty("claude", "disabled", "설정에서 상태줄 연결 안내를 확인하세요.") };
    if settings.codex_enabled && codex.status != "ready" { state.failures.fetch_add(1, Ordering::Relaxed); } else { state.failures.store(0, Ordering::Relaxed); }
    {
        let mut data = state.snapshot.lock().unwrap();
        if data.settings.codex_enabled != settings.codex_enabled || data.settings.claude_enabled != settings.claude_enabled {
            data.refreshing = false; drop(data); publish(&app, &state); return;
        }
        if codex.status == "error" {
            if let Some(old) = data.providers.iter().find(|p| p.id == "codex") {
                codex.windows = old.windows.clone(); codex.fetched_at = old.fetched_at;
            }
        }
        data.providers = vec![claude, codex,
            ProviderState::empty("antigravity", "unsupported", "외부 한도 조회 경로를 검증 중입니다."),
            ProviderState::empty("grok", "unsupported", "웹 구독 한도 연동 경로가 아직 확인되지 않았습니다.")];
        data.refreshing = false;
    }
    publish(&app, &state);
}

#[tauri::command]
fn get_snapshot(state: tauri::State<'_, Shared>) -> Snapshot { state.snapshot.lock().unwrap().clone() }

#[tauri::command]
async fn refresh_now(app: tauri::AppHandle, state: tauri::State<'_, Shared>) -> Result<(), String> {
    if now().saturating_sub(state.last_attempt.load(Ordering::Relaxed)) < 10 { return Err("잠시 기다린 후 다시 갱신해 주세요.".into()); }
    refresh(app, state.inner().clone(), false).await;
    Ok(())
}

#[tauri::command]
async fn save_settings(app: tauri::AppHandle, state: tauri::State<'_, Shared>, settings: Settings) -> Result<(), String> {
    settings.validate()?;
    std::fs::create_dir_all(&state.data_dir).map_err(|_| "설정 폴더 생성 실패")?;
    std::fs::write(state.data_dir.join("settings.json"), serde_json::to_vec_pretty(&settings).map_err(|_| "설정 변환 실패")?).map_err(|_| "설정 저장 실패")?;
    state.snapshot.lock().unwrap().settings = settings;
    publish(&app, state.inner());
    // Serialize behind any in-flight request before fetching the new configuration.
    { let _guard = state.gate.lock().await; }
    refresh(app, state.inner().clone(), true).await;
    Ok(())
}

pub(crate) fn mini_action(app: &tauri::AppHandle, action: &str, position: Option<(i32,i32)>) {
    if action == "show" { show_dashboard(app); return; }
    let state = app.state::<Shared>();
    let mut settings = state.snapshot.lock().unwrap().settings.clone();
    match action {
        "position" => if let Some((x,y)) = position { settings.mini_x=Some(x); settings.mini_y=Some(y); },
        "lock" => settings.mini_locked = !settings.mini_locked,
        "reset" => { settings.mini_x=None; settings.mini_y=None; },
        "hide" => settings.mini_enabled=false,
        "toggle" => { settings.mini_enabled=!settings.mini_enabled; if settings.mini_providers.is_empty() { settings.mini_providers.push("codex".into()); } },
        _ => return,
    }
    if std::fs::create_dir_all(&state.data_dir).is_ok() &&
        std::fs::write(state.data_dir.join("settings.json"),serde_json::to_vec_pretty(&settings).unwrap()).is_ok() {
        state.snapshot.lock().unwrap().settings=settings;
        publish(app,state.inner());
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show_dashboard(app)))
        .invoke_handler(tauri::generate_handler![get_snapshot, refresh_now, save_settings])
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let settings: Settings = std::fs::read(data_dir.join("settings.json")).ok()
                .and_then(|b| serde_json::from_slice::<Settings>(&b).ok()).filter(|s| s.validate().is_ok()).unwrap_or_default();
            // Embed the bridge so moving the EXE never depends on a developer checkout.
            let bridge = data_dir.join("claude-statusline.ps1");
            let bridge_bytes = include_bytes!("../../scripts/claude-statusline.ps1");
            std::fs::create_dir_all(&data_dir)?;
            if std::fs::read(&bridge).ok().as_deref() != Some(bridge_bytes.as_slice()) {
                let temporary = data_dir.join("claude-statusline.update.ps1");
                std::fs::write(&temporary,bridge_bytes)?;
                std::fs::rename(&temporary,&bridge)?;
            }
            let state = Arc::new(Inner {
                snapshot: Mutex::new(Snapshot {
                    providers: PROVIDERS.iter().map(|id| ProviderState::empty(id, "unavailable", "아직 조회하지 않았습니다.")).collect(),
                    settings, refreshing: false,
                    claude_path: data_dir.join("claude-usage.json").to_string_lossy().into(),
                    bridge_command: format!("powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File \"{}\" -OutputPath \"{}\"", bridge.display(), data_dir.join("claude-usage.json").display()),
                }), gate: tokio::sync::Mutex::new(()), data_dir, last_attempt: AtomicU64::new(0), failures: AtomicU64::new(0), mini: crate::minibar::MiniBar::new(app.handle().clone()),
            });
            app.manage(state.clone());
            {
                let show = MenuItem::with_id(app, "show", "사용량 보기", true, None::<&str>)?;
                let quit = MenuItem::with_id(app, "quit", "AI Token 종료", true, None::<&str>)?;
                let mini = MenuItem::with_id(app, "mini", "미니바 켜기 / 끄기", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show, &mini, &quit])?;
                TrayIconBuilder::with_id("ai-token").icon(Image::from_bytes(include_bytes!("../icons/32x32.png"))?).tooltip("AI Token · 조회 대기")
                    .menu(&menu).show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id.as_ref() { "show" => show_dashboard(app), "mini" => mini_action(app,"toggle",None), "quit" => app.exit(0), _ => {} })
                    .on_tray_icon_event(|tray, event| { if matches!(event, TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. }) { show_dashboard(tray.app_handle()); } })
                    .build(app)?;
            }
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                refresh(handle.clone(), state.clone(), true).await;
                let mut last_claude = None;
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    let settings = state.snapshot.lock().unwrap().settings.clone();
                    // File metadata only; no browser, no constant CLI process, no AI calls.
                    if settings.claude_enabled {
                        let modified = std::fs::metadata(state.data_dir.join("claude-usage.json")).and_then(|m| m.modified()).ok();
                        if modified != last_claude {
                            last_claude = modified;
                            let claude = providers::fetch_claude(&state.data_dir.join("claude-usage.json"));
                            if let Some(p) = state.snapshot.lock().unwrap().providers.iter_mut().find(|p| p.id == "claude") { *p = claude; }
                        }
                    }
                    let backoff = 1u64 << state.failures.load(Ordering::Relaxed).min(3);
                    let interval = (settings.refresh_minutes * 60 * backoff).min(3600);
                    if settings.codex_enabled && now().saturating_sub(state.last_attempt.load(Ordering::Relaxed)) >= interval {
                        refresh(handle.clone(), state.clone(), false).await;
                    } else { publish(&handle, &state); }
                }
            });
            // Tray-only by default. --show is useful for first-run verification.
            if std::env::args().any(|arg| arg == "--show") { show_dashboard(app.handle()); }
            Ok(())
        })
        .build(tauri::generate_context!()).expect("AI Token 초기화 실패")
        .run(|_, event| { if let tauri::RunEvent::ExitRequested { api, code: None, .. } = event { api.prevent_exit(); } });
}

#[cfg(test)]
mod tray_tests {
    use super::*;
    fn snapshot() -> Snapshot {
        Snapshot { providers: PROVIDERS.iter().map(|id|ProviderState::empty(id,"unavailable","")).collect(),
            settings: Settings::default(), refreshing:false, claude_path:String::new(), bridge_command:String::new() }
    }
    #[test] fn tooltip_only_includes_selected_services() {
        let text=tray_tooltip(&snapshot(),1);
        assert!(text.starts_with("AI Token")); assert!(text.contains("Codex"));
        assert!(!text.contains("Claude")); assert!(!text.contains("Grok")); assert!(!text.contains("Antigravity"));
    }
    #[test] fn tooltip_respects_windows_limit_without_breaking_unicode() {
        let mut s=snapshot(); s.settings.mini_providers=PROVIDERS.iter().map(|id|id.to_string()).collect();
        for p in &mut s.providers { p.status="ready".into(); p.fetched_at=Some(1);
            p.windows=vec![crate::model::QuotaWindow { label:"긴 이름 🐱".repeat(40), used_percent:30.0, resets_at:None }]; }
        let text=tray_tooltip(&s,1); assert!(text.encode_utf16().count()<=127); assert!(!text.contains('�'));
    }
}

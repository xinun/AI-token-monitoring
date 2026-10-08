use crate::{model::{now, ProviderState, Settings, Snapshot, PROVIDERS}, providers, updates};
use tauri_plugin_updater::UpdaterExt;
use tauri_plugin_opener::OpenerExt;
use std::time::{Duration, Instant};
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
    update_gate: tokio::sync::Mutex<()>,
    pending_update: Mutex<Option<tauri_plugin_updater::Update>>,
    last_update_attempt: AtomicU64,
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

fn update_failure(app: &tauri::AppHandle, state: &Shared, message: &str) {
    {
        let mut snapshot = state.snapshot.lock().unwrap();
        snapshot.updates.status = "error".into();
        snapshot.updates.checking = false;
        snapshot.updates.message = message.into();
    }
    publish(app, state);
}

async fn check_updates(app: tauri::AppHandle, state: Shared, manual: bool) -> Result<(), String> {
    let Ok(_guard) = state.update_gate.try_lock() else { return Err("업데이트 작업이 진행 중입니다.".into()); };
    if !manual && !state.snapshot.lock().unwrap().settings.update_check_enabled { return Ok(()); }
    let time = now();
    if !updates::check_due(state.last_update_attempt.load(Ordering::Relaxed), time, manual) {
        return Err("잠시 기다린 후 다시 확인해 주세요. 수동 확인은 1분 간격으로 할 수 있습니다.".into());
    }
    state.last_update_attempt.store(time, Ordering::Relaxed);
    {
        let mut snapshot = state.snapshot.lock().unwrap();
        snapshot.updates.checking = true;
        if snapshot.updates.available_version.is_none() { snapshot.updates.status = "checking".into(); }
        snapshot.updates.message = "새 버전을 확인하고 있습니다.".into();
    }
    publish(&app, &state);
    let result = match app.updater_builder().timeout(Duration::from_secs(20))
        .version_comparator(|current, remote| updates::newer_stable(&current, &remote.version)).build() {
        Ok(updater) => updater.check().await.map_err(|_| ()),
        Err(_) => Err(()),
    };
    let result = match result {
        Ok(Some(update)) if !updates::trusted_installer(&update.download_url, &update.version) => Err(()),
        other => other,
    };
    match result {
        Ok(update) => {
            {
                let mut snapshot = state.snapshot.lock().unwrap();
                let info = &mut snapshot.updates;
                info.checking = false;
                info.checked_at = Some(now());
                info.downloaded_bytes = 0;
                info.total_bytes = None;
                info.available_version = update.as_ref().map(|u| u.version.clone());
                info.release_notes = update.as_ref().and_then(|u| u.body.as_ref().map(|b| b.chars().take(4000).collect()));
                info.status = if update.is_some() { "available" } else { "upToDate" }.into();
                info.message = if update.is_some() { "새 버전이 있습니다. 업데이트 설치를 선택하면 적용합니다." } else { "현재 최신 정식 버전을 사용하고 있습니다." }.into();
            }
            *state.pending_update.lock().unwrap() = update;
            publish(&app, &state);
            Ok(())
        }
        Err(()) => {
            let message = "업데이트 정보를 확인하지 못했습니다. 인터넷 연결을 확인하거나 나중에 다시 시도해 주세요.";
            update_failure(&app, &state, message);
            Err(message.into())
        }
    }
}

#[tauri::command]
async fn check_for_updates(app: tauri::AppHandle, state: tauri::State<'_, Shared>) -> Result<(), String> {
    check_updates(app, state.inner().clone(), true).await
}

#[tauri::command]
async fn open_release_page(app: tauri::AppHandle, state: tauri::State<'_, Shared>) -> Result<(), String> {
    let version = {
        let pending = state.pending_update.lock().unwrap();
        match pending.as_ref() {
            Some(update) if updates::trusted_installer(&update.download_url, &update.version) => Some(update.version.clone()),
            Some(_) => return Err("릴리즈 주소를 확인하지 못했습니다. 새 버전을 다시 확인해 주세요.".into()),
            None => None,
        }
    };
    let target = updates::release_page(version.as_deref()).map_err(|_| "릴리즈 주소를 확인하지 못했습니다.")?;
    app.opener().open_url(target, None::<&str>).map_err(|_| "브라우저를 열지 못했습니다. 기본 브라우저 설정을 확인해 주세요.".into())
}

#[tauri::command]
async fn install_update(app: tauri::AppHandle, state: tauri::State<'_, Shared>, expected_version: String) -> Result<(), String> {
    let state = state.inner().clone();
    let Ok(_guard) = state.update_gate.try_lock() else { return Err("업데이트 작업이 진행 중입니다.".into()); };
    let mut update = state.pending_update.lock().unwrap().clone().ok_or("먼저 새 버전을 확인해 주세요.")?;
    if !updates::approved_installer(&update.download_url, &update.version, &expected_version) {
        return Err("업데이트 정보가 변경되었습니다. 버전을 다시 확인하고 설치를 선택해 주세요.".into());
    }
    update.timeout = Some(Duration::from_secs(300));
    {
        let mut snapshot = state.snapshot.lock().unwrap();
        snapshot.updates.status = "downloading".into();
        snapshot.updates.checking = false;
        snapshot.updates.downloaded_bytes = 0;
        snapshot.updates.total_bytes = None;
        snapshot.updates.message = "업데이트를 다운로드하고 있습니다.".into();
    }
    publish(&app, &state);
    let mut downloaded = 0u64;
    let mut last_event = Instant::now();
    let bytes = match update.download(|chunk, total| {
        downloaded = downloaded.saturating_add(chunk as u64);
        if last_event.elapsed() >= Duration::from_millis(250) || total.is_some_and(|t| downloaded >= t) {
            {
                let mut snapshot = state.snapshot.lock().unwrap();
                snapshot.updates.downloaded_bytes = downloaded;
                snapshot.updates.total_bytes = total;
            }
            publish(&app, &state);
            last_event = Instant::now();
        }
    }, || {}).await {
        Ok(bytes) => bytes,
        Err(_) => {
            let message = "업데이트 다운로드 또는 파일 검증에 실패했습니다. 다시 시도해 주세요.";
            update_failure(&app, &state, message);
            return Err(message.into());
        }
    };
    // download() only returns after verifying the key AND signed version.
    {
        let mut snapshot = state.snapshot.lock().unwrap();
        snapshot.updates.status = "installing".into();
        snapshot.updates.downloaded_bytes = bytes.len() as u64;
        snapshot.updates.total_bytes = Some(bytes.len() as u64);
        snapshot.updates.message = "검증된 업데이트를 설치합니다. 앱이 종료된 뒤 다시 실행됩니다.".into();
    }
    publish(&app, &state);
    // Windows' updater launches NSIS and exits; the installer restarts the app.
    // Keep blocking file extraction off the async worker and avoid a second restart.
    match tauri::async_runtime::spawn_blocking(move || update.install(bytes)).await {
        Ok(Ok(())) => {
            #[cfg(not(windows))]
            app.restart();
            Ok(())
        }
        _ => {
            let message = "업데이트 설치를 시작하지 못했습니다. 다시 시도해 주세요.";
            update_failure(&app, &state, message);
            Err(message.into())
        }
    }
}

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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::Builder::new().open_js_links_on_click(false).build())
        .invoke_handler(tauri::generate_handler![get_snapshot, refresh_now, save_settings, check_for_updates, open_release_page, install_update])
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
                    updates: updates::UpdateState::default(),
                }), gate: tokio::sync::Mutex::new(()), data_dir, last_attempt: AtomicU64::new(0), failures: AtomicU64::new(0), mini: crate::minibar::MiniBar::new(app.handle().clone()),
                update_gate: tokio::sync::Mutex::new(()), pending_update: Mutex::new(None), last_update_attempt: AtomicU64::new(0),
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
            let update_handle = handle.clone();
            let update_state = state.clone();
            tauri::async_runtime::spawn(async move { let _ = check_updates(update_handle, update_state, false).await; });
            tauri::async_runtime::spawn(async move {
                refresh(handle.clone(), state.clone(), true).await;
                let mut last_claude = None;
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    let settings = state.snapshot.lock().unwrap().settings.clone();
                    if settings.update_check_enabled && updates::check_due(state.last_update_attempt.load(Ordering::Relaxed), now(), false) {
                        let handle = handle.clone(); let state = state.clone();
                        tauri::async_runtime::spawn(async move { let _ = check_updates(handle, state, false).await; });
                    }
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
            settings: Settings::default(), refreshing:false, claude_path:String::new(), bridge_command:String::new(), updates: updates::UpdateState::default() }
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

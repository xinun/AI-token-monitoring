use crate::model::{now, ProviderState, QuotaWindow};
use serde_json::{json, Value};
use std::{path::{Path, PathBuf}, process::Stdio};
use tokio::{io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader}, process::Command, time::{timeout, Duration}};

pub fn codex_executable() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("AI_TOKEN_CODEX_PATH").map(PathBuf::from).filter(|p| p.is_file()) { return Some(p); }
    let name = if cfg!(windows) { "codex.exe" } else { "codex" };
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) { let p = dir.join(name); if p.is_file() { return Some(p); } }
    }
    if cfg!(windows) {
        let base = PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("OpenAI/Codex/bin");
        let mut candidates: Vec<_> = std::fs::read_dir(base).ok()?.flatten().map(|entry| entry.path().join(name)).filter(|p| p.is_file()).collect();
        candidates.sort_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
        return candidates.pop();
    }
    None
}

async fn read_response<R: tokio::io::AsyncRead + Unpin>(reader: &mut BufReader<R>, id: u64) -> Result<Value, String> {
    // Bound each protocol line and discard notifications; never store raw account data.
    for _ in 0..128 {
        let mut line = String::new();
        let size = (&mut *reader).take(1_048_577).read_line(&mut line).await.map_err(|_| "Codex 응답을 읽지 못했습니다.")?;
        if size == 0 { return Err("Codex 연결이 종료되었습니다.".into()); }
        if size > 1_048_576 { return Err("Codex 응답 크기 제한을 초과했습니다.".into()); }
        let Ok(value) = serde_json::from_str::<Value>(&line) else { continue };
        if value.get("id").and_then(Value::as_u64) == Some(id) {
            if value.get("error").is_some() { return Err("Codex 한도를 조회하지 못했습니다. CLI 로그인과 버전을 확인하세요.".into()); }
            return value.get("result").cloned().ok_or_else(|| "Codex 응답 형식이 올바르지 않습니다.".into());
        }
    }
    Err("Codex 응답 횟수 제한을 초과했습니다.".into())
}

pub fn parse_codex(value: &Value) -> Vec<QuotaWindow> {
    let mut windows = vec![];
    let buckets: Vec<(String, &Value)> = if let Some(map) = value.get("rateLimitsByLimitId").and_then(Value::as_object).filter(|m| !m.is_empty()) {
        map.iter().map(|(id, v)| (v.get("limitName").and_then(Value::as_str).unwrap_or(id).to_string(), v)).collect()
    } else { value.get("rateLimits").map(|v| vec![("Codex".into(), v)]).unwrap_or_default() };
    for (bucket, data) in buckets {
        for (key, fallback) in [("primary", "단기"), ("secondary", "주간")] {
            let Some(window) = data.get(key).filter(|v| !v.is_null()) else { continue };
            let Some(used) = window.get("usedPercent").and_then(Value::as_f64).filter(|p| p.is_finite() && (0.0..=100.0).contains(p)) else { continue };
            let duration: String = match window.get("windowDurationMins").and_then(Value::as_u64) {
                Some(10080) => "주간".into(), Some(m) if m % 60 == 0 => format!("{}시간", m / 60), Some(m) => format!("{}분", m), None => fallback.into(),
            };
            windows.push(QuotaWindow { label: format!("{} · {}", bucket, duration), used_percent: used, resets_at: window.get("resetsAt").and_then(Value::as_u64) });
        }
    }
    windows
}

pub async fn fetch_codex() -> ProviderState {
    let Some(exe) = codex_executable() else { return ProviderState::empty("codex", "unavailable", "Codex CLI를 찾지 못했습니다. 설치 후 로그인해 주세요."); };
    let mut command = Command::new(exe);
    command.args(["app-server", "--stdio", "-c", "analytics.enabled=false"])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).kill_on_drop(true);
    #[cfg(windows)] command.creation_flags(0x08000000);
    let Ok(mut child) = command.spawn() else { return ProviderState::empty("codex", "error", "Codex CLI를 실행하지 못했습니다."); };
    let result = timeout(Duration::from_secs(20), async {
        let mut input = child.stdin.take().ok_or("Codex 입력 연결 실패")?;
        let mut output = BufReader::new(child.stdout.take().ok_or("Codex 출력 연결 실패")?);
        let init = json!({"id": 1, "method": "initialize", "params": {"clientInfo": {"name": "ai_token", "title": "AI Token", "version": env!("CARGO_PKG_VERSION")}}});
        input.write_all(format!("{}\n", init).as_bytes()).await.map_err(|_| "Codex 초기화 실패")?;
        read_response(&mut output, 1).await?;
        input.write_all(b"{\"method\":\"initialized\",\"params\":{}}\n{\"id\":2,\"method\":\"account/rateLimits/read\"}\n").await.map_err(|_| "Codex 조회 요청 실패")?;
        read_response(&mut output, 2).await
    }).await;
    let _ = child.kill().await;
    let _ = child.wait().await;
    match result {
        Ok(Ok(value)) => {
            let windows = parse_codex(&value);
            if windows.is_empty() { return ProviderState::empty("codex", "unavailable", "계정에서 조회 가능한 구독 한도가 없습니다."); }
            ProviderState { id: "codex".into(), status: "ready".into(), message: "Codex 구독 한도 · ChatGPT 일반 채팅 한도와 별개".into(), windows, fetched_at: Some(now()), source: "Codex app-server".into() }
        },
        Ok(Err(message)) => ProviderState::empty("codex", "error", &message),
        Err(_) => ProviderState::empty("codex", "error", "Codex 조회 시간이 초과되었습니다. 나중에 다시 시도하세요."),
    }
}

pub fn parse_claude(value: &Value) -> Vec<QuotaWindow> {
    [("five_hour", "5시간"), ("seven_day", "주간")].iter().filter_map(|(key, label)| {
        let window = value.get("rate_limits")?.get(*key)?;
        let used = window.get("used_percentage")?.as_f64()?;
        if !used.is_finite() || !(0.0..=100.0).contains(&used) { return None; }
        Some(QuotaWindow { label: label.to_string(), used_percent: used, resets_at: window.get("resets_at").and_then(Value::as_u64) })
    }).collect()
}

pub fn fetch_claude(path: &Path) -> ProviderState {
    // This file is an explicit bridge output, not a credential or conversation file.
    let result = (|| {
        let file = std::fs::File::open(path).ok()?;
        if file.metadata().ok()?.len() > 65536 { return None; }
        let value: Value = serde_json::from_reader(file).ok()?;
        let time = value.get("captured_at")?.as_u64()?;
        if time > now() + 60 { return None; }
        Some((parse_claude(&value), time))
    })();
    match result {
        Some((windows, time)) if !windows.is_empty() => ProviderState { id: "claude".into(), status: "ready".into(), message: "Claude Code가 전달한 마지막 한도 · 미사용 중에는 갱신되지 않을 수 있음".into(), windows, fetched_at: Some(time), source: "Claude Code statusline".into() },
        _ => ProviderState::empty("claude", "unavailable", "Claude Code 상태줄 연결을 설정하고 한 번 사용해 주세요."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn codex_buckets_are_not_added_together() {
        let value = json!({"rateLimits": {"primary": {"usedPercent":99}}, "rateLimitsByLimitId": {"a": {"primary":{"usedPercent":20,"windowDurationMins":300}}, "b": {"secondary":{"usedPercent":60,"windowDurationMins":10080}}}});
        let windows = parse_codex(&value);
        assert_eq!(windows.len(), 2); assert_eq!(windows[0].used_percent, 20.0); assert_eq!(windows[1].used_percent, 60.0);
    }
    #[test] fn absent_is_not_zero() {
        assert!(parse_claude(&json!({"rate_limits":{"five_hour":null}})).is_empty());
        assert!(parse_codex(&json!({"rateLimits":{"primary":{"resetsAt":1}}})).is_empty());
    }
    #[test] fn claude_context_is_not_subscription_quota() {
        assert!(parse_claude(&json!({"context_window":{"used_percentage":20}})).is_empty());
        let windows = parse_claude(&json!({"rate_limits":{"five_hour":{"used_percentage":25,"resets_at":1000}}}));
        assert_eq!(windows.len(), 1); assert_eq!(windows[0].used_percent, 25.0);
    }
}

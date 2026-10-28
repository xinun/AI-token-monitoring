use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub const PROVIDERS: [&str; 4] = ["claude", "codex", "antigravity", "grok"];
pub fn now() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() }

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QuotaWindow {
    pub label: String,
    pub used_percent: f64,
    pub resets_at: Option<u64>,
}

impl QuotaWindow {
    pub fn remaining(&self, time: u64) -> Option<f64> {
        if !self.used_percent.is_finite() || !(0.0..=100.0).contains(&self.used_percent) { return None; }
        if self.resets_at.is_some_and(|reset| reset <= time) { return None; }
        Some(100.0 - self.used_percent)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProviderState {
    pub id: String,
    pub status: String,
    pub message: String,
    pub windows: Vec<QuotaWindow>,
    pub fetched_at: Option<u64>,
    pub source: String,
}

impl ProviderState {
    pub fn empty(id: &str, status: &str, message: &str) -> Self {
        Self { id: id.into(), status: status.into(), message: message.into(), windows: vec![], fetched_at: None, source: String::new() }
    }
    pub fn summary(&self, time: u64) -> String {
        if self.status != "ready" || self.fetched_at.map_or(true, |t| time.saturating_sub(t) > 900) {
            return "확인 불가 / 최신 데이터 없음".into();
        }
        self.windows.iter().filter_map(|w| w.remaining(time).map(|p| format!("{} 잔여 {:.0}%", w.label, p)))
            .collect::<Vec<_>>().join(" · ")
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub codex_enabled: bool,
    pub claude_enabled: bool,
    pub refresh_minutes: u64,
    pub mini_enabled: bool,
    pub mini_only_when_running: bool,
    pub mini_providers: Vec<String>,
    pub mini_locked: bool,
    pub mini_x: Option<i32>,
    pub mini_y: Option<i32>,
    pub update_check_enabled: bool,
    pub launch_at_login: bool,
}

impl Default for Settings {
    fn default() -> Self { Self { codex_enabled: true, claude_enabled: false, refresh_minutes: 5, mini_enabled: true, mini_only_when_running: false, mini_providers: vec!["codex".into()], mini_locked: true, mini_x: None, mini_y: None, update_check_enabled: true, launch_at_login: false } }
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if ![5, 10, 15].contains(&self.refresh_minutes) { return Err("갱신 주기는 5, 10, 15분 중 선택하세요.".into()); }
        if self.mini_providers.len() > 4 || self.mini_providers.iter().any(|id| !PROVIDERS.contains(&id.as_str())) { return Err("미니바 표시 항목이 올바르지 않습니다.".into()); }
        let mut seen = std::collections::HashSet::new();
        if self.mini_providers.iter().any(|id| !seen.insert(id)) { return Err("표시 항목이 중복되었습니다.".into()); }
        if self.mini_enabled && self.mini_providers.is_empty() { return Err("미니바에 표시할 AI를 하나 이상 선택하세요.".into()); }
        if self.mini_enabled && self.mini_only_when_running && !self.mini_providers.iter().any(|id| ["codex", "claude"].contains(&id.as_str())) {
            return Err("실행 중 자동 표시에는 Codex나 Claude를 하나 이상 선택하세요.".into());
        }
        Ok(())
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub providers: Vec<ProviderState>,
    pub settings: Settings,
    pub refreshing: bool,
    pub claude_path: String,
    pub bridge_command: String,
    pub updates: crate::updates::UpdateState,
    pub running_providers: Vec<String>,
}

impl Snapshot {
    pub fn visible_mini_providers(&self) -> Vec<&str> {
        if !self.settings.mini_enabled { return vec![]; }
        self.settings.mini_providers.iter()
            .filter(|id| !self.settings.mini_only_when_running || self.running_providers.contains(id))
            .map(String::as_str).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn reset_is_unknown_not_full() {
        let q = QuotaWindow { label: "5시간".into(), used_percent: 72.0, resets_at: Some(100) };
        assert_eq!(q.remaining(99), Some(28.0));
        assert_eq!(q.remaining(100), None);
    }
    #[test] fn invalid_percent_is_unknown() {
        for used in [-1.0, 101.0, f64::NAN] {
            assert_eq!(QuotaWindow { label: "x".into(), used_percent: used, resets_at: None }.remaining(1), None);
        }
    }
    #[test] fn stale_data_is_not_current() {
        let mut p = ProviderState::empty("codex", "ready", ""); p.fetched_at = Some(1);
        assert!(p.summary(1000).contains("확인 불가"));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MiniTone { Unknown, Normal, Low, Critical }

pub fn mini_lines(provider: Option<&ProviderState>, time: u64) -> Vec<(String, MiniTone)> {
    let Some(p) = provider else { return vec![("조회 대기".into(), MiniTone::Unknown)]; };
    let fresh = p.status == "ready" && p.fetched_at.is_some_and(|t| t <= time+60 && time.saturating_sub(t) <= 900);
    if p.windows.is_empty() {
        return vec![("—".into(), MiniTone::Unknown), (match p.status.as_str() { "disabled" => "연결 꺼짐", "unsupported" => "미연동", _ => "조회 대기" }.into(), MiniTone::Unknown)];
    }
    p.windows.iter().take(2).map(|w| {
        let label = w.label.rsplit('·').next().unwrap_or(&w.label).trim();
        let label = if p.windows.len()>2 && w.label == p.windows[1].label { format!("{} 외", label) } else { label.into() };
        let value = if fresh { w.remaining(time) } else { None };
        let tone = match value {
            Some(v) if v <= 10.0 => MiniTone::Critical,
            Some(v) if v <= 20.0 => MiniTone::Low,
            Some(_) => MiniTone::Normal,
            None => MiniTone::Unknown,
        };
        (format!("{} {}", label, value.map(|v| format!("{:.0}%", v)).unwrap_or_else(|| "—".into())), tone)
    }).collect()
}

#[cfg(test)]
mod mini_tests {
    use super::*;
    #[test] fn legacy_settings_preserve_connections() {
        let s: Settings = serde_json::from_str(r#"{"codexEnabled":false,"claudeEnabled":true,"refreshMinutes":10}"#).unwrap();
        assert!(!s.codex_enabled); assert!(s.claude_enabled); assert_eq!(s.refresh_minutes,10);
        assert_eq!(s.mini_providers,vec!["codex"]); assert!(s.validate().is_ok());
        assert!(!s.mini_only_when_running); assert!(!s.launch_at_login);
    }
    #[test] fn selections_are_validated() {
        let mut s = Settings::default(); s.mini_providers = vec!["unknown".into()]; assert!(s.validate().is_err());
        s.mini_providers = vec!["codex".into(),"codex".into()]; assert!(s.validate().is_err());
        s.mini_providers.clear(); assert!(s.validate().is_err()); s.mini_enabled=false; assert!(s.validate().is_ok());
    }
    #[test] fn mini_limits_stay_separate_and_stale_is_unknown() {
        let mut p = ProviderState::empty("codex","ready",""); p.fetched_at=Some(10);
        p.windows=vec![QuotaWindow { label:"Codex · 5시간".into(),used_percent:30.0,resets_at:Some(100) },QuotaWindow { label:"Codex · 주간".into(),used_percent:90.0,resets_at:Some(2000) }];
        assert_eq!(mini_lines(Some(&p),20),vec![("5시간 70%".into(),MiniTone::Normal),("주간 10%".into(),MiniTone::Critical)]);
        assert_eq!(mini_lines(Some(&p),100)[0].0,"5시간 —");
        assert_eq!(mini_lines(Some(&p),100)[0].1,MiniTone::Unknown);
        assert_eq!(mini_lines(Some(&p),1000)[1].0,"주간 —");
        assert!(mini_lines(Some(&p),1000).iter().all(|line| line.1 == MiniTone::Unknown));
        p.windows[0].resets_at=None;
        for (remaining,tone) in [(100.0,MiniTone::Normal),(20.1,MiniTone::Normal),(20.0,MiniTone::Low),(10.1,MiniTone::Low),(10.0,MiniTone::Critical),(0.0,MiniTone::Critical)] {
            p.windows[0].used_percent=100.0-remaining;
            assert_eq!(mini_lines(Some(&p),20)[0].1,tone,"remaining {remaining}");
        }
        p.windows[0].used_percent=f64::NAN;
        assert_eq!(mini_lines(Some(&p),20)[0].1,MiniTone::Unknown);
    }
    #[test] fn auto_visibility_filters_selected_services_and_respects_manual_off() {
        let mut snapshot = Snapshot { providers: vec![], settings: Settings::default(), refreshing: false, claude_path: String::new(), bridge_command: String::new(), updates: crate::updates::UpdateState::default(), running_providers: vec![] };
        assert_eq!(snapshot.visible_mini_providers(), ["codex"]);
        snapshot.settings.mini_only_when_running = true;
        assert!(snapshot.visible_mini_providers().is_empty());
        snapshot.running_providers = vec!["codex".into(), "claude".into()];
        assert_eq!(snapshot.visible_mini_providers(), ["codex"]);
        snapshot.settings.mini_providers.push("claude".into());
        assert_eq!(snapshot.visible_mini_providers(), ["codex", "claude"]);
        snapshot.running_providers.remove(0);
        assert_eq!(snapshot.visible_mini_providers(), ["claude"]);
        snapshot.settings.mini_enabled = false;
        assert!(snapshot.visible_mini_providers().is_empty());
    }
    #[test] fn unsupported_only_selection_cannot_enable_process_visibility() {
        let mut settings = Settings::default();
        settings.mini_only_when_running = true;
        settings.mini_providers = vec!["grok".into()];
        assert!(settings.validate().is_err());
        settings.mini_providers.push("codex".into());
        assert!(settings.validate().is_ok());
    }
}

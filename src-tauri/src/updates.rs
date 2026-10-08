use semver::Version;
use serde::Serialize;
use std::cmp::Ordering;
use url::Url;

pub const CHECK_INTERVAL: u64 = 6 * 60 * 60;
pub const MANUAL_COOLDOWN: u64 = 60;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateState {
    pub current_version: String,
    pub available_version: Option<String>,
    pub release_notes: Option<String>,
    pub status: String,
    pub checking: bool,
    pub checked_at: Option<u64>,
    pub message: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            current_version: env!("CARGO_PKG_VERSION").into(),
            available_version: None, release_notes: None,
            status: "idle".into(), checking: false, checked_at: None,
            message: "새 버전은 앱 시작 시와 6시간 간격으로 확인합니다.".into(),
            downloaded_bytes: 0, total_bytes: None,
        }
    }
}

pub fn newer_stable(current: &Version, remote: &Version) -> bool {
    remote.pre.is_empty() && remote.cmp_precedence(current) == Ordering::Greater
}

pub fn check_due(last_attempt: u64, time: u64, manual: bool) -> bool {
    last_attempt == 0 || time.saturating_sub(last_attempt) >= if manual { MANUAL_COOLDOWN } else { CHECK_INTERVAL }
}

// Restrict the initial URL to our exact signed NSIS asset. HTTPS redirects to
// GitHub's asset CDN remain handled by the updater's HTTP client.
pub fn trusted_installer(url: &Url, version: &str) -> bool {
    let Ok(version) = Version::parse(version) else { return false };
    if !version.pre.is_empty() { return false; }
    let expected = format!("/xinun/AI-token-monitoring/releases/download/v{version}/AI-Token-{version}-windows-x64-setup.exe");
    url.scheme() == "https" && url.host_str() == Some("github.com")
        && url.username().is_empty() && url.password().is_none()
        && url.port().is_none() && url.query().is_none() && url.fragment().is_none()
        && url.path() == expected
}

pub fn approved_installer(url: &Url, version: &str, approved_version: &str) -> bool {
    version == approved_version && trusted_installer(url, version)
}

// The browser destination is constructed here, never supplied by the webview.
pub fn release_page(version: Option<&str>) -> Result<String, ()> {
    match version {
        None => Ok("https://github.com/xinun/AI-token-monitoring/releases/latest".into()),
        Some(value) => {
            let version = Version::parse(value).map_err(|_| ())?;
            if !version.pre.is_empty() { return Err(()); }
            Ok(format!("https://github.com/xinun/AI-token-monitoring/releases/tag/v{version}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn version(value: &str) -> Version { Version::parse(value.trim_start_matches('v')).unwrap() }
    #[test]
    fn comparison_is_numeric_and_excludes_preview_versions() {
        assert!(newer_stable(&version("0.2.0"), &version("v0.10.0")));
        assert!(!newer_stable(&version("0.2.0"), &version("0.2.0")));
        assert!(!newer_stable(&version("0.2.0"), &version("0.1.9")));
        assert!(!newer_stable(&version("0.2.0"), &version("0.3.0-beta.1")));
        assert!(newer_stable(&version("0.2.0-beta.1"), &version("0.2.0")));
    }
    #[test]
    fn build_metadata_is_not_an_update() {
        assert!(!newer_stable(&version("0.2.0+build1"), &version("0.2.0+build2")));
    }
    #[test]
    fn cooldown_and_six_hour_schedule_are_separate() {
        assert!(check_due(0, 1, false));
        assert!(!check_due(100, 159, true));
        assert!(check_due(100, 160, true));
        assert!(!check_due(100, 160, false));
        assert!(check_due(100, 100 + CHECK_INTERVAL, false));
        assert!(!check_due(100, 99, true));
    }
    #[test]
    fn installer_links_are_repository_version_and_file_specific() {
        let good = "https://github.com/xinun/AI-token-monitoring/releases/download/v0.2.1/AI-Token-0.2.1-windows-x64-setup.exe";
        assert!(trusted_installer(&Url::parse(good).unwrap(), "0.2.1"));
        for value in [good.replace("https:", "http:"), good.replace("github.com", "github.com.evil.example"),
            good.replace("xinun", "someone"), good.replace("v0.2.1", "v0.2.0"),
            good.replace("setup.exe", "portable.zip"), format!("{good}?redirect=elsewhere"),
            good.replace("https://", "https://user:pass@"), format!("{good}#fragment")] {
            assert!(!trusted_installer(&Url::parse(&value).unwrap(), "0.2.1"), "{value}");
        }
        assert!(!trusted_installer(&Url::parse(good).unwrap(), "invalid"));
    }
    #[test]
    fn approval_cannot_install_a_different_candidate() {
        let url = Url::parse("https://github.com/xinun/AI-token-monitoring/releases/download/v0.2.2/AI-Token-0.2.2-windows-x64-setup.exe").unwrap();
        assert!(approved_installer(&url, "0.2.2", "0.2.2"));
        assert!(!approved_installer(&url, "0.2.2", "0.2.1"));
        assert!(!approved_installer(&url, "0.2.2", ""));
        assert!(!approved_installer(&url, "0.2.1", "0.2.1"));
    }
    #[test]
    fn release_page_is_fixed_to_our_repository_and_stable_tag() {
        assert_eq!(release_page(None).unwrap(), "https://github.com/xinun/AI-token-monitoring/releases/latest");
        assert_eq!(release_page(Some("0.2.1")).unwrap(), "https://github.com/xinun/AI-token-monitoring/releases/tag/v0.2.1");
        for value in ["0.2.1-beta.1", "https://example.com", "../../elsewhere", "0.2.1?redirect=bad", ""] {
            assert!(release_page(Some(value)).is_err());
        }
    }
}

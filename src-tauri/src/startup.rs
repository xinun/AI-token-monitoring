//! Windows login registration for this user only. Never writes machine-wide keys.

fn command_for_path(path: &str) -> Result<String, String> {
    if path.is_empty() || path.contains(['"', '\0', '\r', '\n']) {
        return Err("자동 실행 경로가 올바르지 않습니다.".into());
    }
    let command = format!("\"{path}\" --autostart");
    if command.encode_utf16().count() > 260 { return Err("자동 실행 경로가 너무 깁니다. 더 짧은 설치 경로를 사용해 주세요.".into()); }
    Ok(command)
}

pub fn current_command() -> Result<String, String> {
    let path = std::env::current_exe().map_err(|_| "자동 실행 경로를 확인하지 못했습니다.")?;
    command_for_path(path.to_str().ok_or("자동 실행 경로를 읽지 못했습니다.")?)
}

#[cfg(windows)]
mod platform {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::{Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, ERROR_SUCCESS}, System::Registry::*};
    const RUN: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    const VALUE: &str = "AI Token";
    fn wide(value: &str) -> Vec<u16> { value.encode_utf16().chain(Some(0)).collect() }
    fn missing(error: u32) -> bool { [ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND].contains(&error) }
    struct Key(HKEY);
    impl Drop for Key { fn drop(&mut self) { unsafe { RegCloseKey(self.0); } } }

    pub fn registration() -> Result<Option<String>, String> {
        unsafe {
            let mut handle = null_mut();
            let result = RegOpenKeyExW(HKEY_CURRENT_USER, wide(RUN).as_ptr(), 0, KEY_QUERY_VALUE, &mut handle);
            if missing(result) { return Ok(None); }
            if result != ERROR_SUCCESS { return Err("Windows 자동 실행 상태를 읽지 못했습니다.".into()); }
            let key = Key(handle);
            let name = wide(VALUE);
            let mut kind = 0;
            let mut bytes = 0;
            let result = RegQueryValueExW(key.0, name.as_ptr(), null(), &mut kind, null_mut(), &mut bytes);
            if missing(result) { return Ok(None); }
            if result != ERROR_SUCCESS || kind != REG_SZ || bytes > 65_536 || bytes % 2 != 0 {
                return Err("Windows 자동 실행 등록 형식을 확인하지 못했습니다.".into());
            }
            let mut value = vec![0u16; bytes as usize / 2];
            let result = RegQueryValueExW(key.0, name.as_ptr(), null(), &mut kind, value.as_mut_ptr().cast(), &mut bytes);
            if result != ERROR_SUCCESS || kind != REG_SZ { return Err("Windows 자동 실행 상태를 읽지 못했습니다.".into()); }
            value.truncate(bytes as usize / 2);
            if value.last() == Some(&0) { value.pop(); }
            String::from_utf16(&value).map(Some).map_err(|_| "Windows 자동 실행 경로를 읽지 못했습니다.".into())
        }
    }

    pub fn write_registration(value: Option<&str>) -> Result<(), String> {
        unsafe {
            let mut handle = null_mut();
            let result = if value.is_some() {
                RegCreateKeyExW(HKEY_CURRENT_USER, wide(RUN).as_ptr(), 0, null(), REG_OPTION_NON_VOLATILE, KEY_SET_VALUE, null(), &mut handle, null_mut())
            } else {
                RegOpenKeyExW(HKEY_CURRENT_USER, wide(RUN).as_ptr(), 0, KEY_SET_VALUE, &mut handle)
            };
            if value.is_none() && missing(result) { return Ok(()); }
            if result != ERROR_SUCCESS { return Err("Windows 자동 실행 등록을 변경하지 못했습니다.".into()); }
            let key = Key(handle);
            let name = wide(VALUE);
            let result = match value {
                Some(value) => {
                    let data = wide(value);
                    RegSetValueExW(key.0, name.as_ptr(), 0, REG_SZ, data.as_ptr().cast(), (data.len() * 2) as u32)
                }
                None => RegDeleteValueW(key.0, name.as_ptr()),
            };
            if result == ERROR_SUCCESS || value.is_none() && missing(result) { Ok(()) }
            else { Err("Windows 자동 실행 등록을 변경하지 못했습니다.".into()) }
        }
    }
}

#[cfg(windows)]
pub use platform::{registration, write_registration};
#[cfg(not(windows))]
pub fn registration() -> Result<Option<String>, String> { Ok(None) }
#[cfg(not(windows))]
pub fn write_registration(value: Option<&str>) -> Result<(), String> {
    if value.is_some() { Err("로그인 시 자동 실행은 현재 Windows에서 지원합니다.".into()) } else { Ok(()) }
}

// Keep the OS registration and the settings file in agreement. Only an explicit
// settings save calls this; startup itself never enables autostart.
pub fn save_with_autostart(
    current: bool,
    desired: bool,
    mut set_enabled: impl FnMut(bool) -> Result<(), String>,
    persist: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    if current != desired { set_enabled(desired)?; }
    if let Err(error) = persist() {
        if current != desired && set_enabled(current).is_err() {
            return Err(format!("{error} 자동 실행 설정 복원에도 실패했습니다. 설정을 다시 확인해 주세요."));
        }
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_command_quotes_spaces_and_korean_paths() {
        assert_eq!(command_for_path(r"C:\Users\사용자\AppData\Local\AI Token\ai-token.exe").unwrap(), "\"C:\\Users\\사용자\\AppData\\Local\\AI Token\\ai-token.exe\" --autostart");
        for path in ["", "bad\"path", "bad\0path", "bad\npath"] { assert!(command_for_path(path).is_err()); }
        assert!(command_for_path(&"x".repeat(260)).is_err());
    }
    #[test]
    fn unchanged_startup_setting_does_not_touch_os_registration() {
        assert!(save_with_autostart(false, false, |_| panic!("must not register"), || Ok(())).is_ok());
    }
    #[test]
    fn file_failure_restores_previous_os_setting() {
        let mut changes = vec![];
        assert!(save_with_autostart(false, true, |enabled| { changes.push(enabled); Ok(()) }, || Err("write failed".into())).is_err());
        assert_eq!(changes, [true, false]);
    }
    #[test]
    fn registration_failure_does_not_save_successful_settings() {
        assert!(save_with_autostart(false, true, |_| Err("registry denied".into()), || panic!("must not persist")).is_err());
    }
    #[test]
    fn disabling_and_rollback_errors_are_reported() {
        let mut changes = vec![];
        assert!(save_with_autostart(true, false, |enabled| { changes.push(enabled); Ok(()) }, || Ok(())).is_ok());
        assert_eq!(changes, [false]);
        let error = save_with_autostart(false, true, |enabled| if enabled { Ok(()) } else { Err("rollback failed".into()) }, || Err("write failed".into())).unwrap_err();
        assert!(error.contains("복원에도 실패"));
    }
}

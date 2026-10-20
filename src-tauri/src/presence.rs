//! Process names and ancestry only: no command lines, credentials or usage queries.
use std::collections::{HashMap, HashSet};

struct ProcessEntry {
    pid: u32,
    parent_pid: u32,
    name: String,
    same_session: bool,
}

fn provider_for_name(name: &str) -> Option<&'static str> {
    if name.eq_ignore_ascii_case("codex.exe") { Some("codex") }
    else if name.eq_ignore_ascii_case("claude.exe") { Some("claude") }
    else { None }
}

fn running_from_processes(processes: &[ProcessEntry], own_pid: u32) -> Vec<String> {
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut pending = vec![own_pid];
    for process in processes {
        children.entry(process.parent_pid).or_default().push(process.pid);
        if process.name.eq_ignore_ascii_case("ai-token.exe") { pending.push(process.pid); }
    }
    let mut excluded = HashSet::new();
    while let Some(pid) = pending.pop() {
        if excluded.insert(pid) {
            if let Some(descendants) = children.get(&pid) { pending.extend(descendants); }
        }
    }
    let running: HashSet<_> = processes.iter()
        .filter(|process| process.same_session && !excluded.contains(&process.pid))
        .filter_map(|process| provider_for_name(&process.name)).collect();
    ["codex", "claude"].into_iter().filter(|id| running.contains(id)).map(String::from).collect()
}

// Errors retain the last successful result instead of hiding/showing the bar.
pub fn apply_observation(current: &mut Vec<String>, observation: std::io::Result<Vec<String>>) -> bool {
    match observation {
        Ok(next) if *current != next => { *current = next; true }
        _ => false,
    }
}

#[cfg(windows)]
pub fn read_running() -> std::io::Result<Vec<String>> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE, ERROR_NO_MORE_FILES},
        System::{Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS}, RemoteDesktop::ProcessIdToSessionId},
    };
    struct Handle(HANDLE);
    impl Drop for Handle { fn drop(&mut self) { unsafe { CloseHandle(self.0); } } }
    unsafe {
        let own_pid = std::process::id();
        let mut own_session = 0;
        if ProcessIdToSessionId(own_pid, &mut own_session) == 0 { return Err(std::io::Error::last_os_error()); }
        let handle = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if handle == INVALID_HANDLE_VALUE { return Err(std::io::Error::last_os_error()); }
        let snapshot = Handle(handle);
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut has_entry = Process32FirstW(snapshot.0, &mut entry);
        let mut processes = Vec::new();
        while has_entry != 0 {
            let end = entry.szExeFile.iter().position(|value| *value == 0).unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..end]);
            let mut session = 0;
            let same_session = provider_for_name(&name).is_some()
                && ProcessIdToSessionId(entry.th32ProcessID, &mut session) != 0 && session == own_session;
            processes.push(ProcessEntry { pid: entry.th32ProcessID, parent_pid: entry.th32ParentProcessID, name, same_session });
            has_entry = Process32NextW(snapshot.0, &mut entry);
        }
        let error = GetLastError();
        if error != ERROR_NO_MORE_FILES { return Err(std::io::Error::from_raw_os_error(error as i32)); }
        Ok(running_from_processes(&processes, own_pid))
    }
}

#[cfg(not(windows))]
pub fn read_running() -> std::io::Result<Vec<String>> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Process detection is currently Windows-only."))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn process(pid: u32, parent_pid: u32, name: &str) -> ProcessEntry {
        ProcessEntry { pid, parent_pid, name: name.into(), same_session: true }
    }
    #[test]
    fn quota_query_children_are_not_user_activity() {
        let mut processes = vec![process(10, 1, "ai-token.exe"), process(11, 10, "helper.exe"), process(12, 11, "codex.exe"),
            process(20, 1, "AI-TOKEN.EXE"), process(21, 20, "claude.exe")];
        assert!(running_from_processes(&processes, 10).is_empty());
        processes.push(process(30, 1, "Codex.exe"));
        processes.push(process(31, 30, "codex.exe"));
        processes.push(process(40, 1, "CLAUDE.EXE"));
        assert_eq!(running_from_processes(&processes, 10), ["codex", "claude"]);
    }
    #[test]
    fn unrelated_processes_and_other_sessions_do_not_show_ai_tools() {
        let mut processes = vec![process(2, 1, "node.exe"), process(3, 1, "chrome.exe"), process(4, 1, "wsl.exe"), process(5, 1, "my-codex.exe")];
        let mut other_session = process(6, 1, "codex.exe"); other_session.same_session = false;
        processes.push(other_session);
        assert!(running_from_processes(&processes, 10).is_empty());
    }
    #[test]
    fn missing_parent_and_cyclic_ancestry_are_bounded() {
        let processes = vec![process(11, 10, "helper.exe"), process(10, 11, "ai-token.exe"), process(12, 11, "codex.exe"), process(20, 999, "claude.exe")];
        assert_eq!(running_from_processes(&processes, 10), ["claude"]);
    }
    #[test]
    fn failed_or_unchanged_observation_does_not_toggle_visibility() {
        let mut current = vec!["codex".into()];
        assert!(!apply_observation(&mut current, Err(std::io::Error::other("snapshot failed"))));
        assert_eq!(current, ["codex"]);
        assert!(!apply_observation(&mut current, Ok(vec!["codex".into()])));
        assert!(apply_observation(&mut current, Ok(vec![])));
        assert!(current.is_empty());
    }
}

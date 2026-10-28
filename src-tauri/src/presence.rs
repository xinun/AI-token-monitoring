//! Process names, ancestry and a strict Codex desktop package identity only.
//! Never reads command lines, credentials, conversations or usage counters.
use std::collections::{HashMap, HashSet};

struct ProcessEntry {
    pid: u32,
    parent_pid: u32,
    name: String,
    same_session: bool,
    codex_desktop: bool,
}

fn provider_for_name(name: &str) -> Option<&'static str> {
    if name.eq_ignore_ascii_case("codex.exe") { Some("codex") }
    else if name.eq_ignore_ascii_case("claude.exe") { Some("claude") }
    else { None }
}

fn provider_for_process(process: &ProcessEntry) -> Option<&'static str> {
    provider_for_name(&process.name).or_else(|| {
        (process.codex_desktop && process.name.eq_ignore_ascii_case("ChatGPT.exe")).then_some("codex")
    })
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
        .filter_map(provider_for_process).collect();
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
fn is_packaged_codex_desktop(pid: u32) -> bool {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE, ERROR_SUCCESS},
        Storage::Packaging::Appx::{GetPackageFamilyName, PACKAGE_FAMILY_NAME_MAX_LENGTH},
        System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    };
    struct ProcessHandle(HANDLE);
    impl Drop for ProcessHandle { fn drop(&mut self) { unsafe { CloseHandle(self.0); } } }
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() { return false; }
        let process = ProcessHandle(handle);
        // Package families are bounded by Windows; no unbounded allocation or retry.
        let mut family = [0u16; PACKAGE_FAMILY_NAME_MAX_LENGTH as usize + 1];
        let mut length = family.len() as u32;
        if GetPackageFamilyName(process.0, &mut length, family.as_mut_ptr()) != ERROR_SUCCESS {
            return false;
        }
        let length = length as usize;
        if length == 0 || length > family.len() || family[length-1] != 0 { return false; }
        String::from_utf16(&family[..length-1]).is_ok_and(|value| value == "OpenAI.Codex_2p2nqsd0c76g0")
    }
}

#[cfg(windows)]
fn read_processes() -> std::io::Result<Vec<ProcessEntry>> {
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
            let desktop_candidate = name.eq_ignore_ascii_case("ChatGPT.exe");
            let same_session = (provider_for_name(&name).is_some() || desktop_candidate)
                && ProcessIdToSessionId(entry.th32ProcessID, &mut session) != 0 && session == own_session;
            // Ordinary ChatGPT and other packages must never count as Codex.
            let codex_desktop = same_session && desktop_candidate && is_packaged_codex_desktop(entry.th32ProcessID);
            processes.push(ProcessEntry { pid: entry.th32ProcessID, parent_pid: entry.th32ParentProcessID, name, same_session, codex_desktop });
            has_entry = Process32NextW(snapshot.0, &mut entry);
        }
        let error = GetLastError();
        if error != ERROR_NO_MORE_FILES { return Err(std::io::Error::from_raw_os_error(error as i32)); }
        Ok(processes)
    }
}

#[cfg(windows)]
pub fn read_running() -> std::io::Result<Vec<String>> {
    Ok(running_from_processes(&read_processes()?, std::process::id()))
}

#[cfg(not(windows))]
pub fn read_running() -> std::io::Result<Vec<String>> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Process detection is currently Windows-only."))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    #[ignore = "opt-in read-only Windows process observation; prints supported provider labels only"]
    fn windows_process_observation_smoke() {
        let running = read_running().expect("Windows process observation must complete");
        assert!(running.iter().all(|id| ["codex", "claude"].contains(&id.as_str())));
        let unique: HashSet<_> = running.iter().collect();
        assert_eq!(unique.len(), running.len(), "each supported provider appears at most once");
        println!("supported providers: {}", running.join(", "));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "opt-in Codex package identity check; prints supported provider labels only"]
    fn windows_packaged_desktop_observation_smoke() {
        let processes = read_processes().expect("Windows process observation must complete");
        let desktop: Vec<_> = processes.iter().filter(|process| process.codex_desktop).collect();
        assert!(desktop.iter().all(|process| process.same_session && process.name.eq_ignore_ascii_case("ChatGPT.exe")));
        // Absence is valid on a machine without an active packaged Codex desktop.
        println!("packaged desktop providers: {}", if desktop.is_empty() { "" } else { "codex" });
    }

    fn process(pid: u32, parent_pid: u32, name: &str) -> ProcessEntry {
        ProcessEntry { pid, parent_pid, name: name.into(), same_session: true, codex_desktop: false }
    }

    #[test]
    fn only_verified_packaged_chatgpt_host_counts_as_codex_desktop() {
        let ordinary = process(2, 1, "ChatGPT.exe");
        let mut wrong_name = process(3, 1, "codex-command-runner.exe"); wrong_name.codex_desktop = true;
        let mut browser = process(4, 1, "chrome.exe"); browser.codex_desktop = true;
        assert!(running_from_processes(&[ordinary, wrong_name, browser], 10).is_empty());
        let mut packaged = process(20, 1, "cHaTgPt.ExE"); packaged.codex_desktop = true;
        assert_eq!(running_from_processes(&[packaged], 10), ["codex"]);
        let mut other_session = process(21, 1, "ChatGPT.exe"); other_session.codex_desktop = true; other_session.same_session = false;
        let mut own_child = process(22, 10, "ChatGPT.exe"); own_child.codex_desktop = true;
        assert!(running_from_processes(&[process(10, 1, "ai-token.exe"), own_child, other_session], 10).is_empty());
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

use crate::app::BackgroundMessage;
use crate::models::ProcessInfo;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;
use taut::Waker;
use windows::core::{Owned, PWSTR};
use windows::Win32::Foundation::ERROR_NO_MORE_FILES;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};

const SCAN_INTERVAL: Duration = Duration::from_millis(500);
const PATH_BUFFER_SIZE: usize = 32_768;

#[derive(Default)]
struct Scanner {
    last: Option<Result<Vec<ProcessInfo>, String>>,
    path_cache: HashMap<u32, (String, PathBuf)>,
    path_buffer: Vec<u16>,
}

impl Scanner {
    fn update(
        &mut self,
        result: windows::core::Result<Vec<ProcessInfo>>,
    ) -> Option<BackgroundMessage> {
        let next = result.map_err(|error| format!("Process scan failed: {error}"));
        if self.last.as_ref() == Some(&next) {
            return None;
        }
        self.last = Some(next.clone());
        Some(match next {
            Ok(processes) => BackgroundMessage::Processes(processes),
            Err(error) => BackgroundMessage::ScanError(error),
        })
    }
}

pub(crate) fn scan_loop(tx: Sender<BackgroundMessage>, stop: Receiver<()>) {
    let mut scanner = Scanner {
        path_buffer: vec![0; PATH_BUFFER_SIZE],
        ..Default::default()
    };
    loop {
        let result = scan_processes(&mut scanner.path_cache, &mut scanner.path_buffer);
        if let Some(message) = scanner.update(result) {
            if tx.send(message).is_err() {
                break;
            }
            Waker.wake();
        }
        if stop.recv_timeout(SCAN_INTERVAL) != Err(RecvTimeoutError::Timeout) {
            break;
        }
    }
}

pub(crate) fn pids_named(name: &str) -> windows::core::Result<Vec<u32>> {
    let wanted = strip_exe(name);
    let mut pids = Vec::new();
    for_each_process_entry(|entry| {
        if strip_exe(&process_name(entry)).eq_ignore_ascii_case(wanted) {
            pids.push(entry.th32ProcessID);
        }
    })?;
    Ok(pids)
}

fn strip_exe(name: &str) -> &str {
    let bytes = name.as_bytes();
    let Some(stem) = bytes.len().checked_sub(4) else {
        return name;
    };
    if bytes[stem..].eq_ignore_ascii_case(b".exe") {
        &name[..stem]
    } else {
        name
    }
}

fn scan_processes(
    path_cache: &mut HashMap<u32, (String, PathBuf)>,
    path_buffer: &mut [u16],
) -> windows::core::Result<Vec<ProcessInfo>> {
    let mut processes = Vec::new();
    let mut live_pids = HashSet::new();
    for_each_process_entry(|entry| {
        let process = process_from_entry(entry, path_cache, path_buffer);
        live_pids.insert(process.pid);
        processes.push(process);
    })?;
    path_cache.retain(|pid, _| live_pids.contains(pid));
    processes.sort_unstable();
    Ok(processes)
}

fn for_each_process_entry(mut visit: impl FnMut(&PROCESSENTRY32W)) -> windows::core::Result<()> {
    let snapshot = unsafe { Owned::new(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?) };
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    unsafe { Process32FirstW(*snapshot, &mut entry)? };
    loop {
        visit(&entry);
        if let Err(error) = unsafe { Process32NextW(*snapshot, &mut entry) } {
            if error.code() == ERROR_NO_MORE_FILES.to_hresult() {
                return Ok(());
            }
            return Err(error);
        }
    }
}

fn process_from_entry(
    entry: &PROCESSENTRY32W,
    path_cache: &mut HashMap<u32, (String, PathBuf)>,
    path_buffer: &mut [u16],
) -> ProcessInfo {
    let pid = entry.th32ProcessID;
    let name = process_name(entry);
    let exe = match path_cache.get(&pid) {
        Some((cached_name, path)) if cached_name == &name => path.clone(),
        _ => {
            let path = query_process_path(pid, path_buffer);
            path_cache.insert(pid, (name.clone(), path.clone()));
            path
        }
    };
    ProcessInfo { name, pid, exe }
}

fn process_name(entry: &PROCESSENTRY32W) -> String {
    let name_length = entry
        .szExeFile
        .iter()
        .position(|&character| character == 0)
        .unwrap_or(entry.szExeFile.len());
    String::from_utf16_lossy(&entry.szExeFile[..name_length])
}

fn query_process_path(pid: u32, buffer: &mut [u16]) -> PathBuf {
    let Ok(handle) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }) else {
        return PathBuf::new();
    };
    let handle = unsafe { Owned::new(handle) };
    let mut length = buffer.len() as u32;
    if unsafe {
        QueryFullProcessImageNameW(
            *handle,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    }
    .is_err()
    {
        return PathBuf::new();
    }

    PathBuf::from(OsString::from_wide(&buffer[..length as usize]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Foundation::ERROR_ACCESS_DENIED;

    #[test]
    fn scan_recovery_publishes_unchanged_rows_once() {
        let mut scanner = Scanner::default();
        let rows = vec![ProcessInfo {
            name: "Game.exe".into(),
            pid: 1,
            exe: PathBuf::new(),
        }];
        let fail = || {
            Err(windows::core::Error::from_hresult(
                ERROR_ACCESS_DENIED.to_hresult(),
            ))
        };
        assert!(matches!(
            scanner.update(Ok(rows.clone())),
            Some(BackgroundMessage::Processes(_))
        ));
        assert!(scanner.update(Ok(rows.clone())).is_none());
        assert!(matches!(
            scanner.update(fail()),
            Some(BackgroundMessage::ScanError(_))
        ));
        assert!(scanner.update(fail()).is_none());
        assert!(matches!(
            scanner.update(Ok(rows.clone())),
            Some(BackgroundMessage::Processes(recovered)) if recovered == rows
        ));
        assert!(scanner.update(Ok(rows)).is_none());
    }
}

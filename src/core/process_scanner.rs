use crate::app::BackgroundMessage;
use crate::models::process::ProcessInfo;
use eframe::egui::Context;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};

const SCAN_INTERVAL: Duration = Duration::from_millis(500);
const PATH_BUFFER_SIZE: usize = 32_768;

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

pub fn scan_loop(tx: Sender<BackgroundMessage>, ctx: Context) {
    let mut last_processes = None;
    let mut path_cache = HashMap::new();
    let mut path_buffer = vec![0; PATH_BUFFER_SIZE];

    loop {
        if let Ok(processes) = scan_processes(&mut path_cache, &mut path_buffer) {
            if last_processes.as_ref() != Some(&processes) {
                last_processes = Some(processes.clone());
                if tx.send(BackgroundMessage::Processes(processes)).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        }
        std::thread::sleep(SCAN_INTERVAL);
    }
}

fn scan_processes(
    path_cache: &mut HashMap<u32, (String, PathBuf)>,
    path_buffer: &mut [u16],
) -> windows::core::Result<Vec<ProcessInfo>> {
    let snapshot = OwnedHandle(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)? });
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut processes = Vec::new();
    let mut live_pids = HashSet::new();

    unsafe { Process32FirstW(snapshot.0, &mut entry)? };
    loop {
        let pid = entry.th32ProcessID;
        let name_length = entry
            .szExeFile
            .iter()
            .position(|&character| character == 0)
            .unwrap_or(entry.szExeFile.len());
        let name = String::from_utf16_lossy(&entry.szExeFile[..name_length]);
        live_pids.insert(pid);

        let exe = match path_cache.get(&pid) {
            Some((cached_name, path)) if cached_name == &name => path.clone(),
            _ => {
                let path = query_process_path(pid, path_buffer);
                path_cache.insert(pid, (name.clone(), path.clone()));
                path
            }
        };
        processes.push(ProcessInfo { name, pid, exe });

        if unsafe { Process32NextW(snapshot.0, &mut entry) }.is_err() {
            break;
        }
    }

    path_cache.retain(|pid, _| live_pids.contains(pid));
    processes.sort_unstable_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.pid.cmp(&right.pid))
    });
    Ok(processes)
}

fn query_process_path(pid: u32, buffer: &mut [u16]) -> PathBuf {
    let Ok(handle) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }) else {
        return PathBuf::new();
    };
    let handle = OwnedHandle(handle);
    let mut length = buffer.len() as u32;
    if unsafe {
        QueryFullProcessImageNameW(
            handle.0,
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
    use super::{scan_processes, HashMap, PATH_BUFFER_SIZE};

    #[test]
    fn native_snapshot_contains_current_process() {
        let mut cache = HashMap::new();
        let mut buffer = vec![0; PATH_BUFFER_SIZE];
        let processes = scan_processes(&mut cache, &mut buffer).unwrap();

        assert!(processes
            .iter()
            .any(|process| process.pid == std::process::id()));
    }
}

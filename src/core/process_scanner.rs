use crate::app::BackgroundMessage;
use crate::models::ProcessInfo;
use eframe::egui::Context;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;
use windows::core::{Owned, PWSTR};
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
    last_processes: Option<Vec<ProcessInfo>>,
    last_error: Option<String>,
    path_cache: HashMap<u32, (String, PathBuf)>,
    path_buffer: Vec<u16>,
}

impl Scanner {
    fn poll(&mut self) -> Option<BackgroundMessage> {
        match scan_processes(&mut self.path_cache, &mut self.path_buffer) {
            Ok(processes) => self.update_processes(processes),
            Err(error) => self.update_error(format!("Process scan failed: {error}")),
        }
    }

    fn update_processes(&mut self, processes: Vec<ProcessInfo>) -> Option<BackgroundMessage> {
        self.last_error = None;
        if self.last_processes.as_ref() == Some(&processes) {
            return None;
        }
        self.last_processes = Some(processes.clone());
        Some(BackgroundMessage::Processes(processes))
    }

    fn update_error(&mut self, error: String) -> Option<BackgroundMessage> {
        if self.last_error.as_ref() == Some(&error) {
            return None;
        }
        self.last_error = Some(error.clone());
        Some(BackgroundMessage::Error(error))
    }
}

pub(crate) fn scan_loop(tx: Sender<BackgroundMessage>, ctx: Context) {
    let mut scanner = Scanner {
        path_buffer: vec![0; PATH_BUFFER_SIZE],
        ..Default::default()
    };
    while publish_update(&mut scanner, &tx, &ctx) {
        std::thread::sleep(SCAN_INTERVAL);
    }
}

fn publish_update(scanner: &mut Scanner, tx: &Sender<BackgroundMessage>, ctx: &Context) -> bool {
    let Some(message) = scanner.poll() else {
        return true;
    };
    if tx.send(message).is_err() {
        return false;
    }
    ctx.request_repaint();
    true
}

fn scan_processes(
    path_cache: &mut HashMap<u32, (String, PathBuf)>,
    path_buffer: &mut [u16],
) -> windows::core::Result<Vec<ProcessInfo>> {
    let snapshot = unsafe { Owned::new(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?) };
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut processes = Vec::new();
    let mut live_pids = HashSet::new();
    unsafe { Process32FirstW(*snapshot, &mut entry)? };
    loop {
        let process = process_from_entry(&entry, path_cache, path_buffer);
        live_pids.insert(process.pid);
        processes.push(process);
        if unsafe { Process32NextW(*snapshot, &mut entry) }.is_err() {
            break;
        }
    }
    path_cache.retain(|pid, _| live_pids.contains(pid));
    processes.sort_unstable();
    Ok(processes)
}

fn process_from_entry(
    entry: &PROCESSENTRY32W,
    path_cache: &mut HashMap<u32, (String, PathBuf)>,
    path_buffer: &mut [u16],
) -> ProcessInfo {
    let pid = entry.th32ProcessID;
    let name_length = entry
        .szExeFile
        .iter()
        .position(|&character| character == 0)
        .unwrap_or(entry.szExeFile.len());
    let name = String::from_utf16_lossy(&entry.szExeFile[..name_length]);
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

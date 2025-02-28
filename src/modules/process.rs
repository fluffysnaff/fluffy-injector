use std::path::PathBuf;
use sysinfo::System;

pub struct ProcessInfo {
    pub name: String,
    pub pid: u32,
    pub exe: PathBuf,
}

pub fn get_processes() -> Vec<ProcessInfo> {
    let mut sys = System::new_all();
    sys.refresh_all();
    sys.processes()
        .iter()
        .map(|(pid, process)| ProcessInfo {
            name: process.name().to_string_lossy().into_owned(),
            exe: process.exe().map(|p| p.to_path_buf()).unwrap_or_default(),
            pid: pid.as_u32(),
        })
        .collect()
}

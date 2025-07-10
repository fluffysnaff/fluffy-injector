use crate::app::BackgroundMessage;
use crate::models::process::ProcessInfo;
use eframe::egui::Context;
use std::sync::mpsc::Sender;
use std::time::Duration;
use sysinfo::System;

pub fn scan_loop(tx: Sender<BackgroundMessage>, ctx: Context) {
    let mut sys = System::new_all();
    loop {
        sys.refresh_all();
        
        let processes: Vec<ProcessInfo> = sys
            .processes()
            .iter()
            .map(|(pid, process)| ProcessInfo {
                name: process.name().to_string_lossy().into_owned(),
                pid: pid.as_u32(),
                exe: process.exe().map(|p| p.to_path_buf()).unwrap_or_default(),
            })
            .collect();

        if tx.send(BackgroundMessage::Processes(processes)).is_err() {
            break;
        }
        ctx.request_repaint();
        std::thread::sleep(Duration::from_secs(5));
    }
}
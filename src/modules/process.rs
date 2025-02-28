// Import System from the crate root, but get the extension trait from the module
use sysinfo::System;

pub fn get_processes() -> Vec<(String, u32)> {
    let mut sys = System::new_all();
    sys.refresh_all();
    sys.processes()
        .iter()
        .map(|(pid, process)| {
            // process.name() returns an OsStr, so convert it using to_string_lossy()
            (process.name().to_string_lossy().into_owned(), pid.as_u32())
        })
        .collect()
}

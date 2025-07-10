use std::path::PathBuf;

pub struct ProcessInfo {
    pub name: String,
    pub pid: u32,
    pub exe: PathBuf,
}
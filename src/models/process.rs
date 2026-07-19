use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessInfo {
    pub name: String,
    pub pid: u32,
    pub exe: PathBuf,
}

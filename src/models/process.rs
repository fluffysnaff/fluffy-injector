use std::path::PathBuf;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ProcessInfo {
    pub name: String,
    pub pid: u32,
    pub exe: PathBuf,
}

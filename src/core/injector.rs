use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use windows::{
    core::PCSTR,
    Win32::System::Diagnostics::Debug::WriteProcessMemory,
    Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress},
    Win32::System::Memory::{VirtualAllocEx, MEM_COMMIT, PAGE_READWRITE},
    Win32::System::Threading::{CreateRemoteThread, OpenProcess, PROCESS_ALL_ACCESS},
};

pub fn inject_dll(process_id: u32, dll_path: &str, copy_on_inject: bool) -> Result<()> {
    let copy = copy_on_inject
        .then(|| create_injection_copy(process_id, Path::new(dll_path)))
        .transpose()?;
    let path = copy
        .as_deref()
        .unwrap_or(Path::new(dll_path))
        .to_str()
        .context("DLL path is not valid UTF-8")?;
    inject_dll_path(process_id, path).inspect_err(|_| {
        if let Some(path) = &copy {
            let _ = std::fs::remove_file(path);
        }
    })
}

fn create_injection_copy(process_id: u32, source: &Path) -> Result<PathBuf> {
    if !source.is_file() {
        bail!("DLL does not exist: {}", source.display());
    }

    let dir = std::env::temp_dir().join("fluffy-injector");
    std::fs::create_dir_all(&dir).context("Failed to create temp DLL directory")?;
    // Best-effort: locked (still-loaded) copies fail to delete and are left alone.
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let _ = std::fs::remove_file(entry.path());
        }
    }

    let stem = source.file_stem().and_then(|s| s.to_str()).unwrap_or("dll");
    let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("dll");
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dest = dir.join(format!("{stem}-{process_id}-{id}.{ext}"));
    std::fs::copy(source, &dest).context("Failed to copy DLL for injection")?;
    Ok(dest)
}

fn inject_dll_path(process_id: u32, dll_path: &str) -> Result<()> {
    unsafe {
        let process_handle = OpenProcess(PROCESS_ALL_ACCESS, false, process_id)?;
        let dll_path_bytes = dll_path.as_bytes();
        let alloc_addr = VirtualAllocEx(
            process_handle,
            None,
            dll_path_bytes.len() + 1,
            MEM_COMMIT,
            PAGE_READWRITE,
        );
        if alloc_addr.is_null() {
            bail!("Failed to allocate memory in target process.");
        }
        let mut bytes_written = 0;
        WriteProcessMemory(
            process_handle,
            alloc_addr,
            dll_path_bytes.as_ptr() as _,
            dll_path_bytes.len(),
            Some(&mut bytes_written),
        )?;
        let kernel32_handle = GetModuleHandleA(PCSTR(b"kernel32.dll\0".as_ptr()))?;
        let load_library_addr = GetProcAddress(kernel32_handle, PCSTR(b"LoadLibraryA\0".as_ptr()));
        if load_library_addr.is_none() {
            bail!("Could not find LoadLibraryA address.");
        }
        let _thread_handle = CreateRemoteThread(
            process_handle,
            None,
            0,
            Some(std::mem::transmute(load_library_addr)),
            Some(alloc_addr),
            0,
            None,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::create_injection_copy;

    #[test]
    fn injection_copy_preserves_dll_contents() {
        let source = std::env::temp_dir().join(format!(
            "fluffy-injector-test-{}.dll",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::write(&source, b"test").unwrap();
        let copy = create_injection_copy(1, &source).unwrap();
        assert_eq!(std::fs::read(&copy).unwrap(), b"test");
        assert_ne!(copy, source);
        let _ = std::fs::remove_file(source);
        let _ = std::fs::remove_file(copy);
    }
}

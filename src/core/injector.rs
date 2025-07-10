use anyhow::{bail, Result};
use windows::{
    core::PCSTR,
    Win32::System::Diagnostics::Debug::WriteProcessMemory,
    Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress},
    Win32::System::Memory::{VirtualAllocEx, MEM_COMMIT, PAGE_READWRITE},
    Win32::System::Threading::{CreateRemoteThread, OpenProcess, PROCESS_ALL_ACCESS},
};

pub fn inject_dll(process_id: u32, dll_path: &str) -> Result<()> {
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
        let load_library_addr =
            GetProcAddress(kernel32_handle, PCSTR(b"LoadLibraryA\0".as_ptr()));
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
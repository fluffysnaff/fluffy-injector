use std::ptr;
use windows::{
    Win32::System::Diagnostics::Debug::WriteProcessMemory,
    Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress},
    Win32::System::Memory::{MEM_COMMIT, PAGE_READWRITE, VirtualAllocEx},
    Win32::System::Threading::{CreateRemoteThread, OpenProcess, PROCESS_ALL_ACCESS},
    core::PCSTR,
};

pub fn inject_dll(process_id: u32, dll_path: &str) -> bool {
    unsafe {
        // Open the target process.
        let process = OpenProcess(PROCESS_ALL_ACCESS, false, process_id);
        if process.is_err() {
            return false;
        }
        let process = process.unwrap();

        // Convert the DLL path to bytes and allocate memory in the target process.
        let dll_bytes = dll_path.as_bytes();
        let alloc = VirtualAllocEx(
            process,
            Some(ptr::null()),
            dll_bytes.len(),
            MEM_COMMIT,
            PAGE_READWRITE,
        );
        if alloc.is_null() {
            return false;
        }

        // Write the DLL path into the allocated memory.
        let _ = WriteProcessMemory(
            process,
            alloc,
            dll_bytes.as_ptr() as _,
            dll_bytes.len(),
            None,
        );

        // Get the address of LoadLibraryA from kernel32.dll.
        let kernel32 = GetModuleHandleA(PCSTR(b"kernel32.dll\0".as_ptr()));
        if kernel32.is_err() {
            return false;
        }
        let kernel32 = kernel32.unwrap();

        let load_library_addr = GetProcAddress(kernel32, PCSTR(b"LoadLibraryA\0".as_ptr()));
        if load_library_addr.is_none() {
            return false;
        }
        let load_library_addr = load_library_addr.unwrap();

        // Create a remote thread in the target process that calls LoadLibraryA with our DLL path.
        let _ = CreateRemoteThread(
            process,
            None,
            0,
            Some(std::mem::transmute(load_library_addr)),
            Some(alloc),
            0,
            None,
        );

        true
    }
}

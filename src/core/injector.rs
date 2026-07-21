use anyhow::{bail, Context, Result};
use std::ffi::c_void;
use std::io::ErrorKind;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows::core::{s, Owned};
use windows::Win32::Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
use windows::Win32::System::Threading::{
    CreateRemoteThread, GetExitCodeThread, WaitForSingleObject, LPTHREAD_START_ROUTINE,
    PROCESS_CREATE_THREAD, PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_READ,
    PROCESS_VM_WRITE,
};
use wraith::manipulation::remote::{
    find_remote_module, ProcessAccess, RemoteAllocation, RemoteProcess,
};

const INJECTION_TIMEOUT_MS: u32 = 30_000;
const INJECTION_ACCESS: u32 = PROCESS_CREATE_THREAD.0
    | PROCESS_QUERY_INFORMATION.0
    | PROCESS_VM_OPERATION.0
    | PROCESS_VM_READ.0
    | PROCESS_VM_WRITE.0;

pub(crate) fn inject_dll(
    process_id: u32,
    dll_path: &str,
    copy_on_inject: bool,
    randomize_name: bool,
) -> Result<()> {
    let copy = copy_on_inject
        .then(|| create_injection_copy(process_id, Path::new(dll_path), randomize_name))
        .transpose()?;
    let path = copy.as_deref().unwrap_or(Path::new(dll_path));
    inject_dll_path(process_id, path)
}

fn create_injection_copy(process_id: u32, source: &Path, randomize_name: bool) -> Result<PathBuf> {
    if !source.is_file() {
        bail!("DLL does not exist: {}", source.display());
    }

    let dir = std::env::temp_dir().join("fluffy-injector");
    std::fs::create_dir_all(&dir).context("Failed to create temp DLL directory")?;
    clean_copy_directory(&dir)?;

    let stem = source.file_stem().and_then(|s| s.to_str()).unwrap_or("dll");
    let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("dll");
    let file_name = if randomize_name {
        format!("{}.{}", random_file_stem(), ext)
    } else {
        let id = std::random::random::<u64>(..);
        format!("{stem}-{process_id}-{id}.{ext}")
    };
    let dest = dir.join(file_name);
    std::fs::copy(source, &dest).context("Failed to copy DLL for injection")?;
    Ok(dest)
}

fn clean_copy_directory(directory: &Path) -> Result<()> {
    for entry in std::fs::read_dir(directory).context("Failed to inspect temp DLL directory")? {
        let path = entry.context("Failed to inspect temp DLL entry")?.path();
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::NotFound | ErrorKind::PermissionDenied
                ) => {}
            Err(error) => return Err(error).context("Failed to clean temp DLL directory"),
        }
    }
    Ok(())
}

fn random_file_stem() -> String {
    format!("{:032x}", std::random::random::<u128>(..))
}

fn inject_dll_path(process_id: u32, dll_path: &Path) -> Result<()> {
    let process = RemoteProcess::open(process_id, ProcessAccess::custom(INJECTION_ACCESS))
        .context("Failed to open target process")?;
    let path_memory = write_dll_path(&process, dll_path)?;
    let load_library = remote_load_library_address(&process)?;
    let thread = create_load_library_thread(&process, load_library, path_memory.base())?;
    wait_for_load(*thread, path_memory)?;
    ensure_load_succeeded(*thread)
}

fn write_dll_path(process: &RemoteProcess, dll_path: &Path) -> Result<RemoteAllocation> {
    let path_bytes: Vec<u8> = dll_path
        .as_os_str()
        .encode_wide()
        .chain([0])
        .flat_map(u16::to_ne_bytes)
        .collect();
    let path_memory = process
        .allocate_rw(path_bytes.len())
        .context("Failed to allocate the DLL path in the target process")?;
    let written = process
        .write(path_memory.base(), &path_bytes)
        .context("Failed to write the DLL path to the target process")?;
    if written != path_bytes.len() {
        bail!(
            "Only {written}/{} DLL path bytes were written",
            path_bytes.len()
        );
    }
    Ok(path_memory)
}

fn create_load_library_thread(
    process: &RemoteProcess,
    load_library: usize,
    path_address: usize,
) -> Result<Owned<HANDLE>> {
    let start_routine =
        unsafe { std::mem::transmute::<usize, LPTHREAD_START_ROUTINE>(load_library) };
    let thread = unsafe {
        Owned::new(CreateRemoteThread(
            HANDLE(process.handle() as *mut c_void),
            None,
            0,
            start_routine,
            Some(path_address as *mut c_void),
            0,
            None,
        )?)
    };
    Ok(thread)
}

fn wait_for_load(thread: HANDLE, path_memory: RemoteAllocation) -> Result<()> {
    match unsafe { WaitForSingleObject(thread, INJECTION_TIMEOUT_MS) } {
        WAIT_OBJECT_0 => Ok(()),
        WAIT_TIMEOUT => {
            path_memory.leak();
            bail!("Timed out waiting for LoadLibraryW")
        }
        status => {
            path_memory.leak();
            bail!("Waiting for LoadLibraryW failed with status {}", status.0)
        }
    }
}

fn ensure_load_succeeded(thread: HANDLE) -> Result<()> {
    let mut result = 0;
    unsafe { GetExitCodeThread(thread, &mut result) }
        .context("Failed to read LoadLibraryW result")?;
    if result == 0 {
        bail!("LoadLibraryW rejected the DLL");
    }
    Ok(())
}

fn remote_load_library_address(process: &RemoteProcess) -> Result<usize> {
    let local_module = unsafe { GetModuleHandleA(s!("kernel32.dll")) }
        .context("Failed to locate local kernel32.dll")?;
    let local_export = unsafe { GetProcAddress(local_module, s!("LoadLibraryW")) }
        .context("Failed to locate local LoadLibraryW")? as usize;
    let local_base = local_module.0 as usize;
    let offset = local_export
        .checked_sub(local_base)
        .context("LoadLibraryW resolved outside kernel32.dll")?;
    let remote_module = find_remote_module(process, "kernel32.dll")
        .context("Failed to locate remote kernel32.dll")?;
    if offset >= remote_module.size() {
        bail!("LoadLibraryW resolved outside kernel32.dll");
    }
    remote_module
        .base()
        .checked_add(offset)
        .context("Remote LoadLibraryW address overflowed")
}

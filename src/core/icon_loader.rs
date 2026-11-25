use std::ffi::{c_void, OsStr};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use taut::Waker;
use windows::core::{Owned, PCWSTR};
use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
use windows::Win32::Graphics::Gdi::{
    GetDIBits, GetObjectW, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HBITMAP,
};
use windows::Win32::UI::Shell::ExtractIconExW;
use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, HICON, ICONINFO};

pub(crate) enum IconRequest {
    Dll,
    Process(PathBuf),
}

pub(crate) type IconData = Option<(usize, Vec<u32>)>;

pub(crate) struct Worker {
    pub requests: Sender<IconRequest>,
    pub results: Receiver<(IconRequest, IconData)>,
}

impl Worker {
    pub(crate) fn spawn() -> Self {
        let (requests, rx) = mpsc::channel();
        let (tx, results) = mpsc::channel();
        thread::spawn(move || load_loop(rx, tx));
        Self { requests, results }
    }
}

fn load_loop(rx: Receiver<IconRequest>, tx: Sender<(IconRequest, IconData)>) {
    while let Ok(first) = rx.recv() {
        // A startup batch needs one native wake, not one per picture.
        for icon in std::iter::once(first).chain(rx.try_iter()).take(8) {
            let data = match &icon {
                IconRequest::Dll => taut::png::decode(include_bytes!("../../assets/dll_icon.png")),
                IconRequest::Process(path) => load_exe_icon_data(path),
            };
            if tx.send((icon, data)).is_err() {
                return;
            }
        }
        Waker.wake();
    }
}

fn load_exe_icon_data(exe_path: &Path) -> Option<(usize, Vec<u32>)> {
    let wide_path: Vec<u16> = OsStr::new(exe_path).encode_wide().chain(Some(0)).collect();
    let mut h_icon: HICON = HICON::default();
    let count =
        unsafe { ExtractIconExW(PCWSTR(wide_path.as_ptr()), 0, None, Some(&mut h_icon), 1) };
    if count == 0 || h_icon.is_invalid() {
        return None;
    }
    let hicon = unsafe { Owned::new(h_icon) };
    let mut icon_info = ICONINFO::default();
    unsafe { GetIconInfo(*hicon, &mut icon_info) }.ok()?;
    let color_bitmap = unsafe { Owned::new(icon_info.hbmColor) };
    drop(unsafe { Owned::new(icon_info.hbmMask) });
    let size = bitmap_size(*color_bitmap)?;
    Some((size[0], read_bitmap(*color_bitmap, size)?))
}

fn bitmap_size(color_bitmap: HBITMAP) -> Option<[usize; 2]> {
    let mut bmp: BITMAP = BITMAP::default();
    let ret = unsafe {
        GetObjectW(
            color_bitmap.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some((&mut bmp as *mut BITMAP).cast::<c_void>()),
        )
    };

    if ret == 0 {
        return None;
    }
    let width = usize::try_from(bmp.bmWidth).ok()?;
    let height = usize::try_from(bmp.bmHeight.unsigned_abs()).ok()?;
    Some([width, height])
}

fn read_bitmap(color_bitmap: HBITMAP, [width, height]: [usize; 2]) -> Option<Vec<u32>> {
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: i32::try_from(width).ok()?,
            biHeight: -i32::try_from(height).ok()?,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0,
            ..Default::default()
        },
        ..Default::default()
    };
    // On little-endian Windows, the DIB's BGRA bytes form 0xAARRGGBB words.
    let mut buffer = vec![0u32; width.checked_mul(height)?];
    let rows = u32::try_from(height).ok()?;
    let hdc = unsafe { GetDC(None) };
    if hdc.is_invalid() {
        return None;
    }
    let success = unsafe {
        GetDIBits(
            hdc,
            color_bitmap,
            0,
            rows,
            Some(buffer.as_mut_ptr().cast::<c_void>()),
            &mut info,
            DIB_RGB_COLORS,
        ) != 0
    };
    let released = unsafe { ReleaseDC(None, hdc) } != 0;
    (success && released).then_some(buffer)
}

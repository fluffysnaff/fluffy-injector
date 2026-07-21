use crate::app::BackgroundMessage;
use eframe::egui::{ColorImage, Context};
use std::ffi::{c_void, OsStr};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender};
use windows::core::{Owned, PCWSTR};
use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
use windows::Win32::Graphics::Gdi::{
    GetDIBits, GetObjectW, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS,
};
use windows::Win32::UI::Shell::ExtractIconExW;
use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, HICON, ICONINFO};

pub(crate) fn load_loop(
    rx: Receiver<(u32, std::path::PathBuf)>,
    tx: Sender<BackgroundMessage>,
    ctx: Context,
) {
    while let Ok((pid, path)) = rx.recv() {
        if let Some(color_image) = load_exe_icon_data(&path) {
            if tx.send(BackgroundMessage::Icon((pid, color_image))).is_ok() {
                ctx.request_repaint();
            } else {
                break;
            }
        }
    }
}

fn load_exe_icon_data(exe_path: &Path) -> Option<ColorImage> {
    let wide_path: Vec<u16> = OsStr::new(exe_path).encode_wide().chain(Some(0)).collect();
    let mut h_icon: HICON = HICON::default();
    let count =
        unsafe { ExtractIconExW(PCWSTR(wide_path.as_ptr()), 0, None, Some(&mut h_icon), 1) };
    if count == 0 || h_icon.is_invalid() {
        return None;
    }
    hicon_to_color_image(*unsafe { Owned::new(h_icon) })
}

fn hicon_to_color_image(hicon: HICON) -> Option<ColorImage> {
    let mut icon_info = ICONINFO::default();
    if unsafe { GetIconInfo(hicon, &mut icon_info) }.is_err() {
        return None;
    }
    let color_bitmap = unsafe { Owned::new(icon_info.hbmColor) };
    let mask_bitmap = unsafe { Owned::new(icon_info.hbmMask) };

    let mut bmp: BITMAP = BITMAP::default();
    let ret = unsafe {
        GetObjectW(
            (*color_bitmap).into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some((&mut bmp as *mut BITMAP).cast::<c_void>()),
        )
    };

    if ret == 0 {
        return None;
    }

    let width = bmp.bmWidth as usize;
    let height = bmp.bmHeight.unsigned_abs() as usize;

    let mut bi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut buffer = vec![0u8; width * height * 4];
    let hdc = unsafe { GetDC(None) };

    let success = unsafe {
        GetDIBits(
            hdc,
            *color_bitmap,
            0,
            height as u32,
            Some(buffer.as_mut_ptr().cast::<c_void>()),
            &mut bi,
            DIB_RGB_COLORS,
        ) != 0
    };

    if unsafe { ReleaseDC(None, hdc) } == 0 {
        return None;
    }
    drop(mask_bitmap);

    if success {
        for chunk in buffer.as_chunks_mut::<4>().0 {
            chunk.swap(0, 2);
        }
        Some(ColorImage::from_rgba_unmultiplied([width, height], &buffer))
    } else {
        None
    }
}

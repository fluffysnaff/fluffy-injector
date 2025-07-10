use crate::app::BackgroundMessage;
use eframe::egui::{ColorImage, Context};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender};
use windows::core::{PCWSTR, Result};
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS,
};
use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
use windows::Win32::UI::Shell::ExtractIconExW;
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

pub fn load_loop(
    rx: Receiver<(u32, std::path::PathBuf)>,
    tx: Sender<BackgroundMessage>,
    ctx: Context,
) {
    while let Ok((pid, path)) = rx.recv() {
        if let Ok(Some(color_image)) = load_exe_icon_data(&path) {
            if tx.send(BackgroundMessage::Icon((pid, color_image))).is_ok() {
                ctx.request_repaint();
            } else {
                break;
            }
        }
    }
}

fn load_exe_icon_data(exe_path: &Path) -> Result<Option<ColorImage>> {
    let wide_path: Vec<u16> = OsStr::new(exe_path).encode_wide().chain(Some(0)).collect();
    let mut h_icon: HICON = HICON::default();
    let count =
        unsafe { ExtractIconExW(PCWSTR(wide_path.as_ptr()), 0, None, Some(&mut h_icon), 1) };
    if count > 0 && !h_icon.is_invalid() {
        let result = hicon_to_color_image(h_icon);
        unsafe { DestroyIcon(h_icon)? };
        return Ok(result);
    }
    Ok(None)
}

fn hicon_to_color_image(hicon: HICON) -> Option<ColorImage> {
    let mut icon_info = ICONINFO::default();
    if unsafe { GetIconInfo(hicon, &mut icon_info) }.is_err() {
        return None;
    }

    let mut bmp: BITMAP = BITMAP::default();
    let ret = unsafe {
        GetObjectW(
            icon_info.hbmColor.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bmp as *mut _ as *mut _),
        )
    };

    if ret == 0 {
        unsafe {
            let _ = DeleteObject(icon_info.hbmColor.into());
            let _ = DeleteObject(icon_info.hbmMask.into());
        }
        return None;
    }

    let width = bmp.bmWidth as usize;
    let height = bmp.bmHeight.abs() as usize;

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
            icon_info.hbmColor,
            0,
            height as u32,
            Some(buffer.as_mut_ptr() as *mut _),
            &mut bi,
            DIB_RGB_COLORS,
        ) != 0
    };

    unsafe {
        let _ = ReleaseDC(None, hdc);
        let _ = DeleteObject(icon_info.hbmColor.into());
        let _ = DeleteObject(icon_info.hbmMask.into());
    }

    if success {
        for chunk in buffer.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }
        Some(ColorImage::from_rgba_unmultiplied([width, height], &buffer))
    } else {
        None
    }
}
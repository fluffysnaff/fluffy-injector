use crate::app::BackgroundMessage;
use eframe::egui::{ColorImage, Context};
use std::ffi::{c_void, OsStr};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender};
use windows::core::{Owned, PCWSTR};
use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
use windows::Win32::Graphics::Gdi::{
    GetDIBits, GetObjectW, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HBITMAP,
};
use windows::Win32::UI::Shell::ExtractIconExW;
use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, HICON, ICONINFO};

pub(crate) fn load_loop(
    rx: Receiver<(u32, std::path::PathBuf)>,
    tx: Sender<BackgroundMessage>,
    ctx: Context,
) {
    while let Ok((pid, path)) = rx.recv() {
        let Some(color_image) = load_exe_icon_data(&path) else {
            continue;
        };
        if tx
            .send(BackgroundMessage::Icon((pid, color_image)))
            .is_err()
        {
            return;
        }
        ctx.request_repaint();
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
    let color_bitmap = icon_color_bitmap(hicon)?;
    let size = bitmap_size(*color_bitmap)?;
    let mut buffer = read_bitmap(*color_bitmap, size)?;
    for pixel in buffer.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    Some(ColorImage::from_rgba_unmultiplied(size, &buffer))
}

fn icon_color_bitmap(hicon: HICON) -> Option<Owned<HBITMAP>> {
    let mut icon_info = ICONINFO::default();
    if unsafe { GetIconInfo(hicon, &mut icon_info) }.is_err() {
        return None;
    }
    let color_bitmap = unsafe { Owned::new(icon_info.hbmColor) };
    let mask_bitmap = unsafe { Owned::new(icon_info.hbmMask) };
    drop(mask_bitmap);
    Some(color_bitmap)
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

fn bitmap_info([width, height]: [usize; 2]) -> Option<BITMAPINFO> {
    let width = i32::try_from(width).ok()?;
    let height = i32::try_from(height).ok()?.checked_neg()?;
    Some(BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0,
            ..Default::default()
        },
        ..Default::default()
    })
}

fn read_bitmap(color_bitmap: HBITMAP, size: [usize; 2]) -> Option<Vec<u8>> {
    let [width, height] = size;
    let byte_count = width.checked_mul(height)?.checked_mul(4)?;
    let mut buffer = vec![0; byte_count];
    let mut info = bitmap_info(size)?;
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

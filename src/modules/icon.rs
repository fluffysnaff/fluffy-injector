use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, GetDIBits, GetObjectW,
};
use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
use windows::Win32::UI::Shell::ExtractIconExW;
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};
use windows::core::PCWSTR;

pub fn load_exe_icon(ctx: &Context, exe_path: &Path) -> Option<TextureHandle> {
    // Convert exe path to a wide string.
    let wide: Vec<u16> = OsStr::new(exe_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // Initialize hicon_small using a null pointer.
    let mut hicon_small: HICON = HICON(std::ptr::null_mut());
    let count =
        unsafe { ExtractIconExW(PCWSTR(wide.as_ptr()), 0, None, Some(&mut hicon_small), 1) };
    if count == 0 || hicon_small.0.is_null() {
        return None;
    }

    if let Some((rgba, width, height)) = hicon_to_rgba(hicon_small) {
        let color_image =
            ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &rgba);
        let tex = ctx.load_texture(
            exe_path.to_string_lossy(),
            color_image,
            TextureOptions::default(),
        );
        unsafe { DestroyIcon(hicon_small) };
        return Some(tex);
    }
    unsafe { DestroyIcon(hicon_small) };
    None
}

fn hicon_to_rgba(hicon: HICON) -> Option<(Vec<u8>, u32, u32)> {
    // Retrieve icon information.
    let mut iconinfo = ICONINFO::default();
    let ok = unsafe { GetIconInfo(hicon, &mut iconinfo) }.is_ok();
    if !ok {
        return None;
    }
    let mut bmp: BITMAP = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<BITMAP>() as i32;
    let ret = unsafe {
        // Convert hbmColor into the expected HGDIOBJ.
        GetObjectW(
            iconinfo.hbmColor.into(),
            size,
            Some(&mut bmp as *mut _ as *mut _),
        )
    };
    if ret == 0 {
        return None;
    }
    let width = bmp.bmWidth as u32;
    let height = bmp.bmHeight.abs() as u32;
    let mut buffer = vec![0u8; (width * height * 4) as usize];
    let mut bi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32), // Negative for top-down DIB.
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0, // Use 0 instead of casting BI_RGB.
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [Default::default()],
    };
    // Wrap HWND in Some() so that GetDC receives Option<HWND>.
    let hdc = unsafe { GetDC(Some(HWND(std::ptr::null_mut()))) };
    let ret = unsafe {
        GetDIBits(
            hdc,
            iconinfo.hbmColor,
            0,
            height,
            Some(buffer.as_mut_ptr() as *mut _),
            &mut bi,
            DIB_RGB_COLORS,
        )
    };
    // Similarly wrap HWND in Some() for ReleaseDC.
    unsafe { ReleaseDC(Some(HWND(std::ptr::null_mut())), hdc) };
    if ret == 0 {
        return None;
    }
    for chunk in buffer.chunks_exact_mut(4) {
        chunk.swap(0, 2);
    }
    Some((buffer, width, height))
}

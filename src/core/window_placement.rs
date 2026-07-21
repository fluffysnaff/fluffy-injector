use crate::models::config::WindowPlacement;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::c_void;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowPlacement, SetWindowPlacement, SW_SHOWMAXIMIZED, SW_SHOWNORMAL, WINDOWPLACEMENT,
};

pub(crate) fn handle(window: &impl HasWindowHandle) -> Option<HWND> {
    let RawWindowHandle::Win32(handle) = window.window_handle().ok()?.as_raw() else {
        return None;
    };
    Some(HWND(handle.hwnd.get() as *mut c_void))
}

pub(crate) fn capture(hwnd: HWND) -> windows::core::Result<WindowPlacement> {
    let placement = native_placement(hwnd)?;
    let rect = placement.rcNormalPosition;
    Ok(WindowPlacement {
        position: [rect.left, rect.top],
        maximized: placement.showCmd == SW_SHOWMAXIMIZED.0 as u32,
    })
}

pub(crate) fn restore(hwnd: HWND, placement: WindowPlacement) -> windows::core::Result<()> {
    let mut native = native_placement(hwnd)?;
    let [left, top] = placement.position;
    let width = native.rcNormalPosition.right - native.rcNormalPosition.left;
    let height = native.rcNormalPosition.bottom - native.rcNormalPosition.top;
    native.rcNormalPosition = RECT {
        left,
        top,
        right: left + width,
        bottom: top + height,
    };
    native.showCmd = if placement.maximized {
        SW_SHOWMAXIMIZED.0 as u32
    } else {
        SW_SHOWNORMAL.0 as u32
    };
    unsafe { SetWindowPlacement(hwnd, &native) }
}

fn native_placement(hwnd: HWND) -> windows::core::Result<WINDOWPLACEMENT> {
    let mut placement = WINDOWPLACEMENT {
        length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
        ..Default::default()
    };
    unsafe { GetWindowPlacement(hwnd, &mut placement)? };
    Ok(placement)
}

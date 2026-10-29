//! Clip the Windows HWND itself: CSS rounding cannot remove an opaque WebView2 host.
//! Keep WebView2 opaque and avoid mixing alpha compositing with layered color keys.

use std::io;
use windows_sys::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::{
        ClientToScreen, CreateEllipticRgn, CreateRectRgn, DeleteObject, EqualRgn, GetWindowRgn,
        SetWindowRgn,
    },
    UI::WindowsAndMessaging::{GetClientRect, GetWindowRect},
};

pub fn apply_circle(window: &tauri::WebviewWindow) -> Result<(), String> {
    let hwnd = window.hwnd().map_err(|error| error.to_string())?;
    // The Tauri window owns this HWND for the duration of the call.
    unsafe { clip_to_client_circle(hwnd.0 as HWND) }.map_err(|error| error.to_string())
}

unsafe fn clip_to_client_circle(hwnd: HWND) -> io::Result<()> {
    let mut client: RECT = std::mem::zeroed();
    let mut frame: RECT = std::mem::zeroed();
    let mut origin = POINT { x: 0, y: 0 };
    if GetClientRect(hwnd, &mut client) == 0
        || GetWindowRect(hwnd, &mut frame) == 0
        || ClientToScreen(hwnd, &mut origin) == 0
    {
        return Err(io::Error::last_os_error());
    }

    // Query physical pixels after every resize/DPI change; never assume 1 CSS px = 1 px.
    let width = client.right - client.left;
    let height = client.bottom - client.top;
    if width <= 0 || height <= 0 {
        return Ok(());
    }
    let diameter = width.min(height);
    // SetWindowRgn is relative to the outer window, not its client area.
    let left = origin.x - frame.left + (width - diameter) / 2;
    let top = origin.y - frame.top + (height - diameter) / 2;
    let region = CreateEllipticRgn(left, top, left + diameter, top + diameter);
    if region.is_null() {
        return Err(io::Error::last_os_error());
    }
    // SetWindowRgn sends window-position messages. Avoid reapplying an unchanged
    // region if those messages cause another resize notification.
    let previous = CreateRectRgn(0, 0, 0, 0);
    if previous.is_null() {
        let error = io::Error::last_os_error();
        DeleteObject(region);
        return Err(error);
    }
    let unchanged = GetWindowRgn(hwnd, previous) > 0 && EqualRgn(previous, region) != 0;
    DeleteObject(previous);
    if unchanged {
        DeleteObject(region);
        return Ok(());
    }
    if SetWindowRgn(hwnd, region, 1) == 0 {
        let error = io::Error::last_os_error();
        DeleteObject(region);
        return Err(error);
    }
    // Windows owns the region after a successful SetWindowRgn; do not delete it.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::{
        Graphics::Gdi::PtInRegion,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, SetWindowPos, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER,
            WS_POPUP,
        },
    };

    #[test]
    fn native_region_excludes_corners_after_resize() {
        unsafe {
            let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                std::ptr::null(),
                WS_POPUP,
                0,
                0,
                76,
                76,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            );
            assert!(!hwnd.is_null());
            // Physical sizes at 100%, 125%, 150% and 200%, then back to 100%.
            for diameter in [76, 95, 114, 152, 76] {
                assert_ne!(
                    SetWindowPos(
                        hwnd,
                        std::ptr::null_mut(),
                        0,
                        0,
                        diameter,
                        diameter,
                        SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOZORDER
                    ),
                    0
                );
                clip_to_client_circle(hwnd).unwrap();
                clip_to_client_circle(hwnd).unwrap();
                let region = CreateRectRgn(0, 0, 0, 0);
                assert!(!region.is_null());
                assert!(GetWindowRgn(hwnd, region) > 0);
                assert_ne!(PtInRegion(region, diameter / 2, diameter / 2), 0);
                assert_ne!(PtInRegion(region, diameter / 2, 2), 0);
                for (x, y) in [
                    (0, 0),
                    (diameter - 1, 0),
                    (0, diameter - 1),
                    (diameter - 1, diameter - 1),
                ] {
                    assert_eq!(PtInRegion(region, x, y), 0);
                }
                DeleteObject(region);
            }
            DestroyWindow(hwnd);
        }
    }
}

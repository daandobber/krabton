//! Native top-level windows that plugin editors embed into (Windows only).
//! The UI thread's event loop already pumps messages for every window on
//! that thread, so these windows need no loop of their own.

use std::sync::{Mutex, OnceLock};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

static CLASS: OnceLock<Vec<u16>> = OnceLock::new();
static CLOSED: Mutex<Vec<isize>> = Mutex::new(Vec::new());
static RESIZED: Mutex<Vec<(isize, u32, u32)>> = Mutex::new(Vec::new());

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn h(hwnd: isize) -> HWND {
    hwnd as HWND
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CLOSE => {
            // The host tears the editor down on its next idle() call.
            if let Ok(mut c) = CLOSED.lock() {
                c.push(hwnd as isize);
            }
            ShowWindow(hwnd, SW_HIDE);
            0
        }
        WM_SIZE => {
            let w = (lparam as usize & 0xffff) as u32;
            let hgt = ((lparam as usize >> 16) & 0xffff) as u32;
            if w > 0 && hgt > 0 {
                if let Ok(mut r) = RESIZED.lock() {
                    r.retain(|x| x.0 != hwnd as isize);
                    r.push((hwnd as isize, w, hgt));
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn class_name() -> &'static [u16] {
    CLASS.get_or_init(|| {
        let name = wide("KrabtonPluginEditor");
        // SAFETY: registering a window class with valid, 'static data.
        unsafe {
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: GetModuleHandleW(std::ptr::null()),
                hIcon: std::ptr::null_mut(),
                hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
                hbrBackground: std::ptr::null_mut(),
                lpszMenuName: std::ptr::null(),
                lpszClassName: name.as_ptr(),
            };
            RegisterClassW(&wc);
        }
        name
    })
}

fn style(resizable: bool) -> u32 {
    let base = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_CLIPCHILDREN;
    if resizable {
        base | WS_THICKFRAME | WS_MAXIMIZEBOX
    } else {
        base
    }
}

/// Creates a hidden window with a client area of `w` x `h` pixels.
pub fn create_window(title: &str, w: u32, hgt: u32, resizable: bool) -> Result<isize, String> {
    let class = class_name();
    let title = wide(title);
    let st = style(resizable);
    let mut rect = RECT { left: 0, top: 0, right: w as i32, bottom: hgt as i32 };
    // SAFETY: plain Win32 calls with valid arguments.
    unsafe {
        AdjustWindowRectEx(&mut rect, st, 0, 0);
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            title.as_ptr(),
            st,
            80,
            80,
            rect.right - rect.left,
            rect.bottom - rect.top,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            GetModuleHandleW(std::ptr::null()),
            std::ptr::null(),
        );
        if hwnd.is_null() {
            return Err("could not create the editor window".into());
        }
        Ok(hwnd as isize)
    }
}

/// Resizes the window so its client area is `w` x `h` pixels.
pub fn resize_window(hwnd: isize, w: u32, hgt: u32) {
    // SAFETY: valid window handle owned by us.
    unsafe {
        let st = GetWindowLongW(h(hwnd), GWL_STYLE) as u32;
        let mut rect = RECT { left: 0, top: 0, right: w as i32, bottom: hgt as i32 };
        AdjustWindowRectEx(&mut rect, st, 0, 0);
        SetWindowPos(h(hwnd), std::ptr::null_mut(), 0, 0, rect.right - rect.left, rect.bottom - rect.top, SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

pub fn show_window(hwnd: isize) {
    // SAFETY: valid window handle.
    unsafe {
        ShowWindow(h(hwnd), SW_SHOW);
        SetForegroundWindow(h(hwnd));
    }
}

pub fn focus_window(hwnd: isize) {
    // SAFETY: valid window handle.
    unsafe {
        ShowWindow(h(hwnd), SW_RESTORE);
        SetForegroundWindow(h(hwnd));
    }
}

pub fn destroy_window(hwnd: isize) {
    // SAFETY: valid window handle owned by us.
    unsafe {
        DestroyWindow(h(hwnd));
    }
    if let Ok(mut c) = CLOSED.lock() {
        c.retain(|x| *x != hwnd);
    }
    if let Ok(mut r) = RESIZED.lock() {
        r.retain(|x| x.0 != hwnd);
    }
}

pub fn dpi_scale(hwnd: isize) -> f64 {
    // SAFETY: valid window handle.
    let dpi = unsafe { GetDpiForWindow(h(hwnd)) };
    if dpi == 0 {
        1.0
    } else {
        dpi as f64 / 96.0
    }
}

/// True (once) when the user closed this window.
pub fn take_closed(hwnd: isize) -> bool {
    let Ok(mut c) = CLOSED.lock() else { return false };
    let before = c.len();
    c.retain(|x| *x != hwnd);
    c.len() != before
}

/// The latest client size after the user resized the window.
pub fn take_resized(hwnd: isize) -> Option<(u32, u32)> {
    let mut r = RESIZED.lock().ok()?;
    let pos = r.iter().position(|x| x.0 == hwnd)?;
    let (_, w, hgt) = r.remove(pos);
    Some((w, hgt))
}

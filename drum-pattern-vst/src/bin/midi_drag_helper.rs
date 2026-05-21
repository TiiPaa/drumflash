#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "windows")]
#[path = "../native_drag.rs"]
mod native_drag;

#[cfg(target_os = "windows")]
use std::{
    ffi::c_void,
    mem::zeroed,
    os::windows::ffi::OsStrExt,
    path::PathBuf,
    ptr::{null, null_mut},
};

#[cfg(target_os = "windows")]
use windows_sys::Win32::{
    Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::{
        BeginPaint, CreateFontW, CreateRoundRectRgn, CreateSolidBrush, DeleteObject, DrawTextW,
        EndPaint, FillRect, FrameRect, SelectObject, SetBkMode, SetTextColor, SetWindowRgn,
        DT_CENTER, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, PAINTSTRUCT, TRANSPARENT,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos,
        GetMessageW, GetWindowLongPtrW, KillTimer, LoadCursorW, PostQuitMessage, RegisterClassW,
        SetTimer, SetWindowLongPtrW, ShowWindow, TranslateMessage, CREATESTRUCTW, CS_HREDRAW,
        CS_VREDRAW, GWLP_USERDATA, IDC_HAND, MSG, SW_SHOW, WM_CREATE, WM_DESTROY, WM_LBUTTONDOWN,
        WM_PAINT, WM_RBUTTONDOWN, WM_TIMER, WNDCLASSW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
        WS_VISIBLE,
    },
};

#[cfg(target_os = "windows")]
const WINDOW_WIDTH: i32 = 132;
#[cfg(target_os = "windows")]
const WINDOW_HEIGHT: i32 = 44;
#[cfg(target_os = "windows")]
const AUTO_CLOSE_TIMER: usize = 1;
#[cfg(target_os = "windows")]
const AUTO_CLOSE_MS: u32 = 30_000;

#[cfg(target_os = "windows")]
struct HelperState {
    path: PathBuf,
}

#[cfg(target_os = "windows")]
fn main() {
    let Some(path) = std::env::args_os().nth(1).map(PathBuf::from) else {
        std::process::exit(2);
    };

    match run_drag_window(path) {
        Ok(()) => std::process::exit(0),
        Err(_) => std::process::exit(1),
    }
}

#[cfg(target_os = "windows")]
fn run_drag_window(path: PathBuf) -> Result<(), String> {
    if !path.is_file() {
        return Err("MIDI file does not exist".to_string());
    }

    unsafe {
        let instance = GetModuleHandleW(null());
        if instance.is_null() {
            return Err("GetModuleHandleW failed".to_string());
        }

        let class_name = wide_null("DrumFlashMidiDragHelper");
        let cursor = LoadCursorW(null_mut(), IDC_HAND);
        let window_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: null_mut(),
            hCursor: cursor,
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: class_name.as_ptr(),
        };

        if RegisterClassW(&window_class) == 0 {
            return Err("RegisterClassW failed".to_string());
        }

        let mut cursor_pos = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut cursor_pos) == 0 {
            cursor_pos.x = 320;
            cursor_pos.y = 240;
        }

        let title = wide_null("Drum Flash MIDI Drag");
        let state = Box::into_raw(Box::new(HelperState { path }));
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_VISIBLE,
            cursor_pos.x - (WINDOW_WIDTH / 2),
            cursor_pos.y - (WINDOW_HEIGHT / 2),
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            null_mut(),
            null_mut(),
            instance,
            state as *const c_void,
        );

        if hwnd.is_null() {
            drop(Box::from_raw(state));
            return Err("CreateWindowExW failed".to_string());
        }

        let region = CreateRoundRectRgn(0, 0, WINDOW_WIDTH + 1, WINDOW_HEIGHT + 1, 18, 18);
        if !region.is_null() && SetWindowRgn(hwnd, region, 1) == 0 {
            DeleteObject(region);
        }

        ShowWindow(hwnd, SW_SHOW);

        let mut message: MSG = zeroed();
        while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CREATE => {
            let create = &*(lparam as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
            SetTimer(hwnd, AUTO_CLOSE_TIMER, AUTO_CLOSE_MS, None);
            0
        }
        WM_PAINT => {
            paint_window(hwnd);
            0
        }
        WM_LBUTTONDOWN => {
            KillTimer(hwnd, AUTO_CLOSE_TIMER);
            let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const HelperState;
            if !state.is_null() {
                let _ = native_drag::start_midi_file_drag(&(*state).path);
            }
            DestroyWindow(hwnd);
            0
        }
        WM_RBUTTONDOWN | WM_TIMER => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut HelperState;
            if !state.is_null() {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                drop(Box::from_raw(state));
            }
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

#[cfg(target_os = "windows")]
unsafe fn paint_window(hwnd: HWND) {
    let mut paint: PAINTSTRUCT = zeroed();
    let hdc = BeginPaint(hwnd, &mut paint);
    if hdc.is_null() {
        return;
    }

    let mut rect = RECT {
        left: 0,
        top: 0,
        right: WINDOW_WIDTH,
        bottom: WINDOW_HEIGHT,
    };
    let brush = CreateSolidBrush(rgb(32, 36, 42));
    if !brush.is_null() {
        FillRect(hdc, &rect, brush);
        DeleteObject(brush);
    }

    let accent = CreateSolidBrush(rgb(58, 139, 255));
    if !accent.is_null() {
        let accent_rect = RECT {
            left: 0,
            top: 0,
            right: 6,
            bottom: WINDOW_HEIGHT,
        };
        FillRect(hdc, &accent_rect, accent);
        DeleteObject(accent);
    }

    let border = CreateSolidBrush(rgb(74, 82, 94));
    if !border.is_null() {
        FrameRect(hdc, &rect, border);
        DeleteObject(border);
    }

    SetBkMode(hdc, TRANSPARENT as i32);
    SetTextColor(hdc, rgb(238, 242, 247));
    let font_name = wide_null("Segoe UI Semibold");
    let font = CreateFontW(
        -15,
        0,
        0,
        0,
        600,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        font_name.as_ptr(),
    );
    let old_font = if font.is_null() {
        null_mut()
    } else {
        SelectObject(hdc, font)
    };

    let text = wide_null("MIDI DRAG");
    rect.left = 10;
    rect.right = WINDOW_WIDTH - 8;
    DrawTextW(
        hdc,
        text.as_ptr(),
        -1,
        &mut rect,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );

    if !old_font.is_null() {
        SelectObject(hdc, old_font);
    }
    if !font.is_null() {
        DeleteObject(font);
    }

    EndPaint(hwnd, &paint);
}

#[cfg(target_os = "windows")]
fn wide_null(value: &str) -> Vec<u16> {
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(target_os = "windows")]
const fn rgb(red: u8, green: u8, blue: u8) -> COLORREF {
    red as u32 | ((green as u32) << 8) | ((blue as u32) << 16)
}

#[cfg(not(target_os = "windows"))]
fn main() {
    std::process::exit(1);
}

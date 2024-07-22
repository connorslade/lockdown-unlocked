use std::{ffi::c_void, iter, mem};

use crate::{hook::LazyHook, hooks::CHROME_WINDOW, log};
use anyhow::Result;
use windows::{
    core::{s, PCWSTR},
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        System::LibraryLoader::GetProcAddress,
        UI::WindowsAndMessaging::{
            DefWindowProcW, GetClassInfoExW, GetWindowRect, RegisterClassExW, SetWindowPos, HMENU,
            MA_NOACTIVATEANDEAT, SWP_NOSIZE, SWP_NOZORDER, WINDOW_EX_STYLE, WINDOW_STYLE,
            WM_ACTIVATE, WM_INPUT, WM_KILLFOCUS, WM_MOUSEACTIVATE, WM_MOVE, WM_POWERBROADCAST,
            WM_SETFOCUS, WM_SHOWWINDOW, WM_SYSCOMMAND, WM_SYSKEYDOWN, WM_SYSKEYUP, WNDCLASSEXW,
            WS_CAPTION,
        },
    },
};

use super::{BROWSER_WINDOWS, LIB_USER32};

const WINDOW_SIZE: (i32, i32) = (1920, 1080);
const CHROME_NAME: &str = "Respondus LockDown Browser";
const BROWSER_NAME: &str = "LockDown Browser";

static mut CREATE_WINDOW_EXA_HOOK: LazyHook = LazyHook::new();

pub unsafe fn init() -> Result<()> {
    let create_win_proc = GetProcAddress(*LIB_USER32, s!("CreateWindowExW")).unwrap();

    CREATE_WINDOW_EXA_HOOK
        .init(
            create_win_proc as *const c_void,
            create_win_detour as *const c_void,
        )
        .hook()?;

    Ok(())
}

type CreateWindowExW = unsafe extern "system" fn(
    WINDOW_EX_STYLE,
    PCWSTR,
    PCWSTR,
    WINDOW_STYLE,
    i32,
    i32,
    i32,
    i32,
    HWND,
    HMENU,
    HINSTANCE,
    *const c_void,
) -> HWND;

unsafe extern "system" fn create_win_detour(
    dwexstyle: WINDOW_EX_STYLE,
    mut lpclassname: PCWSTR,
    lpwindowname: PCWSTR,
    mut dwstyle: WINDOW_STYLE,
    mut x: i32,
    mut y: i32,
    mut nwidth: i32,
    mut nheight: i32,
    hwndparent: HWND,
    hmenu: HMENU,
    hinstance: HINSTANCE,
    lpparam: *const c_void,
) -> HWND {
    let mut old_proc = None;
    let mut is_browser = false;

    if !lpwindowname.is_null() {
        let name_len = (0..).find(|&i| *lpwindowname.0.add(i) == 0).unwrap();
        let name = String::from_utf16_lossy(std::slice::from_raw_parts(lpwindowname.0, name_len));
        log!("CreateWindowExW - {name}");

        if name == CHROME_NAME {
            if CHROME_WINDOW.is_some() {
                panic!("Already hooked");
            }

            x += 100;
            y += 100;
            nwidth = WINDOW_SIZE.0;
            nheight = WINDOW_SIZE.1;
            dwstyle |= WS_CAPTION;

            let mut class_info = std::mem::zeroed::<WNDCLASSEXW>();
            GetClassInfoExW(hinstance, lpclassname, &mut class_info).unwrap();
            old_proc = Some(class_info.lpfnWndProc.unwrap());

            let class_name = String::from("LockDownBrowserChromeHooked")
                .encode_utf16()
                .chain(iter::once(0))
                .collect::<Vec<_>>();
            let class_name_ptr = Box::into_raw(class_name.into_boxed_slice()) as *const u16;

            let new_class = WNDCLASSEXW {
                lpszClassName: PCWSTR(class_name_ptr),
                lpfnWndProc: Some(chrome_window_proc),
                cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
                ..class_info
            };
            RegisterClassExW(&new_class);

            lpclassname = PCWSTR(class_name_ptr);
        } else if name == BROWSER_NAME {
            is_browser = true;
            nwidth = WINDOW_SIZE.0 - 16;
            nheight = WINDOW_SIZE.1 - 64;

            if let Some((chrome_hwnd, _)) = CHROME_WINDOW {
                let mut rect = std::mem::zeroed::<RECT>();
                GetWindowRect(chrome_hwnd, &mut rect).unwrap();
                x = rect.left + 8;
                y = rect.top + 64 + 32;
            }
        }
    }

    log!("old_proc: {:?}", old_proc);
    let hwnd = CREATE_WINDOW_EXA_HOOK
        .trampoline::<CreateWindowExW, _>(|func| {
            func(
                dwexstyle,
                lpclassname,
                lpwindowname,
                dwstyle,
                x,
                y,
                nwidth,
                nheight,
                hwndparent,
                hmenu,
                hinstance,
                lpparam,
            )
        })
        .unwrap();

    if let Some(old_proc) = old_proc {
        CHROME_WINDOW = Some((hwnd, old_proc));
    }

    if is_browser {
        BROWSER_WINDOWS.push(hwnd);
    }

    hwnd
}

unsafe extern "system" fn chrome_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_MOVE => {
            for browser_hwnd in BROWSER_WINDOWS.iter() {
                let mut rect = std::mem::zeroed::<RECT>();
                GetWindowRect(hwnd, &mut rect).unwrap();
                let _ = SetWindowPos(
                    *browser_hwnd,
                    HWND::default(),
                    rect.left + 8,
                    rect.top + 64 + 32,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER,
                );
            }
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
        WM_ACTIVATE | WM_SHOWWINDOW | WM_SYSCOMMAND | WM_SYSKEYDOWN | WM_SYSKEYUP | WM_INPUT
        | WM_SETFOCUS | WM_KILLFOCUS => return LRESULT::default(),
        WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATEANDEAT as isize),
        WM_POWERBROADCAST => return LRESULT(1),
        _ => {}
    }

    match CHROME_WINDOW {
        Some((_, old_proc)) => old_proc(hwnd, msg, wparam, lparam),
        None => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

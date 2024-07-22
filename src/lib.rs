#![cfg(windows)]

use std::{ffi::c_void, iter, mem, panic, process};

use anyhow::Result;
use hook::LazyHook;
use windows::{
    core::{s, PCSTR, PCWSTR},
    Win32::{
        Foundation::{
            GetLastError, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, MAX_PATH, RECT, WPARAM,
        },
        System::{
            Diagnostics::Debug::OutputDebugStringA,
            Environment::GetCommandLineA,
            LibraryLoader::{GetProcAddress, LoadLibraryA},
            ProcessStatus::GetProcessImageFileNameA,
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
        },
        UI::WindowsAndMessaging::{
            DefWindowProcW, GetClassInfoExW, GetWindowRect, RegisterClassExW, SetWindowPos, HMENU,
            SWP_NOSIZE, SWP_NOZORDER, WINDOW_EX_STYLE, WINDOW_STYLE, WM_ACTIVATE, WM_MOUSEACTIVATE,
            WM_MOVE, WM_SHOWWINDOW, WM_SYSCOMMAND, WM_SYSKEYDOWN, WM_SYSKEYUP, WNDCLASSEXW,
            WS_CAPTION,
        },
    },
};

mod hook;

macro_rules! log {
    ($($arg:tt)*) => {{
        let msg = to_pcstr(&format!($($arg)*));
        OutputDebugStringA(PCSTR(msg.as_ptr()));
    }};
}

#[no_mangle]
#[allow(non_snake_case, unused_variables)]
unsafe extern "system" fn DllMain(
    dll_module: HINSTANCE,
    call_reason: u32,
    reserved: *mut (),
) -> bool {
    let mut buf = [0u8; MAX_PATH as usize];
    GetProcessImageFileNameA(HANDLE(usize::MAX as *mut c_void), &mut buf);
    let name = String::from_utf8_lossy(&buf);

    let result = match call_reason {
        DLL_PROCESS_ATTACH => {
            log!("DLL Injected into: {name}");
            panic::set_hook(Box::new(|info| {
                log!("== Panic ==");
                log!("{info}");
                process::abort();
            }));
            process_attach()
        }
        DLL_PROCESS_DETACH => {
            log!("DLL Unloaded from: {name}");
            process_detach()
        }
        _ => return true,
    };

    handle_error(result);
    true
}

unsafe fn handle_error(result: Result<()>) {
    if let Err(err) = result {
        log!("Error: {err}");
        let last_error = GetLastError();
        log!("Last Error: {}", last_error.0);
        log!("{}", err.backtrace());
    }
}

#[no_mangle]
unsafe extern "system" fn enum_processes_detour(
    lpidprocess: *mut u32,
    _cb: u32,
    lpcbneeded: *mut u32,
) -> i32 {
    log!("EnumProcesses");
    *lpidprocess = 0;
    *lpcbneeded = 0;
    1
}

const WINDOW_SIZE: (i32, i32) = (1920, 1080);
const CHROME_NAME: &str = "Respondus LockDown Browser";
const BROWSER_NAME: &str = "LockDown Browser";

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

type WindowProc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

static mut CHROME_WINDOW: Option<(HWND, WindowProc)> = None;
static mut BROWSER_WINDOWS: Vec<HWND> = Vec::new();

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
        }
        WM_ACTIVATE | WM_SHOWWINDOW | WM_MOUSEACTIVATE | WM_SYSCOMMAND | WM_SYSKEYDOWN
        | WM_SYSKEYUP => {
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
        _ => {}
    }

    match CHROME_WINDOW {
        Some((_, old_proc)) => old_proc(hwnd, msg, wparam, lparam),
        None => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

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

type GetForegroundWindow = unsafe extern "system" fn() -> HWND;
unsafe extern "system" fn get_foreground_window_detour() -> HWND {
    if let Some((hwnd, _)) = CHROME_WINDOW {
        return hwnd;
    }

    GET_FOREGROUND_WINDOW_HOOK
        .trampoline::<GetForegroundWindow, HWND>(|func| func())
        .unwrap()
}

static mut ENUM_PROCESSES_HOOK: LazyHook = LazyHook::new();
static mut CREATE_WINDOW_EXA_HOOK: LazyHook = LazyHook::new();
static mut GET_FOREGROUND_WINDOW_HOOK: LazyHook = LazyHook::new();

unsafe fn process_attach() -> Result<()> {
    let cmd = GetCommandLineA();
    let cmd = String::from_utf8_lossy(cmd.as_bytes());
    log!("Command Line: {}", &cmd);

    let lib_psapi = LoadLibraryA(s!("Psapi.dll")).unwrap();
    let lib_user32 = LoadLibraryA(s!("User32.dll")).unwrap();

    let enum_proc = GetProcAddress(lib_psapi, s!("EnumProcesses")).unwrap();
    ENUM_PROCESSES_HOOK
        .init(
            enum_proc as *const c_void,
            enum_processes_detour as *const c_void,
        )
        .hook()?;

    let create_win_proc = GetProcAddress(lib_user32, s!("CreateWindowExW")).unwrap();
    CREATE_WINDOW_EXA_HOOK
        .init(
            create_win_proc as *const c_void,
            create_win_detour as *const c_void,
        )
        .hook()?;

    let get_foreground_window = GetProcAddress(lib_user32, s!("GetForegroundWindow")).unwrap();
    GET_FOREGROUND_WINDOW_HOOK
        .init(
            get_foreground_window as *const c_void,
            get_foreground_window_detour as *const c_void,
        )
        .hook()?;

    Ok(())
}

unsafe fn process_detach() -> Result<()> {
    Ok(())
}

fn to_pcstr(s: &str) -> Vec<u8> {
    s.bytes().chain(iter::once(0)).collect()
}

#![cfg(windows)]

use std::{ffi::c_void, iter, mem, panic, process, thread, time::Duration};

use anyhow::Result;
use hook::LazyHook;
use windows::{
    core::{s, PCSTR, PCWSTR},
    Win32::{
        Foundation::{GetLastError, BOOL, HANDLE, HINSTANCE, HWND, LPARAM, MAX_PATH},
        System::{
            Diagnostics::Debug::{OutputDebugStringA, ReadProcessMemory, WriteProcessMemory},
            Environment::GetCommandLineA,
            LibraryLoader::{GetProcAddress, LoadLibraryA},
            ProcessStatus::GetProcessImageFileNameA,
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
            Threading::GetCurrentProcess,
        },
        UI::WindowsAndMessaging::{
            CreateWindowExA, EnumWindows, GetWindowLongA, GetWindowTextA, MoveWindow, GWL_STYLE,
            HMENU, WINDOW_EX_STYLE, WINDOW_STYLE,
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

unsafe extern "system" fn window_enum(hwnd: HWND, _lparam: LPARAM) -> BOOL {
    let mut string = [0u8; 1024];
    GetWindowTextA(hwnd, &mut string);

    let end = string.iter().position(|&c| c == 0).unwrap_or(0);
    let title = String::from_utf8_lossy(&string[..end]);

    let style = GetWindowLongA(hwnd, GWL_STYLE);
    log!("Window: {:?} - {} - {style}", hwnd.0, title);

    if title == "Respondus LockDown Browser" {
        MoveWindow(hwnd, 100, 100, WINDOW_SIZE.0, WINDOW_SIZE.1, BOOL(1)).unwrap();
    } else if title == "LockDown Browser" {
        MoveWindow(
            hwnd,
            100,
            100 + 64,
            WINDOW_SIZE.0,
            WINDOW_SIZE.1 - 64,
            BOOL(1),
        )
        .unwrap();
    }

    BOOL(1)
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

#[rustfmt::skip]
unsafe extern "system" fn create_win_detour(
    dwexstyle: WINDOW_EX_STYLE,
    lpclassname: PCWSTR,
    lpwindowname: PCWSTR,
    dwstyle: WINDOW_STYLE,
    x: i32,
    y: i32,
    nwidth: i32,
    nheight: i32,
    hwndparent: HWND,
    hmenu: HMENU,
    hinstance: HINSTANCE,
    lpparam: *const c_void,
) -> HWND {
    log!("CreateWindowExW");

    let trampoline = CREATE_WINDOW_EXA_HOOK.trampoline::<CreateWindowExW>();
    trampoline(dwexstyle, lpclassname, lpwindowname, dwstyle, x, y, nwidth, nheight, hwndparent, hmenu, hinstance, lpparam)
}

static mut ENUM_PROCESSES_HOOK: LazyHook = LazyHook::new();
static mut CREATE_WINDOW_EXA_HOOK: LazyHook = LazyHook::new();

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

    thread::spawn(|| {
        log!("Thread started");
        thread::sleep(Duration::from_secs(10));
        EnumWindows(Some(window_enum), LPARAM(0))
    });

    Ok(())
}

unsafe fn process_detach() -> Result<()> {
    Ok(())
}

fn to_pcstr(s: &str) -> Vec<u8> {
    s.bytes().chain(iter::once(0)).collect()
}

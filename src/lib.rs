#![cfg(windows)]

use std::{ffi::c_void, iter, panic, process, thread, time::Duration};

use anyhow::Result;
use windows::{
    core::{s, PCSTR},
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
            EnumWindows, GetWindowLongA, GetWindowTextA, MoveWindow, GWL_STYLE,
        },
    },
};

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
    *lpidprocess = 0;
    *lpcbneeded = 0;
    1
}

static mut ENUM_PROCESSES_ORIGINAL_BYTES: [u8; 6] = [0; 6];
static mut BYTES_WRITTEN: usize = 0;
static mut ENUM_PROCESSES_ADDRESS: Option<unsafe extern "system" fn() -> isize> = None;

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

unsafe fn process_attach() -> Result<()> {
    let cmd = GetCommandLineA();
    let cmd = String::from_utf8_lossy(cmd.as_bytes());
    log!("Command Line: {}", &cmd);

    let dll_handle = LoadLibraryA(s!("Psapi.dll")).unwrap();
    let bytes_read: usize = 0;

    ENUM_PROCESSES_ADDRESS = GetProcAddress(dll_handle, s!("EnumProcesses"));

    if ENUM_PROCESSES_ADDRESS.is_none() {
        return Err(anyhow::anyhow!("Failed to get EnumProcesses address"));
    }

    ReadProcessMemory(
        GetCurrentProcess(),
        ENUM_PROCESSES_ADDRESS.unwrap() as *const c_void,
        ENUM_PROCESSES_ORIGINAL_BYTES.as_ptr() as *mut c_void,
        6,
        Some(bytes_read as *mut usize),
    )
    .unwrap();

    let hooked_message_box_address = (enum_processes_detour as *mut ()).cast::<c_void>();
    let offset = hooked_message_box_address as isize;
    let mut patch = [0; 6];
    patch[0] = 0x68;
    let temp = offset.to_ne_bytes();
    patch[1..5].copy_from_slice(&temp[..4]);
    patch[5] = 0xC3;

    WriteProcessMemory(
        GetCurrentProcess(),
        ENUM_PROCESSES_ADDRESS.unwrap() as *const c_void,
        patch.as_ptr().cast::<c_void>(),
        6,
        Some(BYTES_WRITTEN as *mut usize),
    )
    .unwrap();

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

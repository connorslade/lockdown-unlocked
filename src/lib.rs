#![cfg(windows)]

use std::{ffi::c_void, iter, panic, process};

use anyhow::Result;
use windows::{
    core::PCSTR,
    Win32::{
        Foundation::{GetLastError, HANDLE, HINSTANCE, MAX_PATH},
        System::{
            Diagnostics::Debug::OutputDebugStringA,
            ProcessStatus::GetProcessImageFileNameA,
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
        },
    },
};

mod hook;
mod hooks;

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        let msg = $crate::to_pcstr(&format!($($arg)*));
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

unsafe fn process_attach() -> Result<()> {
    hooks::init()?;
    Ok(())
}

unsafe fn process_detach() -> Result<()> {
    Ok(())
}

fn to_pcstr(s: &str) -> Vec<u8> {
    s.bytes().chain(iter::once(0)).collect()
}

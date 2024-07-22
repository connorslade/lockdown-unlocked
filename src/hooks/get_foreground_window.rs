use std::ffi::c_void;

use anyhow::Result;
use windows::{
    core::s,
    Win32::{
        Foundation::HWND,
        System::LibraryLoader::{GetProcAddress, LoadLibraryA},
    },
};

use crate::hook::LazyHook;

use super::CHROME_WINDOW;

static mut GET_FOREGROUND_WINDOW_HOOK: LazyHook = LazyHook::new();

pub unsafe fn init() -> Result<()> {
    let lib_user32 = LoadLibraryA(s!("User32.dll")).unwrap();
    let get_foreground_window = GetProcAddress(lib_user32, s!("GetForegroundWindow")).unwrap();

    GET_FOREGROUND_WINDOW_HOOK
        .init(
            get_foreground_window as *const c_void,
            get_foreground_window_detour as *const c_void,
        )
        .hook()?;

    Ok(())
}

unsafe extern "system" fn get_foreground_window_detour() -> HWND {
    if let Some((hwnd, _)) = CHROME_WINDOW {
        return hwnd;
    }

    HWND::default()
}

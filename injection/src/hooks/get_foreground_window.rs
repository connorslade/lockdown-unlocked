use std::ffi::c_void;

use anyhow::Result;
use windows::{
    core::s,
    Win32::{Foundation::HWND, System::LibraryLoader::GetProcAddress},
};

use crate::hook::LazyHook;

use super::{CHROME_WINDOW, LIB_USER32};

static mut GET_FOREGROUND_WINDOW_HOOK: LazyHook = LazyHook::new();

pub unsafe fn init() -> Result<()> {
    let get_foreground_window = GetProcAddress(*LIB_USER32, s!("GetForegroundWindow")).unwrap();

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

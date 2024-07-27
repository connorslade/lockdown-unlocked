use std::ffi::c_void;

use anyhow::Result;
use windows::{
    core::s,
    Win32::{
        Foundation::HINSTANCE,
        System::LibraryLoader::GetProcAddress,
        UI::WindowsAndMessaging::{HHOOK, HOOKPROC},
    },
};

use crate::{hook::LazyHook, log};

use super::LIB_USER32;

static mut SET_WINDOWS_HOOK_EXA_HOOK: LazyHook = LazyHook::new();

pub unsafe fn init() -> Result<()> {
    let set_windows_hook_exa = GetProcAddress(*LIB_USER32, s!("SetWindowsHookExA")).unwrap();

    SET_WINDOWS_HOOK_EXA_HOOK
        .init(
            set_windows_hook_exa as *const c_void,
            set_windows_hook_detour as *const c_void,
        )
        .hook()?;

    Ok(())
}

unsafe extern "system" fn set_windows_hook_detour(
    id_hook: i32,
    _lpfn: HOOKPROC,
    _hmod: HINSTANCE,
    _dw_thread_id: u32,
) -> HHOOK {
    log!("SetWindowHookExA called - {id_hook}");
    HHOOK::default()
}

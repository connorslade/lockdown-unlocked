use std::ffi::c_void;

use anyhow::Result;
use windows::{
    core::s,
    Win32::{Foundation::BOOL, System::LibraryLoader::GetProcAddress},
};

use crate::{hook::LazyHook, log};

use super::LIB_USER32;

static mut EMPTY_CLIPBOARD_HOOK: LazyHook = LazyHook::new();

pub unsafe fn init() -> Result<()> {
    let empty_clipboard = GetProcAddress(*LIB_USER32, s!("EmptyClipboard")).unwrap();

    EMPTY_CLIPBOARD_HOOK
        .init(
            empty_clipboard as *const c_void,
            empty_clipboard_detour as *const c_void,
        )
        .hook()?;

    Ok(())
}

unsafe extern "system" fn empty_clipboard_detour() -> BOOL {
    log!("EmptyClipboard called");
    BOOL(1)
}

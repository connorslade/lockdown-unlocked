use std::ffi::c_void;

use anyhow::Result;
use windows::{
    core::s,
    Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA},
};

use crate::hook::LazyHook;

static mut ENUM_PROCESSES_HOOK: LazyHook = LazyHook::new();

pub unsafe fn init() -> Result<()> {
    let lib_psapi = LoadLibraryA(s!("Psapi.dll")).unwrap();

    let enum_proc = GetProcAddress(lib_psapi, s!("EnumProcesses")).unwrap();
    ENUM_PROCESSES_HOOK
        .init(
            enum_proc as *const c_void,
            enum_processes_detour as *const c_void,
        )
        .hook()?;
    Ok(())
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

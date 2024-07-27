use std::{ffi::c_void, mem};

use anyhow::Result;
use windows::Win32::{
    Foundation::HANDLE,
    Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};
use winreg::{self, enums::HKEY_CLASSES_ROOT, RegKey};

pub unsafe fn is_admin() -> Result<bool> {
    let mut token_handle = HANDLE::default();

    OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle)?;

    let mut length = 0;
    let mut token_elevation = TOKEN_ELEVATION::default();

    GetTokenInformation(
        token_handle,
        TokenElevation,
        Some(&mut token_elevation as *mut _ as *mut c_void),
        mem::size_of::<TOKEN_ELEVATION>() as u32,
        &mut length,
    )?;

    Ok(token_elevation.TokenIsElevated != 0)
}

/// Sets `HKEY_CLASSES_ROOT\Computer\HKEY_CLASSES_ROOT\rldb\shell\open\command` to the current executable path.
pub unsafe fn register_link_handler() -> Result<()> {
    let exe_path = std::env::current_exe()?;

    let key = r"rldb\shell\open\command";
    let value = format!(r#""{}" "%1""#, exe_path.display());

    let root = RegKey::predef(HKEY_CLASSES_ROOT);
    let (key, _disp) = root.create_subkey(key)?;

    key.set_value("", &value)?;
    Ok(())
}

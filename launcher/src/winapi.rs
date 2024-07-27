use std::{ffi::c_void, mem};

use anyhow::Result;
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{HANDLE, MAX_PATH},
        Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
        System::{
            Environment::{GetCommandLineW, GetCurrentDirectoryW},
            LibraryLoader::GetModuleFileNameW,
            Threading::{GetCurrentProcess, OpenProcessToken},
        },
        UI::{
            Shell::{ShellExecuteExW, SHELLEXECUTEINFOW},
            WindowsAndMessaging::SW_NORMAL,
        },
    },
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

pub unsafe fn relaunch_with_admin() -> Result<()> {
    let mut filename = [0u16; MAX_PATH as usize];
    let len = GetModuleFileNameW(None, &mut filename);
    filename[len as usize] = 0;

    let mut working_dir = [0u16; MAX_PATH as usize];
    let len = GetCurrentDirectoryW(Some(&mut working_dir));
    working_dir[len as usize] = 0;

    ShellExecuteExW(&mut SHELLEXECUTEINFOW {
        lpVerb: w!("runas"),
        lpFile: PCWSTR(filename.as_ptr()),
        lpDirectory: PCWSTR(working_dir.as_ptr()),
        lpParameters: GetCommandLineW(),
        nShow: SW_NORMAL.0,
        cbSize: mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        ..Default::default()
    })?;

    Ok(())
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

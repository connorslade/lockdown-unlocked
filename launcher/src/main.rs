use std::{ffi::c_void, mem, process::Command};

use anyhow::Result;
use dll_syringe::{process::OwnedProcess, Syringe};
use windows::Win32::{
    Foundation::HANDLE,
    Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

const EXE_PATH: &str = r"D:\Sandbox\LockdownBrowser\drive\C\Program Files (x86)\Respondus\LockDown Browser\LockDownBrowser.exe";

fn main() -> Result<()> {
    if !unsafe { is_admin() }? {
        println!("Please run this program as an administrator.");
        return Ok(());
    }

    let mut proc = Command::new(EXE_PATH)
        .spawn()
        .expect("Failed to start LockDown Browser");

    let owned_proc = OwnedProcess::from_pid(proc.id()).unwrap();
    let syringe = Syringe::for_process(owned_proc);
    syringe
        .inject("target/i686-pc-windows-msvc/release/injection.dll")
        .unwrap();

    proc.wait().unwrap();
    Ok(())
}

unsafe fn is_admin() -> Result<bool> {
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

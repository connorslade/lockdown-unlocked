use std::{ffi::c_void, mem, process::Command};

use anyhow::Result;
use dll_syringe::{process::OwnedProcess, Syringe};
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

const EXE_PATH: &str = r"D:\Sandbox\LockdownBrowser\drive\C\Program Files (x86)\Respondus\LockDown Browser\LockDownBrowser.exe";

fn main() -> Result<()> {
    if !unsafe { is_admin() }? {
        println!("This program requires admin privileges, relaunching...");
        unsafe { relaunch_with_admin()? };
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
    println!("Successfully injected DLL into LockDown Browser.");

    proc.wait()?;
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

unsafe fn relaunch_with_admin() -> Result<()> {
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

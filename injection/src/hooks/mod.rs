use anyhow::Result;
use once_cell::sync::Lazy;
use windows::{
    core::s,
    Win32::{
        Foundation::{HMODULE, HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::LoadLibraryA,
    },
};

mod create_window_exw;
mod empty_clipboard;
mod enum_processes;
mod get_foreground_window;
mod set_windows_hook_exa;

type WindowProc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

static mut CHROME_WINDOW: Option<(HWND, WindowProc)> = None;
static mut BROWSER_WINDOWS: Vec<HWND> = Vec::new();

static mut LIB_USER32: Lazy<HMODULE> =
    Lazy::new(|| unsafe { LoadLibraryA(s!("User32.dll")).unwrap() });

pub unsafe fn init() -> Result<()> {
    create_window_exw::init()?;
    empty_clipboard::init()?;
    enum_processes::init()?;
    get_foreground_window::init()?;
    set_windows_hook_exa::init()?;
    Ok(())
}

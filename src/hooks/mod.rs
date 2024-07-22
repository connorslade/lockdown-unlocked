use anyhow::Result;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};

mod create_window_exw;
mod enum_processes;
mod get_foreground_window;

type WindowProc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

static mut CHROME_WINDOW: Option<(HWND, WindowProc)> = None;
static mut BROWSER_WINDOWS: Vec<HWND> = Vec::new();

pub unsafe fn init() -> Result<()> {
    create_window_exw::init()?;
    enum_processes::init()?;
    get_foreground_window::init()?;
    Ok(())
}

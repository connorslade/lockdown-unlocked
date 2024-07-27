use std::{env, process::Command};

use anyhow::{Context, Result};
use config::Config;
use dll_syringe::{process::OwnedProcess, Syringe};

mod config;
mod winapi;
use winapi::{is_admin, register_link_handler};

fn main() -> Result<()> {
    if !unsafe { is_admin() }? {
        println!("[*] This program requires admin privileges, relaunching...");
        // unsafe { relaunch_with_admin()? };
        return Ok(());
    }

    let args = env::args().collect::<Vec<_>>();
    if args.len() == 1 {
        println!("[*] Registering `rldb` link handler...");
        unsafe { register_link_handler() }?;
        return Ok(());
    }

    if args.len() != 2 || !args[1].starts_with("rldb:") {
        println!("[-] Invalid arguments. Please read the documentation at https://github.com/connorslade/lockdown-unlocked.");
        return Ok(());
    }

    let config = Config::load("config.toml")?;

    println!("[*] Starting LockDown Browser. {}", &args[1]);

    let mut proc = Command::new(config.lockdown_browser)
        .arg(&args[1])
        .spawn()
        .context("Failed to start LockDown Browser")?;

    let owned_proc = OwnedProcess::from_pid(proc.id()).unwrap();
    let syringe = Syringe::for_process(owned_proc);
    syringe
        .inject(config.injection)
        .context("Failed to inject DLL into LockDown Browser")?;

    println!("[*] Successfully injected DLL into LockDown Browser.");
    proc.wait()?;

    Ok(())
}

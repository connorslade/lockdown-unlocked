use std::{
    ffi::c_void,
    mem,
    ops::{Deref, DerefMut},
};

use anyhow::Result;
use windows::Win32::System::{
    Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory},
    Threading::GetCurrentProcess,
};

pub unsafe fn hook(function: *const c_void, detour: *const c_void) -> Result<[u8; 6]> {
    let proc = GetCurrentProcess();

    let mut original = [0u8; 6];
    let original_ptr = original.as_mut_ptr() as *mut c_void;
    ReadProcessMemory(proc, function, original_ptr, 6, None)?;

    let mut patch = [0; 6];
    patch[0] = 0x68;
    let temp = (detour as isize).to_ne_bytes();
    patch[1..5].copy_from_slice(&temp[..4]);
    patch[5] = 0xC3;
    WriteProcessMemory(proc, function, patch.as_ptr() as *const c_void, 6, None)?;

    Ok(original)
}

unsafe fn unhook(function: *const c_void, original: [u8; 6]) -> Result<()> {
    let proc = GetCurrentProcess();
    WriteProcessMemory(proc, function, original.as_ptr() as *const c_void, 6, None).unwrap();
    Ok(())
}

pub struct Hook {
    function: *const c_void,
    detour: *const c_void,
    original: Option<[u8; 6]>,
}

impl Hook {
    pub unsafe fn new(function: *const c_void, detour: *const c_void) -> Self {
        Self {
            function,
            detour,
            original: None,
        }
    }

    // so jank ong
    // this has actually gotta be a crime but i dont know enough about x86 assembly to do it better
    pub unsafe fn trampoline<T, K>(&mut self, trampoline: impl FnOnce(T) -> K) -> Result<K> {
        self.unhook()?;
        let out = trampoline(mem::transmute_copy::<_, T>(&self.function));
        self.hook()?;
        Ok(out)
    }

    pub unsafe fn hook(&mut self) -> Result<()> {
        self.original = Some(hook(self.function, self.detour)?);
        Ok(())
    }

    pub unsafe fn unhook(&self) -> Result<()> {
        if let Some(original) = self.original {
            unhook(self.function, original)?;
        }

        Ok(())
    }
}

pub struct LazyHook {
    hook: Option<Hook>,
}

impl LazyHook {
    pub const fn new() -> Self {
        Self { hook: None }
    }

    pub unsafe fn init(&mut self, function: *const c_void, detour: *const c_void) -> &mut Self {
        self.hook = Some(Hook::new(function, detour));
        self
    }
}

impl Deref for LazyHook {
    type Target = Hook;

    fn deref(&self) -> &Self::Target {
        self.hook.as_ref().unwrap()
    }
}

impl DerefMut for LazyHook {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.hook.as_mut().unwrap()
    }
}

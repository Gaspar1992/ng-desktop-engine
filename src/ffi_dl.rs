#![allow(unsafe_code, dead_code, clippy::redundant_pub_crate)]

#[cfg(unix)]
use std::ffi::c_int;
use std::ffi::{CStr, c_char, c_void};

#[cfg(unix)]
#[cfg_attr(target_os = "linux", link(name = "dl"))]
unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

#[cfg(unix)]
const RTLD_LAZY: c_int = 1;

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryA(lp_lib_file_name: *const c_char) -> *mut c_void;
    fn GetProcAddress(h_module: *mut c_void, lp_proc_name: *const c_char) -> *mut c_void;
}

/// Centralized cross-platform dynamic library loader (`dlopen`/`dlsym` on Unix, `LoadLibraryA`/`GetProcAddress` on Windows).
#[derive(Debug, Clone, Copy)]
pub(super) struct DynLib(*mut c_void);

impl DynLib {
    pub(super) unsafe fn open(name: &CStr) -> Option<Self> {
        #[cfg(unix)]
        let handle = unsafe { dlopen(name.as_ptr(), RTLD_LAZY) };
        #[cfg(windows)]
        let handle = unsafe { LoadLibraryA(name.as_ptr()) };
        if handle.is_null() {
            None
        } else {
            Some(Self(handle))
        }
    }

    pub(super) const fn from_ptr(ptr: *mut c_void) -> Self {
        Self(ptr)
    }

    pub(super) const fn as_ptr(self) -> *mut c_void {
        self.0
    }

    pub(super) unsafe fn sym<F: Copy>(self, symbol: &CStr) -> Option<F> {
        #[cfg(unix)]
        let ptr = unsafe { dlsym(self.0, symbol.as_ptr()) };
        #[cfg(windows)]
        let ptr = unsafe { GetProcAddress(self.0, symbol.as_ptr()) };
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { std::mem::transmute_copy::<*mut c_void, F>(&ptr) })
        }
    }
}

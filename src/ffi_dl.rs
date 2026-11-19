#![allow(unsafe_code, clippy::redundant_pub_crate)]

#[cfg(target_os = "linux")]
use std::ffi::{CStr, c_char, c_int, c_void};

#[cfg(target_os = "linux")]
#[link(name = "dl")]
unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

#[cfg(target_os = "linux")]
const RTLD_LAZY: c_int = 1;

/// Centralized dynamic shared library handle (`dlopen` / `dlsym`) isolating OS symbol resolution.
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy)]
pub(super) struct DynLib(*mut c_void);

#[cfg(target_os = "linux")]
impl DynLib {
    pub(super) unsafe fn open(name: &CStr) -> Option<Self> {
        let handle = unsafe { dlopen(name.as_ptr(), RTLD_LAZY) };
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
        let ptr = unsafe { dlsym(self.0, symbol.as_ptr()) };
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { std::mem::transmute_copy::<*mut c_void, F>(&ptr) })
        }
    }
}

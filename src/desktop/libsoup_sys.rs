#![allow(unsafe_code)]

use std::collections::HashMap;
#[cfg(target_os = "linux")]
use std::ffi::c_int;
use std::ffi::{CStr, CString, c_char, c_uint, c_void};

#[cfg(target_os = "linux")]
#[link(name = "dl")]
unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

#[cfg(target_os = "linux")]
const RTLD_LAZY: c_int = 1;

type HeadersForeachFn = unsafe extern "C" fn(*const c_char, *const c_char, *mut c_void);

/// Raw HTTP response returned by `libsoup-3.0`.
pub(super) struct LibsoupRawResponse {
    pub(super) status: u16,
    pub(super) status_text: String,
    pub(super) body_bytes: Vec<u8>,
    pub(super) headers: HashMap<String, String>,
}

/// Dynamically loaded Linux `libsoup-3.0.so.0` + `GLib` HTTP/2 & TLS session bridge.
/// Keeps a persistent `SoupSession` alive so HTTP/2 multiplexed sockets and TLS sessions are reused.
pub(super) struct LibsoupSys {
    session: *mut c_void,
    soup_session_set_timeout: unsafe extern "C" fn(*mut c_void, c_uint),
    soup_message_new: unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_void,
    soup_message_get_request_headers: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    soup_message_get_response_headers: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    soup_message_headers_append: unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char),
    soup_message_headers_foreach: unsafe extern "C" fn(*mut c_void, HeadersForeachFn, *mut c_void),
    soup_message_set_request_body_from_bytes:
        unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_void),
    soup_session_send_and_read: unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        *mut c_void,
        *mut *mut c_void,
    ) -> *mut c_void,
    soup_message_get_status: unsafe extern "C" fn(*mut c_void) -> c_uint,
    soup_message_get_reason_phrase: unsafe extern "C" fn(*mut c_void) -> *const c_char,
    g_bytes_new: unsafe extern "C" fn(*const c_void, usize) -> *mut c_void,
    g_bytes_get_data: unsafe extern "C" fn(*mut c_void, *mut usize) -> *const c_void,
    g_bytes_unref: unsafe extern "C" fn(*mut c_void),
    g_error_free: unsafe extern "C" fn(*mut c_void),
    g_object_unref: unsafe extern "C" fn(*mut c_void),
}

// Guarded externally by `Mutex` inside `NativeHttpEngine`; GLib SoupSession handles mutexed sync calls.
unsafe impl Send for LibsoupSys {}
unsafe impl Sync for LibsoupSys {}

extern "C" fn collect_header_entry(name: *const c_char, value: *const c_char, data: *mut c_void) {
    if name.is_null() || value.is_null() || data.is_null() {
        return;
    }
    unsafe {
        let map = &mut *data.cast::<HashMap<String, String>>();
        let k = CStr::from_ptr(name).to_string_lossy().to_ascii_lowercase();
        let v = CStr::from_ptr(value).to_string_lossy().into_owned();
        map.insert(k, v);
    }
}

pub(super) fn default_reason_phrase(status: u16) -> String {
    match status {
        200 => String::from("OK"),
        201 => String::from("Created"),
        204 => String::from("No Content"),
        400 => String::from("Bad Request"),
        401 => String::from("Unauthorized"),
        403 => String::from("Forbidden"),
        404 => String::from("Not Found"),
        500 => String::from("Internal Server Error"),
        _ if status < 400 => String::from("OK"),
        _ => String::from("Error"),
    }
}

impl LibsoupSys {
    /// Attempts to load `libsoup-3.0.so.0` on Linux and allocate a reusable `SoupSession`.
    #[cfg(target_os = "linux")]
    pub(super) fn try_load() -> Option<Self> {
        unsafe {
            let soup_lib = Self::open_lib(c"libsoup-3.0.so.0")?;
            let glib_lib = Self::open_lib(c"libglib-2.0.so.0")?;
            let gobj_lib = Self::open_lib(c"libgobject-2.0.so.0")?;

            let soup_session_new: unsafe extern "C" fn() -> *mut c_void =
                Self::load_fn(soup_lib, c"soup_session_new")?;
            let session = soup_session_new();
            if session.is_null() {
                return None;
            }

            let soup_session_set_timeout: unsafe extern "C" fn(*mut c_void, c_uint) =
                Self::load_fn(soup_lib, c"soup_session_set_timeout")?;
            soup_session_set_timeout(session, 15);

            Some(Self {
                session,
                soup_session_set_timeout,
                soup_message_new: Self::load_fn(soup_lib, c"soup_message_new")?,
                soup_message_get_request_headers: Self::load_fn(
                    soup_lib,
                    c"soup_message_get_request_headers",
                )?,
                soup_message_get_response_headers: Self::load_fn(
                    soup_lib,
                    c"soup_message_get_response_headers",
                )?,
                soup_message_headers_append: Self::load_fn(
                    soup_lib,
                    c"soup_message_headers_append",
                )?,
                soup_message_headers_foreach: Self::load_fn(
                    soup_lib,
                    c"soup_message_headers_foreach",
                )?,
                soup_message_set_request_body_from_bytes: Self::load_fn(
                    soup_lib,
                    c"soup_message_set_request_body_from_bytes",
                )?,
                soup_session_send_and_read: Self::load_fn(soup_lib, c"soup_session_send_and_read")?,
                soup_message_get_status: Self::load_fn(soup_lib, c"soup_message_get_status")?,
                soup_message_get_reason_phrase: Self::load_fn(
                    soup_lib,
                    c"soup_message_get_reason_phrase",
                )?,
                g_bytes_new: Self::load_fn(glib_lib, c"g_bytes_new")?,
                g_bytes_get_data: Self::load_fn(glib_lib, c"g_bytes_get_data")?,
                g_bytes_unref: Self::load_fn(glib_lib, c"g_bytes_unref")?,
                g_error_free: Self::load_fn(glib_lib, c"g_error_free")?,
                g_object_unref: Self::load_fn(gobj_lib, c"g_object_unref")?,
            })
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub(super) fn try_load() -> Option<Self> {
        None
    }

    #[cfg(target_os = "linux")]
    unsafe fn open_lib(name: &CStr) -> Option<*mut c_void> {
        let handle = unsafe { dlopen(name.as_ptr(), RTLD_LAZY) };
        if handle.is_null() { None } else { Some(handle) }
    }

    #[cfg(target_os = "linux")]
    unsafe fn load_fn<F: Copy>(handle: *mut c_void, sym: &CStr) -> Option<F> {
        let ptr = unsafe { dlsym(handle, sym.as_ptr()) };
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { std::mem::transmute_copy::<*mut c_void, F>(&ptr) })
        }
    }

    /// Updates timeout in seconds on the underlying `SoupSession`.
    pub(super) fn set_timeout_ms(&self, timeout_ms: u32) {
        let secs = timeout_ms.div_ceil(1000).max(1);
        unsafe {
            (self.soup_session_set_timeout)(self.session, secs);
        }
    }

    /// Executes an HTTP request over the persistent `SoupSession`.
    pub(super) fn send_raw(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<&str>,
    ) -> Option<LibsoupRawResponse> {
        let c_method = CString::new(method.trim().to_ascii_uppercase()).ok()?;
        let c_url = CString::new(url.trim()).ok()?;
        unsafe {
            let msg = (self.soup_message_new)(c_method.as_ptr(), c_url.as_ptr());
            if msg.is_null() {
                return None;
            }
            self.apply_request_headers(msg, headers);
            if let Some(payload) = body {
                self.attach_request_body(msg, headers, payload);
            }
            let res = self.execute_and_read(msg);
            (self.g_object_unref)(msg);
            res
        }
    }

    unsafe fn apply_request_headers(&self, msg: *mut c_void, headers: &HashMap<String, String>) {
        let req_headers = unsafe { (self.soup_message_get_request_headers)(msg) };
        if req_headers.is_null() {
            return;
        }
        for (name, value) in headers {
            if let (Ok(c_name), Ok(c_val)) =
                (CString::new(name.as_str()), CString::new(value.as_str()))
            {
                unsafe {
                    (self.soup_message_headers_append)(
                        req_headers,
                        c_name.as_ptr(),
                        c_val.as_ptr(),
                    );
                }
            }
        }
    }

    unsafe fn attach_request_body(
        &self,
        msg: *mut c_void,
        headers: &HashMap<String, String>,
        payload: &str,
    ) {
        let bytes = payload.as_bytes();
        let gbytes = unsafe { (self.g_bytes_new)(bytes.as_ptr().cast::<c_void>(), bytes.len()) };
        if gbytes.is_null() {
            return;
        }
        let content_type = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map_or("application/json", |(_, v)| v.as_str());
        if let Ok(c_type) = CString::new(content_type) {
            unsafe {
                (self.soup_message_set_request_body_from_bytes)(msg, c_type.as_ptr(), gbytes);
            }
        }
        unsafe {
            (self.g_bytes_unref)(gbytes);
        }
    }

    unsafe fn execute_and_read(&self, msg: *mut c_void) -> Option<LibsoupRawResponse> {
        let mut error: *mut c_void = std::ptr::null_mut();
        let gbytes = unsafe {
            (self.soup_session_send_and_read)(
                self.session,
                msg,
                std::ptr::null_mut(),
                &raw mut error,
            )
        };
        if !error.is_null() {
            unsafe { (self.g_error_free)(error) };
            if !gbytes.is_null() {
                unsafe { (self.g_bytes_unref)(gbytes) };
            }
            return None;
        }
        let status_u32 = unsafe { (self.soup_message_get_status)(msg) };
        let status = u16::try_from(status_u32).unwrap_or(500);
        let phrase_ptr = unsafe { (self.soup_message_get_reason_phrase)(msg) };
        let status_text = if phrase_ptr.is_null() {
            default_reason_phrase(status)
        } else {
            let s = unsafe { CStr::from_ptr(phrase_ptr) }
                .to_string_lossy()
                .into_owned();
            if s.trim().is_empty() {
                default_reason_phrase(status)
            } else {
                s
            }
        };

        let mut headers = HashMap::new();
        let resp_headers = unsafe { (self.soup_message_get_response_headers)(msg) };
        if !resp_headers.is_null() {
            let map_ptr: *mut HashMap<String, String> = &raw mut headers;
            unsafe {
                (self.soup_message_headers_foreach)(
                    resp_headers,
                    collect_header_entry,
                    map_ptr.cast::<c_void>(),
                );
            }
        }

        let mut body_bytes = Vec::new();
        if !gbytes.is_null() {
            let mut size: usize = 0;
            let data_ptr = unsafe { (self.g_bytes_get_data)(gbytes, &raw mut size) };
            if !data_ptr.is_null() && size > 0 {
                let slice = unsafe { std::slice::from_raw_parts(data_ptr.cast::<u8>(), size) };
                body_bytes.extend_from_slice(slice);
            }
            unsafe { (self.g_bytes_unref)(gbytes) };
        }

        Some(LibsoupRawResponse {
            status,
            status_text,
            body_bytes,
            headers,
        })
    }
}

impl Drop for LibsoupSys {
    fn drop(&mut self) {
        if !self.session.is_null() {
            unsafe {
                (self.g_object_unref)(self.session);
            }
        }
    }
}

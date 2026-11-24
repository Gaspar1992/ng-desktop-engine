#![allow(unsafe_code)]

use crate::ffi_dl::DynLib;
use std::env;
#[cfg(target_os = "linux")]
use std::ffi::c_int;
#[cfg(unix)]
use std::ffi::{CStr, c_char, c_void};
use std::path::{Path, PathBuf};

/// Normalizes a POSIX/GLib locale string (e.g. `es_ES.UTF-8`) into a BCP-47 tag (`es-ES`).
pub(crate) fn normalize_bcp47_locale(raw: &str) -> Option<String> {
    let base = raw.split(['.', '@']).next()?.trim();
    if base.is_empty() || base.eq_ignore_ascii_case("C") || base.eq_ignore_ascii_case("POSIX") {
        return None;
    }
    Some(base.replace('_', "-"))
}

#[cfg(target_os = "linux")]
unsafe fn open_lib(name: &CStr) -> Option<*mut c_void> {
    unsafe { DynLib::open(name).map(DynLib::as_ptr) }
}

#[cfg(target_os = "linux")]
unsafe fn load_sym<F: Copy>(handle: *mut c_void, sym: &CStr) -> Option<F> {
    unsafe { DynLib::from_ptr(handle).sym(sym) }
}

#[cfg(target_os = "linux")]
fn probe_os_locale_and_timezone() -> (Option<String>, Option<String>) {
    type LangNamesFn = unsafe extern "C" fn() -> *const *const c_char;
    type TzNewLocalFn = unsafe extern "C" fn() -> *mut c_void;
    type TzGetIdFn = unsafe extern "C" fn(*mut c_void) -> *const c_char;
    type TzUnrefFn = unsafe extern "C" fn(*mut c_void);

    unsafe {
        let Some(glib) = open_lib(c"libglib-2.0.so.0") else {
            return (None, None);
        };

        let mut locale = None;
        if let Some(get_langs) = load_sym::<LangNamesFn>(glib, c"g_get_language_names") {
            let mut ptr = get_langs();
            while !ptr.is_null() && !(*ptr).is_null() {
                let raw = CStr::from_ptr(*ptr).to_string_lossy();
                if let Some(norm) = normalize_bcp47_locale(&raw) {
                    locale = Some(norm);
                    break;
                }
                ptr = ptr.add(1);
            }
        }

        let mut timezone = None;
        if let (Some(tz_new), Some(tz_id), Some(tz_unref)) = (
            load_sym::<TzNewLocalFn>(glib, c"g_time_zone_new_local"),
            load_sym::<TzGetIdFn>(glib, c"g_time_zone_get_identifier"),
            load_sym::<TzUnrefFn>(glib, c"g_time_zone_unref"),
        ) {
            let tz = tz_new();
            if !tz.is_null() {
                let raw_ptr = tz_id(tz);
                if !raw_ptr.is_null() {
                    let tz_name = CStr::from_ptr(raw_ptr).to_string_lossy().into_owned();
                    if !tz_name.trim().is_empty() {
                        timezone = Some(tz_name);
                    }
                }
                tz_unref(tz);
            }
        }

        (locale, timezone)
    }
}

#[cfg(target_os = "macos")]
fn probe_os_locale_and_timezone() -> (Option<String>, Option<String>) {
    type CfCopyFn = unsafe extern "C" fn() -> *mut c_void;
    type CfGetStrFn = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
    type CfStringGetCStringFn = unsafe extern "C" fn(*mut c_void, *mut c_char, isize, u32) -> u8;
    type CfReleaseFn = unsafe extern "C" fn(*mut c_void);

    const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

    unsafe fn cf_string_to_rust(
        cf_str: *mut c_void,
        get_cstr: CfStringGetCStringFn,
    ) -> Option<String> {
        if cf_str.is_null() {
            return None;
        }
        let mut buf = [0_i8; 256];
        let ok = unsafe {
            get_cstr(
                cf_str,
                buf.as_mut_ptr(),
                isize::try_from(buf.len()).ok()?,
                K_CF_STRING_ENCODING_UTF8,
            )
        };
        if ok == 0 {
            return None;
        }
        let text = unsafe { CStr::from_ptr(buf.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        if text.trim().is_empty() {
            None
        } else {
            Some(text)
        }
    }

    unsafe {
        let Some(cf) =
            DynLib::open(c"/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation")
        else {
            return (None, None);
        };
        let Some(get_cstr) = cf.sym::<CfStringGetCStringFn>(c"CFStringGetCString") else {
            return (None, None);
        };
        let Some(cf_release) = cf.sym::<CfReleaseFn>(c"CFRelease") else {
            return (None, None);
        };

        let mut locale = None;
        if let (Some(copy_locale), Some(get_ident)) = (
            cf.sym::<CfCopyFn>(c"CFLocaleCopyCurrent"),
            cf.sym::<CfGetStrFn>(c"CFLocaleGetIdentifier"),
        ) {
            let loc = copy_locale();
            if !loc.is_null() {
                if let Some(raw) = cf_string_to_rust(get_ident(loc), get_cstr) {
                    locale = normalize_bcp47_locale(&raw);
                }
                cf_release(loc);
            }
        }

        let mut timezone = None;
        if let (Some(copy_tz), Some(get_tz_name)) = (
            cf.sym::<CfCopyFn>(c"CFTimeZoneCopySystem"),
            cf.sym::<CfGetStrFn>(c"CFTimeZoneGetName"),
        ) {
            let tz = copy_tz();
            if !tz.is_null() {
                timezone = cf_string_to_rust(get_tz_name(tz), get_cstr);
                cf_release(tz);
            }
        }

        (locale, timezone)
    }
}

#[cfg(target_os = "windows")]
fn probe_os_locale_and_timezone() -> (Option<String>, Option<String>) {
    type GetUserDefaultLocaleNameFn =
        unsafe extern "system" fn(*mut u16, std::ffi::c_int) -> std::ffi::c_int;

    let locale = unsafe {
        DynLib::open(c"kernel32.dll").and_then(|kernel| {
            let get_locale: GetUserDefaultLocaleNameFn = kernel.sym(c"GetUserDefaultLocaleName")?;
            let mut buf = [0_u16; 85];
            let len = get_locale(buf.as_mut_ptr(), std::ffi::c_int::try_from(buf.len()).ok()?);
            if len > 1 {
                let slice_len = usize::try_from(len - 1).ok()?;
                let raw = String::from_utf16_lossy(&buf[..slice_len]);
                normalize_bcp47_locale(&raw)
            } else {
                None
            }
        })
    };
    (locale, None)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn probe_os_locale_and_timezone() -> (Option<String>, Option<String>) {
    (None, None)
}

#[cfg(target_os = "linux")]
fn probe_os_network_monitor() -> Option<(bool, bool)> {
    type MonDefaultFn = unsafe extern "C" fn() -> *mut c_void;
    type MonBoolFn = unsafe extern "C" fn(*mut c_void) -> c_int;

    unsafe {
        let gio = open_lib(c"libgio-2.0.so.0")?;
        let get_default: MonDefaultFn = load_sym(gio, c"g_network_monitor_get_default")?;
        let get_avail: MonBoolFn = load_sym(gio, c"g_network_monitor_get_network_available")?;
        let get_metered: MonBoolFn = load_sym(gio, c"g_network_monitor_get_network_metered")?;

        let monitor = get_default();
        if monitor.is_null() {
            return None;
        }
        let online = get_avail(monitor) != 0;
        let metered = get_metered(monitor) != 0;
        Some((online, metered))
    }
}

#[cfg(target_os = "windows")]
fn probe_os_network_monitor() -> Option<(bool, bool)> {
    type InternetGetConnectedStateFn = unsafe extern "system" fn(*mut u32, u32) -> i32;

    unsafe {
        let wininet = DynLib::open(c"wininet.dll")?;
        let get_state: InternetGetConnectedStateFn = wininet.sym(c"InternetGetConnectedState")?;
        let mut flags: u32 = 0;
        let online = get_state(&raw mut flags, 0) != 0;
        Some((online, false))
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn probe_os_network_monitor() -> Option<(bool, bool)> {
    None
}

fn fnv1a_hash64(input: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

fn extract_image_extension(url: &str) -> &'static str {
    let clean = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
    for (ext, label) in [
        (".png", "png"),
        (".jpg", "jpg"),
        (".jpeg", "jpeg"),
        (".webp", "webp"),
        (".svg", "svg"),
        (".ico", "ico"),
    ] {
        if clean.ends_with(ext) {
            return label;
        }
    }
    "img"
}

/// Cross-platform OS provider for network status, system locale, and timezone probing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsSystemEnvBackend {
    /// Linux `GNetworkMonitor` (`libgio-2.0.so.0`) + `GLib` (`libglib-2.0.so.0`).
    LinuxGioGlib,
    /// Windows `WinRT` (`NetworkInformation` + `Windows.Globalization`).
    WindowsWinRt,
    /// macOS `Network.framework` (`NWPathMonitor`) + `Foundation` (`NSLocale`).
    MacOsFoundation,
}

impl OsSystemEnvBackend {
    pub const fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::WindowsWinRt
        } else if cfg!(target_os = "macos") {
            Self::MacOsFoundation
        } else {
            Self::LinuxGioGlib
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LinuxGioGlib => "linux-gio-glib",
            Self::WindowsWinRt => "windows-winrt",
            Self::MacOsFoundation => "macos-foundation",
        }
    }
}

/// Operating System environment probe (`GNetworkMonitor`, `GLib` locale/timezone,
/// XDG image asset cache resolution, and structured system error logging).
#[derive(Debug, Clone)]
pub struct NativeSystemEnv {
    backend: OsSystemEnvBackend,
    locale: String,
    timezone: String,
    online: bool,
    metered: bool,
    cache_dir: PathBuf,
    error_logs: Vec<String>,
}

impl Default for NativeSystemEnv {
    fn default() -> Self {
        Self::detect()
    }
}

impl NativeSystemEnv {
    pub fn detect() -> Self {
        Self::detect_for_app("ng-desktop-app")
    }

    pub fn detect_for_app(app_id: &str) -> Self {
        let trimmed = app_id.trim();
        let clean_app_id = if trimmed.is_empty() {
            "ng-desktop-app"
        } else {
            trimmed
        };

        let (os_locale, os_tz) = probe_os_locale_and_timezone();
        let locale = os_locale
            .or_else(|| {
                env::var("LC_ALL")
                    .ok()
                    .and_then(|v| normalize_bcp47_locale(&v))
            })
            .or_else(|| {
                env::var("LANG")
                    .ok()
                    .and_then(|v| normalize_bcp47_locale(&v))
            })
            .unwrap_or_else(|| String::from("en-US"));

        let timezone = os_tz
            .or_else(|| env::var("TZ").ok().filter(|v| !v.trim().is_empty()))
            .unwrap_or_else(|| String::from("UTC"));

        let (online, metered) = probe_os_network_monitor().unwrap_or((true, false));

        let cache_dir = env::var("XDG_CACHE_HOME")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .map_or_else(
                || env::temp_dir().join("ng-desktop-cache").join(clean_app_id),
                |xdg| PathBuf::from(xdg).join(clean_app_id),
            );

        Self {
            backend: OsSystemEnvBackend::current(),
            locale,
            timezone,
            online,
            metered,
            cache_dir,
            error_logs: Vec::new(),
        }
    }

    pub const fn backend(&self) -> OsSystemEnvBackend {
        self.backend
    }

    pub fn refresh_network(&mut self) {
        if let Some((online, metered)) = probe_os_network_monitor() {
            self.online = online;
            self.metered = metered;
        }
    }

    pub const fn set_network_status(&mut self, online: bool, metered: bool) {
        self.online = online;
        self.metered = metered;
    }

    pub const fn is_online(&self) -> bool {
        self.online
    }

    pub const fn is_metered(&self) -> bool {
        self.metered
    }

    pub const fn connectivity(&self) -> &'static str {
        if !self.online {
            "offline"
        } else if self.metered {
            "limited"
        } else {
            "full"
        }
    }

    pub fn locale(&self) -> &str {
        &self.locale
    }

    pub fn timezone(&self) -> &str {
        &self.timezone
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn log_error(&mut self, message: &str) {
        self.error_logs.push(String::from(message));
    }

    pub fn error_logs(&self) -> &[String] {
        &self.error_logs
    }

    /// Maps remote image URLs onto deterministic local OS cache paths under `cache_dir/images`.
    pub fn resolve_image_asset(&self, src: &str, width: Option<u32>) -> String {
        let trimmed = src.trim();
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            let hash = fnv1a_hash64(trimmed);
            let ext = extract_image_extension(trimmed);
            let width_suffix = width.map_or_else(String::new, |w| format!("?w={w}"));
            let images_path = self
                .cache_dir
                .join("images")
                .join(format!("img_{hash:016x}.{ext}"));
            return format!("{}{width_suffix}", images_path.display());
        }

        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{NativeSystemEnv, normalize_bcp47_locale};

    #[test]
    fn normalizes_locales_and_resolves_cached_image_assets() {
        assert_eq!(
            normalize_bcp47_locale("es_ES.UTF-8").as_deref(),
            Some("es-ES")
        );
        assert_eq!(normalize_bcp47_locale("C.UTF-8"), None);

        let mut env = NativeSystemEnv::detect();
        assert_ne!(env.locale(), "");
        assert_ne!(env.timezone(), "");

        let app_env = NativeSystemEnv::detect_for_app("acme-studio");
        assert!(app_env.cache_dir().ends_with("acme-studio"));

        let resolved_png =
            env.resolve_image_asset("https://cdn.example.com/assets/avatar.png", Some(180));
        let normalized_png = resolved_png.replace('\\', "/");
        assert!(normalized_png.contains("/images/img_"));
        assert!(normalized_png.ends_with(".png?w=180"));

        let resolved_generic =
            env.resolve_image_asset("https://cdn.example.com/hero.webp", Some(320));
        let normalized_generic = resolved_generic.replace('\\', "/");
        assert!(normalized_generic.contains("/images/img_"));
        assert!(normalized_generic.ends_with(".webp?w=320"));

        env.set_network_status(false, true);
        assert_eq!(env.connectivity(), "offline");
        env.set_network_status(true, true);
        assert_eq!(env.connectivity(), "limited");

        env.log_error("Disk write warning");
        assert_eq!(env.error_logs(), ["Disk write warning"]);
    }
}

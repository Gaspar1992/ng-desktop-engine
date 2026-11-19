#![allow(unsafe_code)]

#[cfg(target_os = "linux")]
use crate::ffi_dl::DynLib;
use std::env;
#[cfg(target_os = "linux")]
use std::ffi::{CStr, c_char, c_int, c_void};
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
fn probe_glib_locale_and_timezone() -> (Option<String>, Option<String>) {
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

#[cfg(not(target_os = "linux"))]
fn probe_glib_locale_and_timezone() -> (Option<String>, Option<String>) {
    (None, None)
}

#[cfg(target_os = "linux")]
fn probe_gio_network_monitor() -> Option<(bool, bool)> {
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

#[cfg(not(target_os = "linux"))]
fn probe_gio_network_monitor() -> Option<(bool, bool)> {
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
        let (glib_locale, glib_tz) = probe_glib_locale_and_timezone();
        let locale = glib_locale
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

        let timezone = glib_tz
            .or_else(|| env::var("TZ").ok().filter(|v| !v.trim().is_empty()))
            .unwrap_or_else(|| String::from("UTC"));

        let (online, metered) = probe_gio_network_monitor().unwrap_or((true, false));

        let cache_dir = env::var("XDG_CACHE_HOME")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .map_or_else(
                || PathBuf::from("/tmp/ng_native_cache"),
                |xdg| PathBuf::from(xdg).join("ng-native"),
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
        if let Some((online, metered)) = probe_gio_network_monitor() {
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

    /// Maps remote image URLs onto deterministic local OS cache paths (including `PokéAPI` artwork
    /// under `/tmp/ng_native_pokedex/{id}.png` and general URLs under `cache_dir`).
    pub fn resolve_image_asset(&self, src: &str, width: Option<u32>) -> String {
        let trimmed = src.trim();
        if let Some((_, tail)) = trimmed.split_once("/official-artwork/") {
            let file_part = tail.split('?').next().unwrap_or(tail);
            if let Some(stem) = file_part.strip_suffix(".png")
                && !stem.is_empty()
                && stem.bytes().all(|b| b.is_ascii_digit())
            {
                return format!("/tmp/ng_native_pokedex/{stem}.png");
            }
        }

        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            let hash = fnv1a_hash64(trimmed);
            let ext = extract_image_extension(trimmed);
            let width_suffix = width.map_or_else(String::new, |w| format!("?w={w}"));
            return format!(
                "{}/img_{hash:016x}.{ext}{width_suffix}",
                self.cache_dir.display()
            );
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
        assert_eq!(
            env.resolve_image_asset(
                "https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/official-artwork/25.png",
                Some(180)
            ),
            "/tmp/ng_native_pokedex/25.png"
        );

        let resolved_generic =
            env.resolve_image_asset("https://cdn.example.com/hero.webp", Some(320));
        assert!(resolved_generic.ends_with(".webp?w=320"));
        assert!(resolved_generic.contains("img_"));

        env.set_network_status(false, true);
        assert_eq!(env.connectivity(), "offline");
        env.set_network_status(true, true);
        assert_eq!(env.connectivity(), "limited");

        env.log_error("Disk write warning");
        assert_eq!(env.error_logs(), ["Disk write warning"]);
    }
}

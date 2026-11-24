#![allow(unsafe_code)]

use super::DesktopTheme;
use crate::ffi_dl::DynLib;
use std::env;

/// Cross-platform OS desktop portal & native service driver identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsDesktopPortalBackend {
    /// Linux Freedesktop Portal / `GLib` / `GTK4` (`libgio-2.0.so.0` / `libgtk-4.so.1`).
    LinuxFreedesktopGtk,
    /// macOS Cocoa Portal (`AppKit` `NSPasteboard`, `NSAlert`, `NSOpenPanel`).
    MacOsCocoaPortal,
    /// Windows Win32 Portal (`user32.dll` clipboard/dialogs, `comdlg32.dll`, `advapi32.dll`).
    WindowsWin32Portal,
}

impl OsDesktopPortalBackend {
    #[must_use]
    pub const fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::WindowsWin32Portal
        } else if cfg!(target_os = "macos") {
            Self::MacOsCocoaPortal
        } else {
            Self::LinuxFreedesktopGtk
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LinuxFreedesktopGtk => "linux-freedesktop-gtk4",
            Self::MacOsCocoaPortal => "macos-cocoa-portal",
            Self::WindowsWin32Portal => "windows-win32-portal",
        }
    }
}

/// Probes dynamically loaded OS desktop portal frameworks without hard-linking them.
#[must_use]
pub fn probe_os_portal_driver() -> bool {
    #[cfg(target_os = "linux")]
    unsafe {
        DynLib::open(c"libgio-2.0.so.0").is_some() || DynLib::open(c"libgtk-4.so.1").is_some()
    }
    #[cfg(target_os = "macos")]
    unsafe {
        DynLib::open(c"/System/Library/Frameworks/AppKit.framework/AppKit").is_some()
    }
    #[cfg(target_os = "windows")]
    unsafe {
        type OpenClipboardFn = unsafe extern "system" fn(*mut std::ffi::c_void) -> i32;
        type GetOpenFileNameWFn = unsafe extern "system" fn(*mut std::ffi::c_void) -> i32;
        let user32_ok = DynLib::open(c"user32.dll")
            .and_then(|d| d.sym::<OpenClipboardFn>(c"OpenClipboard"))
            .is_some();
        let comdlg_ok = DynLib::open(c"comdlg32.dll")
            .and_then(|d| d.sym::<GetOpenFileNameWFn>(c"GetOpenFileNameW"))
            .is_some();
        user32_ok && comdlg_ok
    }
}

#[cfg(target_os = "windows")]
fn probe_windows_apps_use_light_theme() -> Option<DesktopTheme> {
    use std::ffi::c_void;

    type RegGetValueWFn = unsafe extern "system" fn(
        *mut c_void,
        *const u16,
        *const u16,
        u32,
        *mut u32,
        *mut c_void,
        *mut u32,
    ) -> i32;

    // Sign-extended 32-bit LONG handle per Windows SDK winreg.h: ((HKEY)(ULONG_PTR)((LONG)0x80000001)).
    const HKEY_CURRENT_USER: isize = -0x7fff_ffff;
    const RRF_RT_REG_DWORD: u32 = 0x0000_0010;

    fn encode_wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    unsafe {
        let advapi = DynLib::open(c"advapi32.dll")?;
        let reg_get_value: RegGetValueWFn = advapi.sym(c"RegGetValueW")?;
        let subkey = encode_wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
        let value_name = encode_wide("AppsUseLightTheme");
        let mut data: u32 = 0;
        let mut size: u32 = u32::try_from(size_of::<u32>()).ok()?;
        let status = reg_get_value(
            HKEY_CURRENT_USER as *mut c_void,
            subkey.as_ptr(),
            value_name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&raw mut data).cast::<c_void>(),
            &raw mut size,
        );
        if status == 0 {
            Some(if data != 0 {
                DesktopTheme::Light
            } else {
                DesktopTheme::Dark
            })
        } else {
            None
        }
    }
}

/// Detects the active desktop color scheme across Linux (`GTK_THEME`), Windows Registry
/// (`AppsUseLightTheme`), and macOS (`APPLE_INTERFACE_STYLE`), honoring `NG_DESKTOP_THEME`.
#[must_use]
pub fn detect_os_theme() -> DesktopTheme {
    if let Ok(val) = env::var("NG_DESKTOP_THEME")
        && !val.trim().is_empty()
    {
        return if val.to_ascii_lowercase().contains("light") {
            DesktopTheme::Light
        } else {
            DesktopTheme::Dark
        };
    }

    #[cfg(target_os = "windows")]
    if let Some(theme) = probe_windows_apps_use_light_theme() {
        return theme;
    }

    #[cfg(target_os = "macos")]
    if let Ok(style) = env::var("APPLE_INTERFACE_STYLE") {
        return if style.to_ascii_lowercase().contains("light") {
            DesktopTheme::Light
        } else {
            DesktopTheme::Dark
        };
    }

    env::var("GTK_THEME").map_or(DesktopTheme::Dark, |val| {
        if val.to_ascii_lowercase().contains("light") {
            DesktopTheme::Light
        } else {
            DesktopTheme::Dark
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_current_os_portal_backend_and_theme() {
        let current = OsDesktopPortalBackend::current();
        assert_ne!(current.as_str(), "");
        let _theme = detect_os_theme();
        if cfg!(target_os = "linux") || cfg!(target_os = "macos") || cfg!(target_os = "windows") {
            assert!(probe_os_portal_driver());
        }
    }
}

#![allow(unsafe_code)]

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Cross-platform OS kernel file-watcher backend identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsFileWatcherBackend {
    /// Linux `inotify` (`inotify_init1` / `inotify_add_watch` via `libc.so.6`).
    LinuxInotify,
    /// Windows `ReadDirectoryChangesW` (`kernel32.dll`).
    WindowsReadDirectoryChanges,
    /// macOS `FSEvents` / `kqueue`.
    MacOsFsEvents,
}

impl OsFileWatcherBackend {
    pub const fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::WindowsReadDirectoryChanges
        } else if cfg!(target_os = "macos") {
            Self::MacOsFsEvents
        } else {
            Self::LinuxInotify
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LinuxInotify => "linux-inotify",
            Self::WindowsReadDirectoryChanges => "windows-read-directory-changes",
            Self::MacOsFsEvents => "macos-fsevents",
        }
    }
}

/// Standard XDG Base Directory specification category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XdgDirKind {
    Config,
    Data,
    Cache,
}

impl XdgDirKind {
    pub fn from_str_lossy(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "data" => Self::Data,
            "cache" => Self::Cache,
            _ => Self::Config,
        }
    }
}

#[cfg(target_os = "linux")]
mod inotify_ffi {
    use std::ffi::{CString, c_char, c_int, c_void};
    use std::path::Path;

    const RTLD_LAZY: c_int = 1;
    const IN_NONBLOCK: c_int = 0o4000;
    const IN_CLOEXEC: c_int = 0o200_0000;
    const IN_MODIFY: u32 = 0x0000_0002;
    const IN_ATTRIB: u32 = 0x0000_0004;
    const IN_CLOSE_WRITE: u32 = 0x0000_0008;

    unsafe extern "C" {
        fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    }

    type InotifyInit1Fn = unsafe extern "C" fn(c_int) -> c_int;
    type InotifyAddWatchFn = unsafe extern "C" fn(c_int, *const c_char, u32) -> c_int;
    type ReadFn = unsafe extern "C" fn(c_int, *mut c_void, usize) -> isize;
    type CloseFn = unsafe extern "C" fn(c_int) -> c_int;

    pub(super) struct InotifyHandle {
        fd: c_int,
        inotify_add_watch: InotifyAddWatchFn,
        read_fn: ReadFn,
        close_fn: CloseFn,
    }

    impl InotifyHandle {
        pub(super) fn try_new() -> Option<Self> {
            unsafe {
                let handle = dlopen(c"libc.so.6".as_ptr(), RTLD_LAZY);
                if handle.is_null() {
                    return None;
                }
                let init_sym = dlsym(handle, c"inotify_init1".as_ptr());
                let add_sym = dlsym(handle, c"inotify_add_watch".as_ptr());
                let read_sym = dlsym(handle, c"read".as_ptr());
                let close_sym = dlsym(handle, c"close".as_ptr());
                if init_sym.is_null()
                    || add_sym.is_null()
                    || read_sym.is_null()
                    || close_sym.is_null()
                {
                    return None;
                }
                let init_fn: InotifyInit1Fn =
                    std::mem::transmute::<*mut c_void, InotifyInit1Fn>(init_sym);
                let fd = init_fn(IN_NONBLOCK | IN_CLOEXEC);
                if fd < 0 {
                    return None;
                }
                Some(Self {
                    fd,
                    inotify_add_watch: std::mem::transmute::<*mut c_void, InotifyAddWatchFn>(
                        add_sym,
                    ),
                    read_fn: std::mem::transmute::<*mut c_void, ReadFn>(read_sym),
                    close_fn: std::mem::transmute::<*mut c_void, CloseFn>(close_sym),
                })
            }
        }

        pub(super) fn add_watch(&self, path: &Path) -> Option<c_int> {
            let c_path = CString::new(path.to_string_lossy().as_bytes()).ok()?;
            let wd = unsafe {
                (self.inotify_add_watch)(
                    self.fd,
                    c_path.as_ptr(),
                    IN_MODIFY | IN_CLOSE_WRITE | IN_ATTRIB,
                )
            };
            if wd >= 0 { Some(wd) } else { None }
        }

        pub(super) fn drain_watch_descriptors(&self) -> Vec<c_int> {
            let mut buf = [0_u8; 1024];
            let mut wds = Vec::new();
            loop {
                let n = unsafe { (self.read_fn)(self.fd, buf.as_mut_ptr().cast(), buf.len()) };
                if n <= 0 {
                    break;
                }
                let bytes_read = usize::try_from(n).unwrap_or(0);
                let mut offset = 0_usize;
                while offset + 16 <= bytes_read {
                    let wd = c_int::from_ne_bytes([
                        buf[offset],
                        buf[offset + 1],
                        buf[offset + 2],
                        buf[offset + 3],
                    ]);
                    let name_len = u32::from_ne_bytes([
                        buf[offset + 12],
                        buf[offset + 13],
                        buf[offset + 14],
                        buf[offset + 15],
                    ]) as usize;
                    wds.push(wd);
                    offset = offset.saturating_add(16 + name_len);
                }
            }
            wds
        }
    }

    impl Drop for InotifyHandle {
        fn drop(&mut self) {
            if self.fd >= 0 {
                unsafe {
                    let _ = (self.close_fn)(self.fd);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct WatchedFileState {
    last_mtime: Option<SystemTime>,
    last_len: u64,
    pending_atomic_change: bool,
}

/// Native XDG File System Manager & Non-blocking Kernel File Watcher.
pub struct NativeFsManager {
    app_id: String,
    backend: OsFileWatcherBackend,
    watched_files: HashMap<PathBuf, WatchedFileState>,
    #[cfg(target_os = "linux")]
    inotify: Option<inotify_ffi::InotifyHandle>,
    #[cfg(target_os = "linux")]
    wd_to_path: HashMap<std::ffi::c_int, PathBuf>,
}

impl Default for NativeFsManager {
    fn default() -> Self {
        Self::new("ng-native-app")
    }
}

impl NativeFsManager {
    pub fn new(app_id: &str) -> Self {
        Self {
            app_id: String::from(app_id),
            backend: OsFileWatcherBackend::current(),
            watched_files: HashMap::new(),
            #[cfg(target_os = "linux")]
            inotify: inotify_ffi::InotifyHandle::try_new(),
            #[cfg(target_os = "linux")]
            wd_to_path: HashMap::new(),
        }
    }

    pub const fn backend(&self) -> OsFileWatcherBackend {
        self.backend
    }

    pub fn is_kernel_watcher_active(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            self.inotify.is_some()
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    fn base_xdg_dir(&self, kind: XdgDirKind) -> PathBuf {
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .unwrap_or_else(|_| String::from("/tmp"));
        let root = match kind {
            XdgDirKind::Config => {
                env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{home}/.config"))
            }
            XdgDirKind::Data => {
                env::var("XDG_DATA_HOME").unwrap_or_else(|_| format!("{home}/.local/share"))
            }
            XdgDirKind::Cache => {
                env::var("XDG_CACHE_HOME").unwrap_or_else(|_| format!("{home}/.cache"))
            }
        };
        PathBuf::from(root).join(&self.app_id)
    }

    /// Resolves a relative path inside an XDG directory while rejecting path traversal (`..`).
    pub fn resolve_xdg_path(&self, kind: XdgDirKind, rel_path: &str) -> Result<PathBuf, String> {
        let trimmed = rel_path.trim().trim_start_matches(['/', '\\']);
        if trimmed.is_empty() {
            return Err(String::from("Relative path cannot be empty"));
        }
        let candidate = Path::new(trimmed);
        for comp in candidate.components() {
            if matches!(
                comp,
                std::path::Component::ParentDir | std::path::Component::RootDir
            ) {
                return Err(format!(
                    "Path traversal rejected in XDG relative path: '{rel_path}'"
                ));
            }
        }
        Ok(self.base_xdg_dir(kind).join(candidate))
    }

    pub fn read_text(&self, path: &Path) -> Result<String, String> {
        fs::read_to_string(path).map_err(|e| format!("Failed to read '{}': {e}", path.display()))
    }

    /// Atomically writes `content` to `path` via a sibling `.tmp.<pid>` file + rename.
    pub fn write_text_atomic(&mut self, path: &Path, content: &str) -> Result<(), String> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create dir '{}': {e}", parent.display()))?;
        }
        let pid = std::process::id();
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("atomic");
        let tmp_path = path.with_file_name(format!(".{file_name}.tmp.{pid}"));
        fs::write(&tmp_path, content)
            .map_err(|e| format!("Failed to write temp file '{}': {e}", tmp_path.display()))?;
        fs::rename(&tmp_path, path)
            .map_err(|e| format!("Failed to rename temp file to '{}': {e}", path.display()))?;

        if let Some(state) = self.watched_files.get_mut(path) {
            state.pending_atomic_change = true;
            #[cfg(target_os = "linux")]
            if let Some(inotify) = &self.inotify
                && let Some(wd) = inotify.add_watch(path)
            {
                self.wd_to_path.insert(wd, path.to_path_buf());
            }
        }
        Ok(())
    }

    fn snapshot_metadata(path: &Path) -> (Option<SystemTime>, u64) {
        fs::metadata(path).map_or((None, 0), |m| (m.modified().ok(), m.len()))
    }

    /// Registers `path` with the OS file watcher (`inotify` on Linux + metadata snapshot).
    pub fn watch_file(&mut self, path: &Path) -> Result<(), String> {
        let (last_mtime, last_len) = Self::snapshot_metadata(path);
        self.watched_files.insert(
            path.to_path_buf(),
            WatchedFileState {
                last_mtime,
                last_len,
                pending_atomic_change: false,
            },
        );
        #[cfg(target_os = "linux")]
        if let Some(inotify) = &self.inotify
            && let Some(wd) = inotify.add_watch(path)
        {
            self.wd_to_path.insert(wd, path.to_path_buf());
        }
        Ok(())
    }

    /// Non-blockingly drains kernel `inotify` events and metadata deltas for watched files.
    pub fn poll_changed_paths(&mut self) -> Vec<PathBuf> {
        let mut changed = Vec::new();
        #[cfg(target_os = "linux")]
        if let Some(inotify) = &self.inotify {
            for wd in inotify.drain_watch_descriptors() {
                if let Some(path) = self.wd_to_path.get(&wd)
                    && !changed.contains(path)
                {
                    changed.push(path.clone());
                }
            }
        }

        for (path, state) in &mut self.watched_files {
            let (mtime, len) = Self::snapshot_metadata(path);
            let metadata_changed =
                (mtime.is_some() && mtime != state.last_mtime) || len != state.last_len;
            if state.pending_atomic_change || metadata_changed {
                state.last_mtime = mtime;
                state.last_len = len;
                state.pending_atomic_change = false;
                if !changed.contains(path) {
                    changed.push(path.clone());
                }
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_xdg_paths_rejects_traversal_and_watches_atomic_writes() {
        let mut fs_mgr = NativeFsManager::new("ng-native-test");
        assert_eq!(fs_mgr.backend(), OsFileWatcherBackend::current());
        assert_ne!(fs_mgr.backend().as_str(), "");

        let cfg_path = fs_mgr
            .resolve_xdg_path(XdgDirKind::Config, "preferences/settings.json")
            .unwrap();
        assert!(
            cfg_path
                .to_string_lossy()
                .contains("ng-native-test/preferences/settings.json")
        );

        let traversal = fs_mgr.resolve_xdg_path(XdgDirKind::Config, "../etc/passwd");
        assert!(traversal.is_err());

        let tmp_file =
            env::temp_dir().join(format!("ng_native_fs_test_{}.json", std::process::id()));
        fs_mgr
            .write_text_atomic(&tmp_file, r#"{"favorites":[25]}"#)
            .unwrap();
        assert_eq!(
            fs_mgr.read_text(&tmp_file).unwrap(),
            r#"{"favorites":[25]}"#
        );

        fs_mgr.watch_file(&tmp_file).unwrap();
        assert_eq!(fs_mgr.poll_changed_paths().len(), 0);

        fs_mgr
            .write_text_atomic(&tmp_file, r#"{"favorites":[25,133]}"#)
            .unwrap();
        let changed = fs_mgr.poll_changed_paths();
        assert_eq!(changed, vec![tmp_file.clone()]);

        let _ = fs::remove_file(&tmp_file);
    }
}

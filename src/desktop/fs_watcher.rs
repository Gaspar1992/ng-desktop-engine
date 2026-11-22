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
    use crate::ffi_dl::DynLib;
    use std::ffi::{CString, c_char, c_int, c_void};
    use std::path::Path;

    const IN_NONBLOCK: c_int = 0o4000;
    const IN_CLOEXEC: c_int = 0o200_0000;
    pub(super) const IN_MODIFY: u32 = 0x0000_0002;
    pub(super) const IN_ATTRIB: u32 = 0x0000_0004;
    pub(super) const IN_CLOSE_WRITE: u32 = 0x0000_0008;
    pub(super) const IN_MOVED_TO: u32 = 0x0000_0080;
    pub(super) const IN_CREATE: u32 = 0x0000_0100;
    pub(super) const IN_DELETE_SELF: u32 = 0x0000_0400;
    pub(super) const IN_MOVE_SELF: u32 = 0x0000_0800;
    pub(super) const IN_IGNORED: u32 = 0x0000_8000;

    type InotifyInit1Fn = unsafe extern "C" fn(c_int) -> c_int;
    type InotifyAddWatchFn = unsafe extern "C" fn(c_int, *const c_char, u32) -> c_int;
    type ReadFn = unsafe extern "C" fn(c_int, *mut c_void, usize) -> isize;
    type CloseFn = unsafe extern "C" fn(c_int) -> c_int;

    #[derive(Debug, Clone)]
    pub(super) struct InotifyRawEvent {
        pub(super) wd: c_int,
        pub(super) mask: u32,
        pub(super) name: Option<String>,
    }

    pub(super) struct InotifyHandle {
        fd: c_int,
        inotify_add_watch: InotifyAddWatchFn,
        read_fn: ReadFn,
        close_fn: CloseFn,
    }

    impl InotifyHandle {
        pub(super) fn try_new() -> Option<Self> {
            unsafe {
                let lib = DynLib::open(c"libc.so.6")?;
                let init_fn: InotifyInit1Fn = lib.sym(c"inotify_init1")?;
                let inotify_add_watch: InotifyAddWatchFn = lib.sym(c"inotify_add_watch")?;
                let read_fn: ReadFn = lib.sym(c"read")?;
                let close_fn: CloseFn = lib.sym(c"close")?;
                let fd = init_fn(IN_NONBLOCK | IN_CLOEXEC);
                if fd < 0 {
                    return None;
                }
                Some(Self {
                    fd,
                    inotify_add_watch,
                    read_fn,
                    close_fn,
                })
            }
        }

        pub(super) fn add_file_watch(&self, path: &Path) -> Option<c_int> {
            self.add_watch_with_mask(
                path,
                IN_MODIFY | IN_CLOSE_WRITE | IN_ATTRIB | IN_DELETE_SELF | IN_MOVE_SELF,
            )
        }

        pub(super) fn add_dir_watch(&self, dir: &Path) -> Option<c_int> {
            self.add_watch_with_mask(dir, IN_MOVED_TO | IN_CREATE | IN_CLOSE_WRITE | IN_MODIFY)
        }

        fn add_watch_with_mask(&self, path: &Path, mask: u32) -> Option<c_int> {
            let c_path = CString::new(path.to_string_lossy().as_bytes()).ok()?;
            let wd = unsafe { (self.inotify_add_watch)(self.fd, c_path.as_ptr(), mask) };
            if wd >= 0 { Some(wd) } else { None }
        }

        pub(super) fn drain_events(&self) -> Vec<InotifyRawEvent> {
            let mut buf = [0_u8; 4096];
            let mut events = Vec::new();
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
                    let mask = u32::from_ne_bytes([
                        buf[offset + 4],
                        buf[offset + 5],
                        buf[offset + 6],
                        buf[offset + 7],
                    ]);
                    let name_len = u32::from_ne_bytes([
                        buf[offset + 12],
                        buf[offset + 13],
                        buf[offset + 14],
                        buf[offset + 15],
                    ]) as usize;
                    let name_end = offset.saturating_add(16).saturating_add(name_len);
                    let name = if name_len > 0 && name_end <= bytes_read {
                        let raw_name = &buf[offset + 16..name_end];
                        let nul_pos = raw_name
                            .iter()
                            .position(|&b| b == 0)
                            .unwrap_or(raw_name.len());
                        let parsed = String::from_utf8_lossy(&raw_name[..nul_pos]).into_owned();
                        if parsed.is_empty() {
                            None
                        } else {
                            Some(parsed)
                        }
                    } else {
                        None
                    };
                    events.push(InotifyRawEvent { wd, mask, name });
                    offset = name_end;
                }
            }
            events
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
    #[cfg(target_os = "linux")]
    dir_wd_to_path: HashMap<std::ffi::c_int, PathBuf>,
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
            #[cfg(target_os = "linux")]
            dir_wd_to_path: HashMap::new(),
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
            self.register_linux_watches(path);
        }
        Ok(())
    }

    fn snapshot_metadata(path: &Path) -> (Option<SystemTime>, u64) {
        fs::metadata(path).map_or((None, 0), |m| (m.modified().ok(), m.len()))
    }

    #[cfg(target_os = "linux")]
    fn register_linux_watches(&mut self, path: &Path) {
        let Some(inotify) = &self.inotify else {
            return;
        };
        if let Some(wd) = inotify.add_file_watch(path) {
            self.wd_to_path.insert(wd, path.to_path_buf());
        }
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
            && !self.dir_wd_to_path.values().any(|dir| dir == parent)
            && let Some(dir_wd) = inotify.add_dir_watch(parent)
        {
            self.dir_wd_to_path.insert(dir_wd, parent.to_path_buf());
        }
    }

    #[cfg(target_os = "linux")]
    fn poll_linux_inotify(&mut self, changed: &mut Vec<PathBuf>) {
        use inotify_ffi::{IN_CREATE, IN_DELETE_SELF, IN_IGNORED, IN_MOVE_SELF, IN_MOVED_TO};

        let Some(inotify) = &self.inotify else {
            return;
        };
        let events = inotify.drain_events();
        let mut rearm_paths = Vec::new();
        let rearm_flags = IN_DELETE_SELF | IN_MOVE_SELF | IN_IGNORED | IN_MOVED_TO | IN_CREATE;

        for ev in events {
            let needs_rearm = (ev.mask & rearm_flags) != 0;

            if let Some(path) = self.wd_to_path.get(&ev.wd).cloned() {
                if !changed.contains(&path) {
                    changed.push(path.clone());
                }
                if (ev.mask & IN_IGNORED) != 0 {
                    self.wd_to_path.remove(&ev.wd);
                }
                if needs_rearm && !rearm_paths.contains(&path) {
                    rearm_paths.push(path);
                }
                continue;
            }

            if let Some(dir) = self.dir_wd_to_path.get(&ev.wd)
                && let Some(ref file_name) = ev.name
            {
                let candidate = dir.join(file_name);
                if self.watched_files.contains_key(&candidate) {
                    if !changed.contains(&candidate) {
                        changed.push(candidate.clone());
                    }
                    if needs_rearm && !rearm_paths.contains(&candidate) {
                        rearm_paths.push(candidate);
                    }
                }
            }
        }

        for path in rearm_paths {
            self.register_linux_watches(&path);
        }
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
        self.register_linux_watches(path);
        Ok(())
    }

    /// Non-blockingly drains kernel `inotify` events when active, or polls file metadata deltas
    /// on platforms without an active kernel watcher.
    pub fn poll_changed_paths(&mut self) -> Vec<PathBuf> {
        let mut changed = Vec::new();
        for (path, state) in &mut self.watched_files {
            if state.pending_atomic_change {
                state.pending_atomic_change = false;
                changed.push(path.clone());
            }
        }

        #[cfg(target_os = "linux")]
        if self.is_kernel_watcher_active() {
            self.poll_linux_inotify(&mut changed);
            for path in &changed {
                if let Some(state) = self.watched_files.get_mut(path) {
                    let (mtime, len) = Self::snapshot_metadata(path);
                    state.last_mtime = mtime;
                    state.last_len = len;
                }
            }
            return changed;
        }

        for (path, state) in &mut self.watched_files {
            let (mtime, len) = Self::snapshot_metadata(path);
            let metadata_changed =
                (mtime.is_some() && mtime != state.last_mtime) || len != state.last_len;
            if metadata_changed {
                state.last_mtime = mtime;
                state.last_len = len;
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
                .replace('\\', "/")
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

    #[test]
    fn detects_external_atomic_rename_and_direct_write_via_watcher() {
        let watch_dir =
            env::temp_dir().join(format!("ng_fs_external_watch_{}", std::process::id()));
        fs::create_dir_all(&watch_dir).unwrap();
        let target = watch_dir.join("state.json");
        let staging = watch_dir.join("state.json.staging");
        fs::write(&target, r#"{"step":"v1"}"#).unwrap();

        let mut fs_mgr = NativeFsManager::new("ng-native-external-watch");
        fs_mgr.watch_file(&target).unwrap();
        assert_eq!(fs_mgr.poll_changed_paths(), Vec::<PathBuf>::new());

        // External atomic swap (staging write + rename over watched target)
        fs::write(&staging, r#"{"step":"v2-renamed-payload"}"#).unwrap();
        fs::rename(&staging, &target).unwrap();
        assert_eq!(fs_mgr.poll_changed_paths(), vec![target.clone()]);

        // Re-armed file watch must still catch direct writes on the new inode
        fs::write(&target, r#"{"step":"v3-direct-write-after-rename"}"#).unwrap();
        assert_eq!(fs_mgr.poll_changed_paths(), vec![target.clone()]);
        assert_eq!(fs_mgr.poll_changed_paths(), Vec::<PathBuf>::new());

        let _ = fs::remove_dir_all(&watch_dir);
    }
}

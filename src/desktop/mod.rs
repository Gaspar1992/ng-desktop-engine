use crate::config::AppPermissions;
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub mod compute_pool;
pub mod fs_watcher;
mod http_codec;
pub mod http_native;
mod libsoup_sys;
pub mod pokedex;
pub mod pokedex_ui;
pub mod sqlite_sys;
pub mod system_env;

pub use compute_pool::{ComputeMatch, NativeComputePool};
pub use fs_watcher::{NativeFsManager, OsFileWatcherBackend, XdgDirKind};
pub use http_native::{
    NativeHttpCallKind, NativeHttpConfig, NativeHttpEngine, NativeHttpRequest, NativeHttpResponse,
    OsHttpPlatform,
};
pub use pokedex::{PokemonIndexEntry, PokemonInfo};
pub use pokedex_ui::PokedexController;
pub use sqlite_sys::{NativeSqliteDatabase, OsSqliteBackend};
pub use system_env::{NativeSystemEnv, OsSystemEnvBackend};

/// Native system color scheme synchronized with GTK4 (`GtkSettings`) / Libadwaita (`AdwStyleManager`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopTheme {
    Dark,
    Light,
}

impl DesktopTheme {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    pub fn from_str_lossy(raw: &str) -> Self {
        if raw.trim().eq_ignore_ascii_case("light") {
            Self::Light
        } else {
            Self::Dark
        }
    }
}

/// Native desktop notification record (`GNotification` / Freedesktop Notification spec).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopNotification {
    pub title: String,
    pub body: String,
    pub icon: Option<String>,
}

/// Native file dialog operation (`GtkFileDialog` open / save).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileDialogKind {
    Open,
    Save,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDialogRequest {
    pub kind: FileDialogKind,
    pub title: String,
    pub suggested_path: String,
}

/// Cross-platform Desktop OS & Native HTTP service hub for Phase 5 (`@ng-desktop/desktop` & `@ng-desktop/http`).
pub struct DesktopServices {
    app_id: String,
    permissions: AppPermissions,
    theme: DesktopTheme,
    clipboard: String,
    notifications: Vec<DesktopNotification>,
    file_dialogs: Vec<FileDialogRequest>,
    http_engine: Arc<Mutex<NativeHttpEngine>>,
    sqlite: NativeSqliteDatabase,
    system_env: NativeSystemEnv,
    compute: NativeComputePool,
    fs_manager: NativeFsManager,
}

impl Default for DesktopServices {
    fn default() -> Self {
        Self::new()
    }
}

impl DesktopServices {
    pub fn new() -> Self {
        Self::with_app_config("ng-desktop-app", AppPermissions::allow_all_sandboxed())
    }

    /// Creates a `DesktopServices` instance with an explicit application identifier and capability manifest.
    pub fn with_app_config(app_id: &str, permissions: AppPermissions) -> Self {
        Self::with_app_config_and_http(
            app_id,
            permissions,
            Arc::new(Mutex::new(NativeHttpEngine::new())),
        )
    }

    /// Creates a `DesktopServices` instance sharing an existing warm `NativeHttpEngine` pool.
    pub fn with_http_engine(http_engine: Arc<Mutex<NativeHttpEngine>>) -> Self {
        Self::with_app_config_and_http(
            "ng-desktop-app",
            AppPermissions::allow_all_sandboxed(),
            http_engine,
        )
    }

    /// Creates a `DesktopServices` instance parameterized by app id, sandbox permissions, and HTTP pool.
    pub fn with_app_config_and_http(
        app_id: &str,
        permissions: AppPermissions,
        http_engine: Arc<Mutex<NativeHttpEngine>>,
    ) -> Self {
        let detected_theme = env::var("GTK_THEME").map_or(DesktopTheme::Dark, |val| {
            if val.to_ascii_lowercase().contains("light") {
                DesktopTheme::Light
            } else {
                DesktopTheme::Dark
            }
        });
        Self {
            app_id: String::from(app_id),
            permissions,
            theme: detected_theme,
            clipboard: String::new(),
            notifications: Vec::new(),
            file_dialogs: Vec::new(),
            http_engine,
            sqlite: NativeSqliteDatabase::new_in_memory(),
            system_env: NativeSystemEnv::detect_for_app(app_id),
            compute: NativeComputePool::new(),
            fs_manager: NativeFsManager::new(app_id),
        }
    }

    #[must_use]
    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    #[must_use]
    pub const fn permissions(&self) -> &AppPermissions {
        &self.permissions
    }

    #[must_use]
    pub const fn check_network_allowed(&self) -> bool {
        self.permissions.network || self.permissions.full_system_access
    }

    #[must_use]
    pub const fn check_sqlite_allowed(&self) -> bool {
        self.permissions.sqlite || self.permissions.full_system_access
    }

    #[must_use]
    pub const fn check_filesystem_xdg_allowed(&self) -> bool {
        self.permissions.filesystem_xdg || self.permissions.full_system_access
    }

    #[must_use]
    pub const fn check_filesystem_dialogs_allowed(&self) -> bool {
        self.permissions.filesystem_dialogs || self.permissions.full_system_access
    }

    #[must_use]
    pub const fn check_clipboard_allowed(&self) -> bool {
        self.permissions.clipboard || self.permissions.full_system_access
    }

    #[must_use]
    pub const fn check_notifications_allowed(&self) -> bool {
        self.permissions.notifications || self.permissions.full_system_access
    }

    #[must_use]
    pub const fn check_compute_allowed(&self) -> bool {
        self.permissions.compute_threads || self.permissions.full_system_access
    }

    pub const fn compute(&self) -> &NativeComputePool {
        &self.compute
    }

    pub const fn fs_manager(&self) -> &NativeFsManager {
        &self.fs_manager
    }

    pub const fn fs_manager_mut(&mut self) -> &mut NativeFsManager {
        &mut self.fs_manager
    }

    pub const fn sqlite(&self) -> &NativeSqliteDatabase {
        &self.sqlite
    }

    pub const fn sqlite_mut(&mut self) -> &mut NativeSqliteDatabase {
        &mut self.sqlite
    }

    pub const fn system_env(&self) -> &NativeSystemEnv {
        &self.system_env
    }

    pub const fn system_env_mut(&mut self) -> &mut NativeSystemEnv {
        &mut self.system_env
    }

    pub fn shared_http_engine(&self) -> Arc<Mutex<NativeHttpEngine>> {
        Arc::clone(&self.http_engine)
    }

    pub const fn theme(&self) -> DesktopTheme {
        self.theme
    }

    pub const fn set_theme(&mut self, theme: DesktopTheme) {
        self.theme = theme;
    }

    pub fn clipboard(&self) -> &str {
        &self.clipboard
    }

    pub fn set_clipboard(&mut self, text: &str) {
        if self.check_clipboard_allowed() {
            text.clone_into(&mut self.clipboard);
        }
    }

    pub fn send_notification(&mut self, title: &str, body: &str, icon: Option<&str>) {
        if self.check_notifications_allowed() {
            self.notifications.push(DesktopNotification {
                title: String::from(title),
                body: String::from(body),
                icon: icon.map(String::from),
            });
        }
    }

    pub fn notifications(&self) -> &[DesktopNotification] {
        &self.notifications
    }

    pub fn open_file_dialog(&mut self, title: &str, default_path: &str) -> String {
        if !self.check_filesystem_dialogs_allowed() {
            return String::new();
        }
        let resolved = if default_path.trim().is_empty() {
            String::from("/home/user/document.txt")
        } else {
            String::from(default_path)
        };
        self.file_dialogs.push(FileDialogRequest {
            kind: FileDialogKind::Open,
            title: String::from(title),
            suggested_path: resolved.clone(),
        });
        resolved
    }

    pub fn save_file_dialog(&mut self, title: &str, suggested_name: &str) -> String {
        if !self.check_filesystem_dialogs_allowed() {
            return String::new();
        }
        let resolved = if suggested_name.trim().is_empty() {
            String::from("/home/user/untitled.txt")
        } else {
            String::from(suggested_name)
        };
        self.file_dialogs.push(FileDialogRequest {
            kind: FileDialogKind::Save,
            title: String::from(title),
            suggested_path: resolved.clone(),
        });
        resolved
    }

    pub fn file_dialogs(&self) -> &[FileDialogRequest] {
        &self.file_dialogs
    }

    pub fn configure_http(&mut self, config: NativeHttpConfig) {
        if let Ok(mut engine) = self.http_engine.lock() {
            engine.set_config(config);
        }
    }

    pub fn http_config(&self) -> NativeHttpConfig {
        self.http_engine
            .lock()
            .map_or_else(|_| NativeHttpConfig::default(), |e| e.config().clone())
    }

    pub fn register_http_endpoint(&mut self, method: &str, url: &str, status: u16, body: &str) {
        if let Ok(mut engine) = self.http_engine.lock() {
            engine.register_http_endpoint(method, url, status, body);
        }
    }

    pub fn execute_http(&mut self, request: &NativeHttpRequest) -> NativeHttpResponse {
        if !self.check_network_allowed() {
            return Self::permission_denied_http_response(request);
        }
        self.http_engine.lock().map_or_else(
            |_| NativeHttpResponse {
                status: 500,
                status_text: String::from("Lock Error"),
                body: String::new(),
                headers: std::collections::HashMap::new(),
                transport: String::from("error"),
                call_kind: request.call_kind,
                cached: false,
            },
            |mut engine| engine.execute_http(request),
        )
    }

    pub fn execute_http_batch(
        &mut self,
        requests: &[NativeHttpRequest],
    ) -> Vec<NativeHttpResponse> {
        if !self.check_network_allowed() {
            return requests
                .iter()
                .map(Self::permission_denied_http_response)
                .collect();
        }
        self.http_engine
            .lock()
            .map_or_else(|_| Vec::new(), |mut engine| engine.execute_batch(requests))
    }

    fn permission_denied_http_response(request: &NativeHttpRequest) -> NativeHttpResponse {
        NativeHttpResponse {
            status: 403,
            status_text: String::from("Forbidden"),
            body: String::from(
                "Permission 'network' denied by application permissions.manifest.json.",
            ),
            headers: std::collections::HashMap::new(),
            transport: String::from("permission-denied"),
            call_kind: request.call_kind,
            cached: false,
        }
    }

    pub fn download_files_parallel(&self, pairs: &[(String, PathBuf)]) -> Vec<bool> {
        self.http_engine.lock().map_or_else(
            |_| vec![false; pairs.len()],
            |engine| engine.download_files_parallel(pairs),
        )
    }

    pub fn http_history(&self) -> Vec<NativeHttpRequest> {
        self.http_engine
            .lock()
            .map_or_else(|_| Vec::new(), |engine| engine.http_history().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manages_desktop_theme_clipboard_notifications_dialogs_and_http_routes() {
        let mut desktop = DesktopServices::new();
        desktop.set_theme(DesktopTheme::Light);
        assert_eq!(desktop.theme().as_str(), "light");

        desktop.set_clipboard("NgNative Clipboard Text");
        assert_eq!(desktop.clipboard(), "NgNative Clipboard Text");

        desktop.send_notification(
            "Build Complete",
            "main.ngb ready",
            Some("sample-desktop-app"),
        );
        assert_eq!(desktop.notifications().len(), 1);
        assert_eq!(desktop.notifications()[0].title, "Build Complete");

        let opened = desktop.open_file_dialog("Open Project", "/tmp/project.json");
        assert_eq!(opened, "/tmp/project.json");
        let saved = desktop.save_file_dialog("Save Export", "/tmp/export.ngb");
        assert_eq!(saved, "/tmp/export.ngb");
        assert_eq!(desktop.file_dialogs().len(), 2);

        desktop.register_http_endpoint(
            "GET",
            "https://api.ng-native.dev/status",
            200,
            r#"{"ok":true}"#,
        );
        let res = desktop.execute_http(&NativeHttpRequest::new(
            "GET",
            "https://api.ng-native.dev/status",
        ));
        assert_eq!(res.status, 200);
        assert_eq!(res.body, r#"{"ok":true}"#);
        assert_eq!(desktop.http_history().len(), 1);
    }

    #[test]
    fn enforces_runtime_capability_manifest_and_blocks_denied_network_requests() {
        let mut desktop = DesktopServices::with_app_config("strict-app", AppPermissions::minimal());
        assert_eq!(desktop.app_id(), "strict-app");
        assert!(!desktop.check_network_allowed());
        assert!(!desktop.check_sqlite_allowed());
        assert!(!desktop.check_clipboard_allowed());
        assert!(!desktop.check_notifications_allowed());
        assert!(!desktop.check_filesystem_dialogs_allowed());
        assert!(!desktop.check_filesystem_xdg_allowed());
        assert!(!desktop.check_compute_allowed());

        desktop.set_clipboard("should-not-write");
        assert_eq!(desktop.clipboard(), "");
        desktop.send_notification("Blocked", "Body", None);
        assert_eq!(desktop.notifications().len(), 0);
        assert_eq!(desktop.open_file_dialog("Open", "/tmp/a"), "");

        let req = NativeHttpRequest::new("GET", "https://api.example.test/v2/items/1");
        let blocked = desktop.execute_http(&req);
        assert_eq!(blocked.status, 403);
        assert_eq!(blocked.status_text, "Forbidden");
        assert_eq!(blocked.transport, "permission-denied");
        assert_eq!(desktop.http_history().len(), 0);

        let batch = desktop.execute_http_batch(&[req]);
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].status, 403);
    }
}

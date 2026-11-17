use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub mod http_native;
mod libsoup_sys;
pub mod pokedex;
pub mod pokedex_ui;

pub use http_native::{
    NativeHttpCallKind, NativeHttpConfig, NativeHttpEngine, NativeHttpRequest, NativeHttpResponse,
    OsHttpPlatform,
};
pub use pokedex::{PokemonIndexEntry, PokemonInfo};
pub use pokedex_ui::PokedexController;

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

/// Cross-platform Desktop OS & Native HTTP service hub for Phase 5 (`@ng-native/desktop` & `@ng-native/http`).
pub struct DesktopServices {
    theme: DesktopTheme,
    clipboard: String,
    notifications: Vec<DesktopNotification>,
    file_dialogs: Vec<FileDialogRequest>,
    http_engine: Arc<Mutex<NativeHttpEngine>>,
}

impl Default for DesktopServices {
    fn default() -> Self {
        Self::new()
    }
}

impl DesktopServices {
    pub fn new() -> Self {
        Self::with_http_engine(Arc::new(Mutex::new(NativeHttpEngine::new())))
    }

    /// Creates a `DesktopServices` instance sharing an existing warm `NativeHttpEngine` pool.
    pub fn with_http_engine(http_engine: Arc<Mutex<NativeHttpEngine>>) -> Self {
        let detected_theme = env::var("GTK_THEME").map_or(DesktopTheme::Dark, |val| {
            if val.to_ascii_lowercase().contains("light") {
                DesktopTheme::Light
            } else {
                DesktopTheme::Dark
            }
        });
        Self {
            theme: detected_theme,
            clipboard: String::new(),
            notifications: Vec::new(),
            file_dialogs: Vec::new(),
            http_engine,
        }
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
        text.clone_into(&mut self.clipboard);
    }

    pub fn send_notification(&mut self, title: &str, body: &str, icon: Option<&str>) {
        self.notifications.push(DesktopNotification {
            title: String::from(title),
            body: String::from(body),
            icon: icon.map(String::from),
        });
    }

    pub fn notifications(&self) -> &[DesktopNotification] {
        &self.notifications
    }

    pub fn open_file_dialog(&mut self, title: &str, default_path: &str) -> String {
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
        self.http_engine
            .lock()
            .map_or_else(|_| Vec::new(), |mut engine| engine.execute_batch(requests))
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

        desktop.send_notification("Build Complete", "main.ngb ready", Some("ng-linux-counter"));
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
}

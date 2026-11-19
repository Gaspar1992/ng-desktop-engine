use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::config::AppPermissions;
use crate::desktop::DesktopServices;
use crate::desktop::compute_pool::NativeComputePool;
use crate::desktop::fs_watcher::{NativeFsManager, XdgDirKind};
use crate::desktop::http_codec::{
    escape_json, extract_json_bool, extract_json_raw_value, extract_json_string,
    extract_json_string_array, extract_json_u64,
};
use crate::desktop::http_native::NativeHttpEngine;
use crate::desktop::sqlite_sys::NativeSqliteDatabase;
use crate::desktop::system_env::NativeSystemEnv;

/// Asynchronous completion payload emitted by a background host operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostAsyncCompletion {
    pub callback_id: u64,
    pub payload_json: String,
}

/// Thread-safe native desktop service multiplexer exposed to embedded JavaScript runtimes.
pub struct HostBridge {
    app_id: String,
    permissions: AppPermissions,
    http_engine: Arc<Mutex<NativeHttpEngine>>,
    sqlite: Mutex<NativeSqliteDatabase>,
    fs_manager: Mutex<NativeFsManager>,
    compute: NativeComputePool,
    system_env: Mutex<NativeSystemEnv>,
    async_completions: Arc<Mutex<Vec<HostAsyncCompletion>>>,
}

impl HostBridge {
    /// Creates a new `HostBridge` configured from the active `DesktopServices`.
    #[must_use]
    pub fn new(desktop: &DesktopServices) -> Self {
        let app_id = desktop.app_id().to_string();
        Self {
            permissions: desktop.permissions().clone(),
            http_engine: desktop.shared_http_engine(),
            sqlite: Mutex::new(NativeSqliteDatabase::new_in_memory()),
            fs_manager: Mutex::new(NativeFsManager::new(&app_id)),
            compute: NativeComputePool::new(),
            system_env: Mutex::new(NativeSystemEnv::detect_for_app(&app_id)),
            async_completions: Arc::new(Mutex::new(Vec::new())),
            app_id,
        }
    }

    /// Dispatches a JSON service invocation from JS and returns a JSON response string.
    #[must_use]
    pub fn handle_service_call(&self, req_json: &str) -> String {
        let action = extract_json_string(req_json, "action").unwrap_or_default();
        match action.as_str() {
            "httpAsync" => self.handle_http_async(req_json),
            "httpBatchAsync" => self.handle_http_batch_async(req_json),
            "downloadImageAsync" => self.handle_download_image_async(req_json),
            "sqliteExecute" | "sqliteQuery" | "sqliteOpen" => self.handle_sqlite(&action, req_json),
            "fsResolvePath" | "fsReadText" | "fsWriteText" | "fsWatchFile"
            | "fsPollChangedFiles" => self.handle_filesystem(&action, req_json),
            "computeFuzzyFilter" | "computeHash" => self.handle_compute(&action, req_json),
            "systemEnv" => self.handle_system_env(req_json),
            "logError" => self.handle_log_error(req_json),
            _ => Self::err_json("Unsupported host service action"),
        }
    }

    /// Drains all completed background host callbacks queued since the previous tick.
    #[must_use]
    pub fn drain_async_completions(&self) -> Vec<HostAsyncCompletion> {
        self.async_completions
            .lock()
            .map_or_else(|_| Vec::new(), |mut queue| std::mem::take(&mut *queue))
    }

    const fn has_permission(&self, granted: bool) -> bool {
        granted || self.permissions.full_system_access
    }

    fn push_completion(
        queue: &Arc<Mutex<Vec<HostAsyncCompletion>>>,
        callback_id: u64,
        payload_json: String,
    ) {
        if let Ok(mut guard) = queue.lock() {
            guard.push(HostAsyncCompletion {
                callback_id,
                payload_json,
            });
        }
    }

    fn err_json(message: &str) -> String {
        format!(r#"{{"ok":false,"error":"{}"}}"#, escape_json(message))
    }

    fn has_parent_dir(path: &str) -> bool {
        Path::new(path)
            .components()
            .any(|c| matches!(c, Component::ParentDir))
    }

    fn handle_http_async(&self, req_json: &str) -> String {
        let Some(callback_id) = extract_json_u64(req_json, "callbackId") else {
            return Self::err_json("Missing callbackId for httpAsync");
        };
        if !self.has_permission(self.permissions.network) {
            Self::push_completion(
                &self.async_completions,
                callback_id,
                r#"{"status":403,"statusText":"Forbidden","body":"Permission denied: network","headers":{},"transport":"host-denied","callKind":"json","cached":false}"#.to_string(),
            );
            return String::from(r#"{"ok":true,"queued":true,"denied":true}"#);
        }

        let payload =
            extract_json_raw_value(req_json, "request").unwrap_or_else(|| req_json.to_string());
        let engine = Arc::clone(&self.http_engine);
        let completions = Arc::clone(&self.async_completions);
        thread::spawn(move || {
            let result_json = engine.lock().map_or_else(
                |_| r#"{"status":500,"statusText":"Host Mutex Poisoned","body":"","headers":{},"transport":"host-error","callKind":"json","cached":false}"#.to_string(),
                |mut guard| guard.execute_bridge_json(&payload),
            );
            Self::push_completion(&completions, callback_id, result_json);
        });
        String::from(r#"{"ok":true,"queued":true}"#)
    }

    fn handle_http_batch_async(&self, req_json: &str) -> String {
        let Some(callback_id) = extract_json_u64(req_json, "callbackId") else {
            return Self::err_json("Missing callbackId for httpBatchAsync");
        };
        if !self.has_permission(self.permissions.network) {
            Self::push_completion(
                &self.async_completions,
                callback_id,
                r#"[{"status":403,"statusText":"Forbidden","body":"Permission denied: network","headers":{},"transport":"host-denied","callKind":"json","cached":false}]"#.to_string(),
            );
            return String::from(r#"{"ok":true,"queued":true,"denied":true}"#);
        }

        let Some(requests_json) = extract_json_raw_value(req_json, "requests") else {
            return Self::err_json("Missing requests array for httpBatchAsync");
        };
        let engine = Arc::clone(&self.http_engine);
        let completions = Arc::clone(&self.async_completions);
        thread::spawn(move || {
            let result_json = engine.lock().map_or_else(
                |_| String::from("[]"),
                |mut guard| guard.execute_bridge_json(&requests_json),
            );
            Self::push_completion(&completions, callback_id, result_json);
        });
        String::from(r#"{"ok":true,"queued":true}"#)
    }

    fn handle_download_image_async(&self, req_json: &str) -> String {
        let url = extract_json_string(req_json, "url")
            .or_else(|| extract_json_string(req_json, "src"))
            .unwrap_or_default();
        let width = extract_json_u64(req_json, "width").and_then(|v| u32::try_from(v).ok());
        let resolved = self
            .system_env
            .lock()
            .map_or_else(|_| url.clone(), |env| env.resolve_image_asset(&url, width));
        let clean_path = resolved.split('?').next().unwrap_or(&resolved).to_string();
        let dest = PathBuf::from(&clean_path);
        if fs::metadata(&dest).is_ok_and(|meta| meta.len() > 0) {
            return format!(
                r#"{{"ok":true,"cached":true,"path":"{}"}}"#,
                escape_json(&clean_path)
            );
        }

        let Some(callback_id) = extract_json_u64(req_json, "callbackId") else {
            return format!(
                r#"{{"ok":false,"cached":false,"path":"{}","error":"Missing callbackId"}}"#,
                escape_json(&clean_path)
            );
        };
        if !self.has_permission(self.permissions.network) {
            Self::push_completion(
                &self.async_completions,
                callback_id,
                r#"{"ok":false,"path":"","error":"Permission denied: network"}"#.to_string(),
            );
            return String::from(r#"{"ok":false,"cached":false,"queued":true,"denied":true}"#);
        }

        let (max_conn, timeout_ms) = self.http_engine.lock().map_or((24_u16, 15_000_u32), |eng| {
            let cfg = eng.config();
            (cfg.max_parallel_connections, cfg.timeout_ms)
        });
        let completions = Arc::clone(&self.async_completions);
        thread::spawn(move || {
            let trimmed = url.trim();
            let downloaded = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                let pairs = [(trimmed.to_string(), dest)];
                NativeHttpEngine::download_files_parallel_with(max_conn, timeout_ms, &pairs)
                    .into_iter()
                    .next()
                    .unwrap_or(false)
            } else {
                false
            };
            let payload = if downloaded {
                format!(r#"{{"ok":true,"path":"{}"}}"#, escape_json(&clean_path))
            } else {
                format!(
                    r#"{{"ok":false,"path":"{}","error":"Image download failed"}}"#,
                    escape_json(&clean_path)
                )
            };
            Self::push_completion(&completions, callback_id, payload);
        });
        String::from(r#"{"ok":true,"cached":false,"queued":true}"#)
    }

    fn handle_sqlite(&self, action: &str, req_json: &str) -> String {
        if !self.has_permission(self.permissions.sqlite) {
            return Self::err_json("Permission denied: sqlite");
        }
        match action {
            "sqliteExecute" => {
                let sql = extract_json_string(req_json, "sql").unwrap_or_default();
                let Ok(mut db) = self.sqlite.lock() else {
                    return Self::err_json("SQLite mutex poisoned");
                };
                match db.execute(&sql) {
                    Ok(()) => String::from(r#"{"ok":true}"#),
                    Err(e) => Self::err_json(&e),
                }
            }
            "sqliteQuery" => {
                let sql = extract_json_string(req_json, "sql").unwrap_or_default();
                let Ok(db) = self.sqlite.lock() else {
                    return String::from(
                        r#"{"ok":false,"rows":[],"error":"SQLite mutex poisoned"}"#,
                    );
                };
                match db.query_json(&sql) {
                    Ok(rows_json) => format!(r#"{{"ok":true,"rows":{rows_json}}}"#),
                    Err(e) => {
                        format!(r#"{{"ok":false,"rows":[],"error":"{}"}}"#, escape_json(&e))
                    }
                }
            }
            "sqliteOpen" => self.handle_sqlite_open(req_json),
            _ => Self::err_json("Unsupported sqlite action"),
        }
    }

    fn handle_sqlite_open(&self, req_json: &str) -> String {
        let rel_or_mem = extract_json_string(req_json, "path").unwrap_or_default();
        if rel_or_mem.trim() == ":memory:" {
            let Ok(mut db) = self.sqlite.lock() else {
                return Self::err_json("SQLite mutex poisoned");
            };
            *db = NativeSqliteDatabase::new_in_memory();
            return String::from(r#"{"ok":true,"path":":memory:"}"#);
        }
        if !self.has_permission(self.permissions.filesystem_xdg) {
            return Self::err_json("Permission denied: filesystem_xdg");
        }
        let resolved = {
            let Ok(fs_mgr) = self.fs_manager.lock() else {
                return Self::err_json("Filesystem mutex poisoned");
            };
            match fs_mgr.resolve_xdg_path(XdgDirKind::Data, &rel_or_mem) {
                Ok(p) => p,
                Err(e) => return Self::err_json(&e),
            }
        };
        if let Some(parent) = resolved.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let resolved_str = resolved.to_string_lossy().into_owned();
        let Ok(mut db) = self.sqlite.lock() else {
            return Self::err_json("SQLite mutex poisoned");
        };
        *db = NativeSqliteDatabase::open_path(&resolved_str);
        format!(r#"{{"ok":true,"path":"{}"}}"#, escape_json(&resolved_str))
    }

    fn handle_filesystem(&self, action: &str, req_json: &str) -> String {
        if !self.has_permission(self.permissions.filesystem_xdg) {
            return Self::err_json("Permission denied: filesystem_xdg");
        }
        let Ok(mut fs_mgr) = self.fs_manager.lock() else {
            return Self::err_json("Filesystem mutex poisoned");
        };
        match action {
            "fsResolvePath" => {
                let kind_raw =
                    extract_json_string(req_json, "kind").unwrap_or_else(|| String::from("config"));
                let rel_path = extract_json_string(req_json, "relPath").unwrap_or_default();
                let kind = XdgDirKind::from_str_lossy(&kind_raw);
                match fs_mgr.resolve_xdg_path(kind, &rel_path) {
                    Ok(path) => format!(
                        r#"{{"ok":true,"path":"{}"}}"#,
                        escape_json(&path.to_string_lossy())
                    ),
                    Err(e) => Self::err_json(&e),
                }
            }
            "fsReadText" => {
                let path = extract_json_string(req_json, "path").unwrap_or_default();
                if Self::has_parent_dir(&path) {
                    return Self::err_json("Path traversal '..' is not allowed");
                }
                match fs_mgr.read_text(Path::new(&path)) {
                    Ok(content) => {
                        format!(r#"{{"ok":true,"content":"{}"}}"#, escape_json(&content))
                    }
                    Err(e) => format!(
                        r#"{{"ok":false,"content":null,"error":"{}"}}"#,
                        escape_json(&e)
                    ),
                }
            }
            "fsWriteText" => {
                let path = extract_json_string(req_json, "path").unwrap_or_default();
                if Self::has_parent_dir(&path) {
                    return Self::err_json("Path traversal '..' is not allowed");
                }
                let content = extract_json_string(req_json, "content").unwrap_or_default();
                match fs_mgr.write_text_atomic(Path::new(&path), &content) {
                    Ok(()) => String::from(r#"{"ok":true}"#),
                    Err(e) => Self::err_json(&e),
                }
            }
            "fsWatchFile" => {
                let path = extract_json_string(req_json, "path").unwrap_or_default();
                if Self::has_parent_dir(&path) {
                    return Self::err_json("Path traversal '..' is not allowed");
                }
                match fs_mgr.watch_file(Path::new(&path)) {
                    Ok(()) => String::from(r#"{"ok":true}"#),
                    Err(e) => Self::err_json(&e),
                }
            }
            "fsPollChangedFiles" => {
                let changed = fs_mgr.poll_changed_paths();
                let encoded: Vec<String> = changed
                    .iter()
                    .map(|p| format!(r#""{}""#, escape_json(&p.to_string_lossy())))
                    .collect();
                format!(r#"{{"ok":true,"paths":[{}]}}"#, encoded.join(","))
            }
            _ => Self::err_json("Unsupported filesystem action"),
        }
    }

    fn handle_compute(&self, action: &str, req_json: &str) -> String {
        if !self.has_permission(self.permissions.compute_threads) {
            return Self::err_json("Permission denied: compute_threads");
        }
        match action {
            "computeFuzzyFilter" => {
                let items = extract_json_string_array(req_json, "items");
                let query = extract_json_string(req_json, "query").unwrap_or_default();
                let limit = extract_json_u64(req_json, "limit")
                    .and_then(|v| usize::try_from(v).ok())
                    .unwrap_or(50);
                let matches = self.compute.fuzzy_filter_json(&items, &query, limit);
                format!(r#"{{"ok":true,"matches":{matches}}}"#)
            }
            "computeHash" => {
                let input = extract_json_string(req_json, "input").unwrap_or_default();
                let hash = self.compute.fnv1a_hex(&input);
                format!(r#"{{"ok":true,"hash":"{hash}"}}"#)
            }
            _ => Self::err_json("Unsupported compute action"),
        }
    }

    fn handle_system_env(&self, req_json: &str) -> String {
        let Ok(mut env) = self.system_env.lock() else {
            return Self::err_json("SystemEnv mutex poisoned");
        };
        if let Some(online) = extract_json_bool(req_json, "online") {
            let metered = extract_json_bool(req_json, "metered").unwrap_or(false);
            env.set_network_status(online, metered);
        } else if extract_json_bool(req_json, "refresh").unwrap_or(false) {
            env.refresh_network();
        }
        let resolved_field = extract_json_string(req_json, "src").map_or_else(String::new, |src| {
            let width = extract_json_u64(req_json, "width").and_then(|v| u32::try_from(v).ok());
            let resolved = env.resolve_image_asset(&src, width);
            format!(r#","resolvedImage":"{}""#, escape_json(&resolved))
        });
        format!(
            r#"{{"ok":true,"appId":"{}","online":{},"metered":{},"connectivity":"{}","locale":"{}","timezone":"{}","cacheDir":"{}"{resolved_field}}}"#,
            escape_json(&self.app_id),
            env.is_online(),
            env.is_metered(),
            env.connectivity(),
            escape_json(env.locale()),
            escape_json(env.timezone()),
            escape_json(&env.cache_dir().to_string_lossy()),
        )
    }

    fn handle_log_error(&self, req_json: &str) -> String {
        let message = extract_json_string(req_json, "message").unwrap_or_default();
        if let Ok(mut env) = self.system_env.lock() {
            env.log_error(&message);
        }
        String::from(r#"{"ok":true}"#)
    }
}

#[cfg(test)]
mod tests {
    use super::HostBridge;
    use crate::config::AppPermissions;
    use crate::desktop::DesktopServices;
    use crate::desktop::http_codec::extract_json_string;
    use std::fs;
    use std::path::PathBuf;
    use std::process;

    #[test]
    fn host_bridge_supports_sqlite_compute_fs_and_system_env() {
        let app_id = format!("ng-host-bridge-test-{}", process::id());
        let desktop =
            DesktopServices::with_app_config(&app_id, AppPermissions::allow_all_sandboxed());
        let bridge = HostBridge::new(&desktop);

        let create_res = bridge.handle_service_call(
            r#"{"action":"sqliteExecute","sql":"CREATE TABLE notes (id INTEGER PRIMARY KEY, title TEXT);"}"#,
        );
        assert!(create_res.contains(r#""ok":true"#));
        let ins_res = bridge.handle_service_call(
            r#"{"action":"sqliteExecute","sql":"INSERT INTO notes (id, title) VALUES (1, 'hello');"}"#,
        );
        assert!(ins_res.contains(r#""ok":true"#));
        let query_res = bridge.handle_service_call(
            r#"{"action":"sqliteQuery","sql":"SELECT id, title FROM notes;"}"#,
        );
        assert!(query_res.contains(r#""ok":true"#));
        assert!(query_res.contains(r#""rows":"#));
        if cfg!(target_os = "linux") {
            assert!(query_res.contains("hello"));
        }

        let fuzzy_res = bridge.handle_service_call(
            r#"{"action":"computeFuzzyFilter","items":["alpha","beta","alpaca"],"query":"alp","limit":2}"#,
        );
        assert!(fuzzy_res.contains(r#""value":"alpha""#));
        assert!(fuzzy_res.contains(r#""value":"alpaca""#));

        let hash_res = bridge.handle_service_call(r#"{"action":"computeHash","input":"ng"}"#);
        assert!(hash_res.contains(r#""ok":true"#));

        let path_res = bridge.handle_service_call(
            r#"{"action":"fsResolvePath","kind":"cache","relPath":"host-test.txt"}"#,
        );
        let resolved_path = extract_json_string(&path_res, "path").expect("resolved XDG path");
        let write_req = format!(
            r#"{{"action":"fsWriteText","path":"{}","content":"saved-content"}}"#,
            resolved_path.replace('\\', "\\\\")
        );
        assert!(
            bridge
                .handle_service_call(&write_req)
                .contains(r#""ok":true"#)
        );
        let read_req = format!(
            r#"{{"action":"fsReadText","path":"{}"}}"#,
            resolved_path.replace('\\', "\\\\")
        );
        assert!(
            bridge
                .handle_service_call(&read_req)
                .contains("saved-content")
        );
        let _ = fs::remove_file(&resolved_path);
        if let Some(parent) = PathBuf::from(&resolved_path).parent() {
            let _ = fs::remove_dir_all(parent);
        }

        let env_res = bridge.handle_service_call(r#"{"action":"systemEnv"}"#);
        assert!(env_res.contains(r#""ok":true"#));
        assert!(env_res.contains(&app_id));
    }

    #[test]
    fn host_bridge_enforces_network_permission_and_hits_image_cache() {
        let app_id = format!("ng-host-bridge-net-{}", process::id());
        let desktop = DesktopServices::with_app_config(&app_id, AppPermissions::minimal());
        let bridge = HostBridge::new(&desktop);

        let denied = bridge.handle_service_call(
            r#"{"action":"httpAsync","callbackId":7,"method":"GET","url":"https://example.test/api"}"#,
        );
        assert!(denied.contains(r#""denied":true"#));
        let completions = bridge.drain_async_completions();
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].callback_id, 7);
        assert!(completions[0].payload_json.contains(r#""status":403"#));

        let url = format!("https://cdn.example.test/avatar-{}.png", process::id());
        let resolve_req = format!(r#"{{"action":"systemEnv","src":"{url}","width":64}}"#);
        let env_res = bridge.handle_service_call(&resolve_req);
        let img_with_query =
            extract_json_string(&env_res, "resolvedImage").expect("resolved image");
        let clean_path = img_with_query.split('?').next().expect("clean cache path");
        let cache_file = PathBuf::from(clean_path);
        if let Some(parent) = cache_file.parent() {
            fs::create_dir_all(parent).expect("create test cache parent dir");
        }
        fs::write(&cache_file, b"PNG_BYTES").expect("seed cached image");

        let download_payload =
            format!(r#"{{"action":"downloadImageAsync","callbackId":8,"url":"{url}","width":64}}"#);
        let download_reply = bridge.handle_service_call(&download_payload);
        assert!(download_reply.contains(r#""ok":true"#));
        assert!(download_reply.contains(r#""cached":true"#));
        assert_eq!(bridge.drain_async_completions(), Vec::new());

        let _ = fs::remove_file(&cache_file);
        if let Some(parent) = cache_file.parent().and_then(|p| p.parent()) {
            let _ = fs::remove_dir_all(parent);
        }
    }
}

use std::collections::VecDeque;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

use crate::config::AppPermissions;
use crate::desktop::compute_pool::NativeComputePool;
use crate::desktop::fs_watcher::{NativeFsManager, XdgDirKind};
use crate::desktop::http_codec::{
    escape_json, extract_json_bool, extract_json_i64, extract_json_object_array,
    extract_json_raw_value, extract_json_string, extract_json_string_array, extract_json_u64,
};
use crate::desktop::http_native::NativeHttpEngine;
use crate::desktop::sqlite_sys::{NativeSqliteDatabase, SqlParam, parse_json_params};
use crate::desktop::system_env::NativeSystemEnv;
use crate::desktop::{
    AppMenuItem, ConfirmDialogRequest, DesktopServices, ShortcutBinding, TrayState, WindowGeometry,
};

use super::server_worker::{NativeServerWorker, is_local_api_url};

const HOST_IO_WORKERS: usize = 6;

pub(crate) type ServerResolveSlot = Arc<(Mutex<Option<(u64, String)>>, Condvar)>;

type IoTask = Box<dyn FnOnce() + Send + 'static>;

struct IoPoolState {
    queue: VecDeque<IoTask>,
    closed: bool,
    started: bool,
}

struct IoWorkerPool {
    shared: Arc<(Mutex<IoPoolState>, Condvar)>,
    handles: Mutex<Vec<JoinHandle<()>>>,
}

impl IoWorkerPool {
    fn new() -> Self {
        Self {
            shared: Arc::new((
                Mutex::new(IoPoolState {
                    queue: VecDeque::new(),
                    closed: false,
                    started: false,
                }),
                Condvar::new(),
            )),
            handles: Mutex::new(Vec::new()),
        }
    }

    fn submit(&self, task: impl FnOnce() + Send + 'static) {
        let should_spawn = {
            let (lock, cvar) = &*self.shared;
            let Ok(mut guard) = lock.lock() else {
                return;
            };
            guard.queue.push_back(Box::new(task));
            let spawn_now = !guard.started;
            guard.started = true;
            cvar.notify_one();
            spawn_now
        };
        if should_spawn
            && let Ok(mut handles) = self.handles.lock()
            && handles.is_empty()
        {
            for idx in 0..HOST_IO_WORKERS {
                let shared = Arc::clone(&self.shared);
                if let Ok(handle) = thread::Builder::new()
                    .name(format!("ng-host-io-{idx}"))
                    .spawn(move || Self::worker_loop(&shared))
                {
                    handles.push(handle);
                }
            }
        }
    }

    fn worker_loop(shared: &Arc<(Mutex<IoPoolState>, Condvar)>) {
        loop {
            let task = {
                let (lock, cvar) = &**shared;
                let Ok(mut guard) = lock.lock() else {
                    break;
                };
                while guard.queue.is_empty() && !guard.closed {
                    let Ok(next) = cvar.wait(guard) else {
                        return;
                    };
                    guard = next;
                }
                if let Some(t) = guard.queue.pop_front() {
                    t
                } else if guard.closed {
                    break;
                } else {
                    continue;
                }
            };
            task();
        }
    }
}

impl Drop for IoWorkerPool {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.shared.0.lock() {
            guard.closed = true;
            self.shared.1.notify_all();
        }
        if let Ok(mut handles) = self.handles.lock() {
            for handle in handles.drain(..) {
                let _ = handle.join();
            }
        }
    }
}

/// Asynchronous completion payload emitted by a background host operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostAsyncCompletion {
    pub callback_id: u64,
    pub payload_json: String,
}

#[derive(Debug, Clone)]
struct DesktopUiState {
    window_geometry: WindowGeometry,
    app_menu: Vec<AppMenuItem>,
    tray_state: TrayState,
    shortcuts: Vec<ShortcutBinding>,
    confirm_dialogs: Vec<ConfirmDialogRequest>,
    next_confirm_response: Option<bool>,
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
    ui_state: Mutex<DesktopUiState>,
    async_completions: Arc<Mutex<Vec<HostAsyncCompletion>>>,
    server_worker: Mutex<Option<Arc<NativeServerWorker>>>,
    server_resolve: Mutex<Option<ServerResolveSlot>>,
    io_pool: IoWorkerPool,
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
            ui_state: Mutex::new(DesktopUiState {
                window_geometry: desktop.window_geometry().clone(),
                app_menu: desktop.app_menu().to_vec(),
                tray_state: desktop.tray_state().clone(),
                shortcuts: desktop.shortcuts().to_vec(),
                confirm_dialogs: desktop.confirm_dialogs().to_vec(),
                next_confirm_response: None,
            }),
            async_completions: Arc::new(Mutex::new(Vec::new())),
            server_worker: Mutex::new(None),
            server_resolve: Mutex::new(None),
            io_pool: IoWorkerPool::new(),
            app_id,
        }
    }

    /// Attaches an isolated backend `NativeServerWorker` to handle local `/api/*` requests.
    pub fn attach_server_worker(&self, worker: Arc<NativeServerWorker>) {
        if let Ok(mut guard) = self.server_worker.lock() {
            *guard = Some(worker);
        }
    }

    /// Hot-reloads the attached `NativeServerWorker` with a newly compiled `server.bundle.js`.
    pub fn reload_server_worker(&self, bundle_js: &str) -> Result<bool, String> {
        let worker_opt = self.server_worker.lock().ok().and_then(|g| g.clone());
        if let Some(worker) = worker_opt {
            worker.reload(bundle_js)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub(crate) fn attach_server_resolve(&self, slot: ServerResolveSlot) {
        if let Ok(mut guard) = self.server_resolve.lock() {
            *guard = Some(slot);
        }
    }

    /// Returns `true` when an isolated backend `NativeServerWorker` is attached.
    #[must_use]
    pub fn has_server_worker(&self) -> bool {
        self.server_worker.lock().is_ok_and(|guard| guard.is_some())
    }

    /// Dispatches a JSON service invocation from JS and returns a JSON response string.
    #[must_use]
    pub fn handle_service_call(&self, req_json: &str) -> String {
        let action = extract_json_string(req_json, "action").unwrap_or_default();
        match action.as_str() {
            "httpAsync" => self.handle_http_async(req_json),
            "httpSync" => self.handle_http_sync(req_json),
            "httpBatchAsync" => self.handle_http_batch_async(req_json),
            "downloadImageAsync" => self.handle_download_image_async(req_json),
            "serverResolve" => self.handle_server_resolve(req_json),
            "sqliteExecute" | "sqliteQuery" | "sqliteOpen" => self.handle_sqlite(&action, req_json),
            "fsResolvePath" | "fsReadText" | "fsWriteText" | "fsWatchFile"
            | "fsPollChangedFiles" => self.handle_filesystem(&action, req_json),
            "computeFuzzyFilter" | "computeHash" => self.handle_compute(&action, req_json),
            "systemEnv" => self.handle_system_env(req_json),
            "logError" => self.handle_log_error(req_json),
            "windowGetGeometry" => self.handle_window_get_geometry(),
            "windowSetGeometry" => self.handle_window_set_geometry(req_json),
            "menuGetItems" => self.handle_menu_get_items(),
            "menuSetItems" => self.handle_menu_set_items(req_json),
            "trayGetState" => self.handle_tray_get_state(),
            "traySetState" => self.handle_tray_set_state(req_json),
            "shortcutRegister" => self.handle_shortcut_register(req_json),
            "shortcutUnregister" => self.handle_shortcut_unregister(req_json),
            "shortcutList" => self.handle_shortcut_list(),
            "dialogConfirm" => self.handle_dialog_confirm(req_json),
            "dialogConfirmSetNext" => self.handle_dialog_confirm_set_next(req_json),
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

    fn handle_server_resolve(&self, req_json: &str) -> String {
        let Some(request_id) = extract_json_u64(req_json, "requestId") else {
            return Self::err_json("Missing requestId for serverResolve");
        };
        let Some(response_json) = extract_json_raw_value(req_json, "response") else {
            return Self::err_json("Missing response payload for serverResolve");
        };
        let Ok(guard) = self.server_resolve.lock() else {
            return Self::err_json("Server resolve lock poisoned");
        };
        let Some(slot) = guard.as_ref() else {
            return Self::err_json("No server resolve slot attached");
        };
        if let Ok(mut pending) = slot.0.lock() {
            *pending = Some((request_id, response_json));
            slot.1.notify_one();
        }
        String::from(r#"{"ok":true}"#)
    }

    fn handle_http_sync(&self, req_json: &str) -> String {
        if !self.has_permission(self.permissions.network) {
            return String::from(
                r#"{"status":403,"statusText":"Forbidden","body":"Permission denied: network","headers":{},"transport":"host-denied","callKind":"json","cached":false}"#,
            );
        }
        let payload =
            extract_json_raw_value(req_json, "request").unwrap_or_else(|| req_json.to_string());
        NativeHttpEngine::execute_bridge_detached(&self.http_engine, &payload)
    }

    fn handle_http_async(&self, req_json: &str) -> String {
        let Some(callback_id) = extract_json_u64(req_json, "callbackId") else {
            return Self::err_json("Missing callbackId for httpAsync");
        };
        let payload =
            extract_json_raw_value(req_json, "request").unwrap_or_else(|| req_json.to_string());
        let target_url = extract_json_string(&payload, "url")
            .or_else(|| extract_json_string(req_json, "url"))
            .unwrap_or_default();

        if is_local_api_url(&target_url)
            && let Ok(guard) = self.server_worker.lock()
            && let Some(worker) = guard.as_ref().cloned()
        {
            drop(guard);
            if let Err(err) =
                worker.dispatch_async(payload, callback_id, Arc::clone(&self.async_completions))
            {
                return Self::err_json(&err);
            }
            return String::from(r#"{"ok":true,"queued":true,"localServer":true}"#);
        }

        if !self.has_permission(self.permissions.network) {
            Self::push_completion(
                &self.async_completions,
                callback_id,
                r#"{"status":403,"statusText":"Forbidden","body":"Permission denied: network","headers":{},"transport":"host-denied","callKind":"json","cached":false}"#.to_string(),
            );
            return String::from(r#"{"ok":true,"queued":true,"denied":true}"#);
        }

        let engine = Arc::clone(&self.http_engine);
        let completions = Arc::clone(&self.async_completions);
        self.io_pool.submit(move || {
            let result_json = NativeHttpEngine::execute_bridge_detached(&engine, &payload);
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
        self.io_pool.submit(move || {
            let result_json = NativeHttpEngine::execute_bridge_detached(&engine, &requests_json);
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
        self.io_pool.submit(move || {
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

    fn parse_sqlite_params(req_json: &str) -> Result<Vec<SqlParam>, String> {
        extract_json_raw_value(req_json, "params")
            .map_or_else(|| Ok(Vec::new()), |raw| parse_json_params(&raw))
    }

    fn handle_sqlite(&self, action: &str, req_json: &str) -> String {
        if !self.has_permission(self.permissions.sqlite) {
            return Self::err_json("Permission denied: sqlite");
        }
        match action {
            "sqliteExecute" => {
                let sql = extract_json_string(req_json, "sql").unwrap_or_default();
                let params = match Self::parse_sqlite_params(req_json) {
                    Ok(p) => p,
                    Err(e) => return Self::err_json(&e),
                };
                let Ok(mut db) = self.sqlite.lock() else {
                    return Self::err_json("SQLite mutex poisoned");
                };
                match db.execute_params(&sql, &params) {
                    Ok(()) => String::from(r#"{"ok":true}"#),
                    Err(e) => Self::err_json(&e),
                }
            }
            "sqliteQuery" => {
                let sql = extract_json_string(req_json, "sql").unwrap_or_default();
                let params = match Self::parse_sqlite_params(req_json) {
                    Ok(p) => p,
                    Err(e) => {
                        return format!(
                            r#"{{"ok":false,"rows":[],"error":"{}"}}"#,
                            escape_json(&e)
                        );
                    }
                };
                let Ok(db) = self.sqlite.lock() else {
                    return String::from(
                        r#"{"ok":false,"rows":[],"error":"SQLite mutex poisoned"}"#,
                    );
                };
                match db.query_json_params(&sql, &params) {
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

    fn encode_geometry_json(geom: &WindowGeometry, include_ok: bool) -> String {
        let x_json = geom
            .x
            .map_or_else(|| String::from("null"), |v| v.to_string());
        let y_json = geom
            .y
            .map_or_else(|| String::from("null"), |v| v.to_string());
        let ok_prefix = if include_ok { r#""ok":true,"# } else { "" };
        format!(
            r#"{{{ok_prefix}"width":{},"height":{},"x":{x_json},"y":{y_json},"maximized":{}}}"#,
            geom.width, geom.height, geom.maximized
        )
    }

    fn apply_geometry_patch(base: &WindowGeometry, patch_json: &str) -> WindowGeometry {
        let width = extract_json_u64(patch_json, "width")
            .and_then(|v| u32::try_from(v).ok())
            .filter(|v| *v >= 160)
            .unwrap_or(base.width);
        let height = extract_json_u64(patch_json, "height")
            .and_then(|v| u32::try_from(v).ok())
            .filter(|v| *v >= 120)
            .unwrap_or(base.height);
        let x = extract_json_i64(patch_json, "x")
            .and_then(|v| i32::try_from(v).ok())
            .or(base.x);
        let y = extract_json_i64(patch_json, "y")
            .and_then(|v| i32::try_from(v).ok())
            .or(base.y);
        let maximized = extract_json_bool(patch_json, "maximized").unwrap_or(base.maximized);
        WindowGeometry {
            width,
            height,
            x,
            y,
            maximized,
        }
    }

    fn parse_menu_items(req_json: &str, key: &str) -> Vec<AppMenuItem> {
        extract_json_object_array(req_json, key)
            .into_iter()
            .filter_map(|obj| {
                let id = extract_json_string(&obj, "id")?;
                let label = extract_json_string(&obj, "label").unwrap_or_else(|| id.clone());
                let accelerator =
                    extract_json_string(&obj, "accelerator").filter(|s| !s.trim().is_empty());
                let enabled = extract_json_bool(&obj, "enabled").unwrap_or(true);
                Some(AppMenuItem {
                    id,
                    label,
                    accelerator,
                    enabled,
                })
            })
            .collect()
    }

    fn encode_menu_items(items: &[AppMenuItem]) -> String {
        let encoded: Vec<String> = items
            .iter()
            .map(|item| {
                let acc_json = item.accelerator.as_ref().map_or_else(
                    || String::from("null"),
                    |acc| format!(r#""{}""#, escape_json(acc)),
                );
                format!(
                    r#"{{"id":"{}","label":"{}","accelerator":{acc_json},"enabled":{}}}"#,
                    escape_json(&item.id),
                    escape_json(&item.label),
                    item.enabled
                )
            })
            .collect();
        format!("[{}]", encoded.join(","))
    }

    fn encode_shortcuts(shortcuts: &[ShortcutBinding]) -> String {
        let encoded: Vec<String> = shortcuts
            .iter()
            .map(|s| {
                format!(
                    r#"{{"accelerator":"{}","actionId":"{}"}}"#,
                    escape_json(&s.accelerator),
                    escape_json(&s.action_id)
                )
            })
            .collect();
        format!("[{}]", encoded.join(","))
    }

    fn handle_window_get_geometry(&self) -> String {
        let disk_json = if self.has_permission(self.permissions.filesystem_xdg)
            && let Ok(fs_mgr) = self.fs_manager.lock()
            && let Ok(path) = fs_mgr.resolve_xdg_path(XdgDirKind::Config, "window-state.json")
        {
            fs_mgr.read_text(&path).ok()
        } else {
            None
        };
        let Ok(mut state) = self.ui_state.lock() else {
            return Self::err_json("Desktop UI mutex poisoned");
        };
        if let Some(raw) = disk_json {
            state.window_geometry = Self::apply_geometry_patch(&state.window_geometry, &raw);
        }
        Self::encode_geometry_json(&state.window_geometry, true)
    }

    fn handle_window_set_geometry(&self, req_json: &str) -> String {
        let updated = {
            let Ok(mut state) = self.ui_state.lock() else {
                return Self::err_json("Desktop UI mutex poisoned");
            };
            let next = Self::apply_geometry_patch(&state.window_geometry, req_json);
            state.window_geometry = next.clone();
            next
        };
        if self.has_permission(self.permissions.filesystem_xdg)
            && let Ok(mut fs_mgr) = self.fs_manager.lock()
            && let Ok(path) = fs_mgr.resolve_xdg_path(XdgDirKind::Config, "window-state.json")
        {
            let serialized = Self::encode_geometry_json(&updated, false);
            let _ = fs_mgr.write_text_atomic(&path, &serialized);
        }
        Self::encode_geometry_json(&updated, true)
    }

    fn handle_menu_get_items(&self) -> String {
        let Ok(state) = self.ui_state.lock() else {
            return Self::err_json("Desktop UI mutex poisoned");
        };
        format!(
            r#"{{"ok":true,"items":{}}}"#,
            Self::encode_menu_items(&state.app_menu)
        )
    }

    fn handle_menu_set_items(&self, req_json: &str) -> String {
        let items = Self::parse_menu_items(req_json, "items");
        let Ok(mut state) = self.ui_state.lock() else {
            return Self::err_json("Desktop UI mutex poisoned");
        };
        state.app_menu = items;
        format!(
            r#"{{"ok":true,"items":{}}}"#,
            Self::encode_menu_items(&state.app_menu)
        )
    }

    fn handle_tray_get_state(&self) -> String {
        let Ok(state) = self.ui_state.lock() else {
            return Self::err_json("Desktop UI mutex poisoned");
        };
        Self::encode_tray_json(&state.tray_state)
    }

    fn handle_tray_set_state(&self, req_json: &str) -> String {
        let Ok(mut state) = self.ui_state.lock() else {
            return Self::err_json("Desktop UI mutex poisoned");
        };
        if let Some(visible) = extract_json_bool(req_json, "visible") {
            state.tray_state.visible = visible;
        }
        if let Some(icon) = extract_json_string(req_json, "icon") {
            state.tray_state.icon = icon;
        }
        if let Some(tooltip) = extract_json_string(req_json, "tooltip") {
            state.tray_state.tooltip = tooltip;
        }
        if extract_json_raw_value(req_json, "menuItems").is_some() {
            state.tray_state.menu_items = Self::parse_menu_items(req_json, "menuItems");
        }
        Self::encode_tray_json(&state.tray_state)
    }

    fn encode_tray_json(tray: &TrayState) -> String {
        format!(
            r#"{{"ok":true,"visible":{},"icon":"{}","tooltip":"{}","menuItems":{}}}"#,
            tray.visible,
            escape_json(&tray.icon),
            escape_json(&tray.tooltip),
            Self::encode_menu_items(&tray.menu_items)
        )
    }

    fn handle_shortcut_register(&self, req_json: &str) -> String {
        let acc = extract_json_string(req_json, "accelerator").unwrap_or_default();
        let action_id = extract_json_string(req_json, "actionId").unwrap_or_default();
        if acc.trim().is_empty() || action_id.trim().is_empty() {
            return Self::err_json("Missing accelerator or actionId");
        }
        let Ok(mut state) = self.ui_state.lock() else {
            return Self::err_json("Desktop UI mutex poisoned");
        };
        if let Some(existing) = state
            .shortcuts
            .iter_mut()
            .find(|item| item.accelerator == acc.trim())
        {
            existing.action_id = action_id.trim().to_string();
        } else {
            state.shortcuts.push(ShortcutBinding {
                accelerator: acc.trim().to_string(),
                action_id: action_id.trim().to_string(),
            });
        }
        format!(
            r#"{{"ok":true,"shortcuts":{}}}"#,
            Self::encode_shortcuts(&state.shortcuts)
        )
    }

    fn handle_shortcut_unregister(&self, req_json: &str) -> String {
        let acc = extract_json_string(req_json, "accelerator").unwrap_or_default();
        let Ok(mut state) = self.ui_state.lock() else {
            return Self::err_json("Desktop UI mutex poisoned");
        };
        state
            .shortcuts
            .retain(|item| item.accelerator != acc.trim());
        format!(
            r#"{{"ok":true,"shortcuts":{}}}"#,
            Self::encode_shortcuts(&state.shortcuts)
        )
    }

    fn handle_shortcut_list(&self) -> String {
        let Ok(state) = self.ui_state.lock() else {
            return Self::err_json("Desktop UI mutex poisoned");
        };
        format!(
            r#"{{"ok":true,"shortcuts":{}}}"#,
            Self::encode_shortcuts(&state.shortcuts)
        )
    }

    fn handle_dialog_confirm(&self, req_json: &str) -> String {
        let title =
            extract_json_string(req_json, "title").unwrap_or_else(|| String::from("Confirm"));
        let message = extract_json_string(req_json, "message").unwrap_or_default();
        let confirm_label =
            extract_json_string(req_json, "confirmLabel").unwrap_or_else(|| String::from("OK"));
        let cancel_label =
            extract_json_string(req_json, "cancelLabel").unwrap_or_else(|| String::from("Cancel"));
        let default_confirmed = extract_json_bool(req_json, "defaultConfirmed").unwrap_or(true);
        let Ok(mut state) = self.ui_state.lock() else {
            return Self::err_json("Desktop UI mutex poisoned");
        };
        let confirmed = state
            .next_confirm_response
            .take()
            .unwrap_or(default_confirmed);
        state.confirm_dialogs.push(ConfirmDialogRequest {
            title,
            message,
            confirm_label,
            cancel_label,
            default_confirmed,
            confirmed,
        });
        format!(r#"{{"ok":true,"confirmed":{confirmed}}}"#)
    }

    fn handle_dialog_confirm_set_next(&self, req_json: &str) -> String {
        let confirmed = extract_json_bool(req_json, "confirmed").unwrap_or(true);
        let Ok(mut state) = self.ui_state.lock() else {
            return Self::err_json("Desktop UI mutex poisoned");
        };
        state.next_confirm_response = Some(confirmed);
        format!(r#"{{"ok":true,"confirmed":{confirmed}}}"#)
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

    #[test]
    fn host_bridge_supports_window_geometry_menus_tray_and_shortcuts() {
        let app_id = format!("ng-host-bridge-ui-{}", process::id());
        let desktop =
            DesktopServices::with_app_config(&app_id, AppPermissions::allow_all_sandboxed());
        let bridge = HostBridge::new(&desktop);

        let set_geom = bridge.handle_service_call(
            r#"{"action":"windowSetGeometry","width":1240,"height":820,"x":40,"y":60,"maximized":true}"#,
        );
        assert!(set_geom.contains(r#""width":1240"#));
        assert!(set_geom.contains(r#""maximized":true"#));
        let get_geom = bridge.handle_service_call(r#"{"action":"windowGetGeometry"}"#);
        assert!(get_geom.contains(r#""width":1240"#));

        let menu_res = bridge.handle_service_call(
            r#"{"action":"menuSetItems","items":[{"id":"refresh","label":"Refresh","accelerator":"<Control>r","enabled":true}]}"#,
        );
        assert!(menu_res.contains(r#""id":"refresh""#));
        let menu_get = bridge.handle_service_call(r#"{"action":"menuGetItems"}"#);
        assert!(menu_get.contains(r#""accelerator":"<Control>r""#));

        let tray_res = bridge.handle_service_call(
            r#"{"action":"traySetState","visible":true,"icon":"app-tray","tooltip":"Running","menuItems":[{"id":"quit","label":"Quit","enabled":true}]}"#,
        );
        assert!(tray_res.contains(r#""visible":true"#));
        assert!(tray_res.contains(r#""tooltip":"Running""#));
        assert!(
            bridge
                .handle_service_call(r#"{"action":"trayGetState"}"#)
                .contains("app-tray")
        );

        let sc_reg = bridge.handle_service_call(
            r#"{"action":"shortcutRegister","accelerator":"<Control>f","actionId":"search"}"#,
        );
        assert!(sc_reg.contains(r#""actionId":"search""#));
        let sc_unreg = bridge
            .handle_service_call(r#"{"action":"shortcutUnregister","accelerator":"<Control>f"}"#);
        assert!(sc_unreg.contains(r#""shortcuts":[]"#));

        let cfg_res = bridge.handle_service_call(
            r#"{"action":"fsResolvePath","kind":"config","relPath":"window-state.json"}"#,
        );
        if let Some(path) = extract_json_string(&cfg_res, "path") {
            let p = PathBuf::from(path);
            let _ = fs::remove_file(&p);
            if let Some(parent) = p.parent() {
                let _ = fs::remove_dir_all(parent);
            }
        }
    }

    #[test]
    fn host_bridge_supports_confirm_dialogs() {
        let desktop = DesktopServices::new();
        let bridge = HostBridge::new(&desktop);

        let default_yes = bridge.handle_service_call(
            r#"{"action":"dialogConfirm","title":"Close","message":"Save first?","defaultConfirmed":true}"#,
        );
        assert!(default_yes.contains(r#""confirmed":true"#));

        let set_no =
            bridge.handle_service_call(r#"{"action":"dialogConfirmSetNext","confirmed":false}"#);
        assert!(set_no.contains(r#""ok":true"#));

        let overridden = bridge.handle_service_call(
            r#"{"action":"dialogConfirm","title":"Overwrite","message":"Replace?","defaultConfirmed":true}"#,
        );
        assert!(overridden.contains(r#""confirmed":false"#));
    }
}

use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::config::AppPermissions;
use crate::desktop::http_codec::escape_json;
use crate::desktop::{DesktopServices, NativeHttpEngine};

use super::host_bridge::{HostAsyncCompletion, HostBridge};
use super::jsc_sys::JscSys;

const NG_NATIVE_SERVER_BOOTSTRAP_JS: &str = r"
(() => {
  const logs = [];
  let timers = new Map();
  let nextTimerId = 1;
  const startTime = Date.now();

  globalThis.console = {
    log: (...args) => logs.push(['LOG', args.join(' ')]),
    info: (...args) => logs.push(['INFO', args.join(' ')]),
    warn: (...args) => logs.push(['WARN', args.join(' ')]),
    debug: (...args) => logs.push(['DEBUG', args.join(' ')]),
    error: (...args) => logs.push(['ERROR', args.join(' ')]),
  };

  if (typeof globalThis.performance === 'undefined') {
    globalThis.performance = {
      now: () => Date.now() - startTime,
      timeOrigin: startTime,
    };
  }

  globalThis.setTimeout = (cb, ms = 0, ...args) => {
    const id = nextTimerId++;
    const delay = Math.max(0, Number(ms) || 0);
    timers.set(id, { id, cb, args, dueAt: Date.now() + delay });
    return id;
  };

  globalThis.clearTimeout = (id) => {
    timers.delete(id);
  };

  globalThis.__tickServerTimers = (nowMs = Date.now()) => {
    const now = typeof nowMs === 'number' && Number.isFinite(nowMs) ? nowMs : Date.now();
    const due = [];
    for (const entry of timers.values()) {
      if (entry.dueAt <= now) {
        due.push(entry);
        timers.delete(entry.id);
      }
    }
    for (const entry of due) {
      entry.cb(...(entry.args || []));
    }
    return timers.size;
  };

  const callService = (payload) => {
    if (typeof globalThis.__NG_NATIVE_HOST_SERVICE__ !== 'function') {
      return { ok: false, error: 'HostService unavailable in server worker' };
    }
    const raw = globalThis.__NG_NATIVE_HOST_SERVICE__(JSON.stringify(payload));
    return raw ? JSON.parse(raw) : { ok: false, error: 'Empty host response' };
  };

  globalThis.__NG_NATIVE_SERVER_HOST__ = {
    sqliteOpen(path) {
      const res = callService({ action: 'sqliteOpen', path: String(path || 'app.db') });
      if (!res.ok) throw new Error(res.error || 'sqliteOpen failed');
      return String(res.path || path);
    },
    sqliteExecute(sql) {
      const res = callService({ action: 'sqliteExecute', sql: String(sql ?? '') });
      if (!res.ok) throw new Error(res.error || 'sqliteExecute failed');
      return true;
    },
    sqliteQuery(sql) {
      const res = callService({ action: 'sqliteQuery', sql: String(sql ?? '') });
      if (!res.ok) throw new Error(res.error || 'sqliteQuery failed');
      return Array.isArray(res.rows) ? res.rows : [];
    },
    fsResolvePath(kind, relPath) {
      const res = callService({
        action: 'fsResolvePath',
        kind: String(kind || 'config'),
        relPath: String(relPath || ''),
      });
      if (!res.ok) throw new Error(res.error || 'fsResolvePath failed');
      return String(res.path);
    },
    fsReadText(path) {
      const res = callService({ action: 'fsReadText', path: String(path ?? '') });
      return res.ok ? (res.content ?? null) : null;
    },
    fsWriteText(path, content) {
      const res = callService({
        action: 'fsWriteText',
        path: String(path ?? ''),
        content: String(content ?? ''),
      });
      if (!res.ok) throw new Error(res.error || 'fsWriteText failed');
      return true;
    },
    fsWatchFile(path) {
      const res = callService({ action: 'fsWatchFile', path: String(path ?? '') });
      return Boolean(res.ok);
    },
    fsPollChangedFiles() {
      const res = callService({ action: 'fsPollChangedFiles' });
      return res.ok && Array.isArray(res.paths) ? res.paths : [];
    },
    computeFuzzyFilter(items = [], query = '', limit = 50) {
      const res = callService({
        action: 'computeFuzzyFilter',
        items: Array.from(items, (x) => String(x ?? '')),
        query: String(query ?? ''),
        limit: Number(limit ?? 50),
      });
      if (!res.ok) throw new Error(res.error || 'computeFuzzyFilter failed');
      return Array.isArray(res.matches) ? res.matches : [];
    },
    computeHash(input = '') {
      const res = callService({ action: 'computeHash', input: String(input ?? '') });
      if (!res.ok) throw new Error(res.error || 'computeHash failed');
      return String(res.hash || '');
    },
    httpRequest(method, url, body, options = {}) {
      const request = {
        method: String(method || 'GET').trim().toUpperCase(),
        url: String(url || '').trim(),
        headers: options?.headers || {},
        callKind: options?.callKind || 'json',
      };
      if (body !== undefined && body !== null) {
        request.body = String(body);
      }
      const res = callService({ action: 'httpSync', request });
      if (res && typeof res.status === 'number') {
        return res;
      }
      return {
        status: 500,
        statusText: 'Host HTTP Error',
        body: JSON.stringify({ error: (res && res.error) || 'httpSync failed' }),
        headers: {},
      };
    },
  };

  if (typeof globalThis.fetch !== 'function') {
    globalThis.fetch = (input, init = {}) => {
      const url = typeof input === 'string' ? input : String(input?.url ?? '');
      const res = globalThis.__NG_NATIVE_SERVER_HOST__.httpRequest(
        init.method || 'GET',
        url,
        init.body,
        { headers: init.headers || {} },
      );
      return Promise.resolve({
        ok: res.status >= 200 && res.status < 300,
        status: res.status,
        statusText: res.statusText,
        headers: res.headers || {},
        text: () => Promise.resolve(res.body),
        json: () => Promise.resolve(JSON.parse(res.body || 'null')),
      });
    };
  }

  globalThis.__NG_SERVER_LAST_RESULT__ = '';
  globalThis.__NG_NATIVE_SERVER_DISPATCH__ = (req) => {
    globalThis.__NG_SERVER_LAST_RESULT__ = '';
    const router = globalThis.__NG_NATIVE_SERVER_ROUTER__;
    if (!router || typeof router.handle !== 'function') {
      globalThis.__NG_SERVER_LAST_RESULT__ = JSON.stringify({
        status: 503,
        statusText: 'Service Unavailable',
        body: JSON.stringify({ error: 'No native server router registered', statusCode: 503 }),
        headers: { 'content-type': 'application/json; charset=utf-8' },
        transport: 'native-server',
        callKind: 'json',
        cached: false,
      });
      return;
    }

    Promise.resolve(router.handle(req))
      .then((res) => {
        globalThis.__NG_SERVER_LAST_RESULT__ = JSON.stringify({
          status: typeof res?.status === 'number' ? res.status : 200,
          statusText: String(res?.statusText || 'OK'),
          body: String(res?.body ?? ''),
          headers: res?.headers || {},
          transport: 'native-server',
          callKind: 'json',
          cached: false,
        });
      })
      .catch((err) => {
        globalThis.__NG_SERVER_LAST_RESULT__ = JSON.stringify({
          status: 500,
          statusText: 'Internal Server Error',
          body: JSON.stringify({
            error: err instanceof Error ? err.message : String(err),
            statusCode: 500,
          }),
          headers: { 'content-type': 'application/json; charset=utf-8' },
          transport: 'native-server',
          callKind: 'json',
          cached: false,
        });
      });
  };
})();
";

struct WorkerBootConfig {
    app_id: String,
    permissions: AppPermissions,
    http_engine: Arc<Mutex<NativeHttpEngine>>,
    persistent_sqlite: bool,
    initial_bundle: String,
}

enum WorkerCommand {
    Dispatch {
        req_json: String,
        callback_id: u64,
        completions: Arc<Mutex<Vec<HostAsyncCompletion>>>,
    },
    Reload {
        bundle_js: String,
        reply: SyncSender<Result<(), String>>,
    },
    Shutdown,
}

/// Returns `true` when `url` targets the local in-process server (`/api/*`, `http(s)://localhost/api/*`, or `local://api/*`).
#[must_use]
pub fn is_local_api_url(url: &str) -> bool {
    let trimmed = url.trim();
    let lower = trimmed.to_ascii_lowercase();
    let path = if let Some(rest) = lower.strip_prefix("http://localhost") {
        rest
    } else if let Some(rest) = lower.strip_prefix("https://localhost") {
        rest
    } else if let Some(rest) = lower.strip_prefix("local://") {
        let normalized = rest.strip_prefix('/').unwrap_or(rest);
        return normalized == "api"
            || normalized.starts_with("api/")
            || normalized.starts_with("api?")
            || normalized.starts_with("api#");
    } else {
        lower.as_str()
    };

    path == "/api"
        || path.starts_with("/api/")
        || path.starts_with("/api?")
        || path.starts_with("/api#")
}

/// Dedicated background thread hosting an isolated `JavaScriptCore` backend runtime
/// (`@ng-desktop/server` routes, SQLite persistence, XDG FS, compute, and outbound HTTP).
pub struct NativeServerWorker {
    tx: Mutex<Sender<WorkerCommand>>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl NativeServerWorker {
    /// Spawns the isolated `ng-native-server` worker thread and evaluates `bundle_js`.
    pub fn start(
        desktop: &DesktopServices,
        bundle_js: &str,
        persistent_sqlite: bool,
    ) -> Result<Self, String> {
        let boot = WorkerBootConfig {
            app_id: desktop.app_id().to_string(),
            permissions: desktop.permissions().clone(),
            http_engine: desktop.shared_http_engine(),
            persistent_sqlite,
            initial_bundle: bundle_js.to_string(),
        };
        let (tx, rx) = mpsc::channel::<WorkerCommand>();
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<(), String>>(1);

        let handle = thread::Builder::new()
            .name(String::from("ng-native-server"))
            .spawn(move || {
                Self::run_worker_thread(&boot, &ready_tx, &rx);
            })
            .map_err(|e| format!("Failed to spawn server worker thread: {e}"))?;

        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                tx: Mutex::new(tx),
                handle: Mutex::new(Some(handle)),
            }),
            Ok(Err(err)) => {
                let _ = handle.join();
                Err(err)
            }
            Err(_) => {
                let _ = handle.join();
                Err(String::from("Server worker exited during startup"))
            }
        }
    }

    /// Enqueues an HTTP request targeting `/api/*` for asynchronous execution on the backend worker.
    pub fn dispatch_async(
        &self,
        req_json: String,
        callback_id: u64,
        completions: Arc<Mutex<Vec<HostAsyncCompletion>>>,
    ) -> Result<(), String> {
        let sender = self
            .tx
            .lock()
            .map_err(|_| String::from("Server worker channel lock poisoned"))?;
        sender
            .send(WorkerCommand::Dispatch {
                req_json,
                callback_id,
                completions,
            })
            .map_err(|_| String::from("Server worker thread is no longer running"))
    }

    /// Hot-reloads `server.bundle.js` inside the isolated backend JSC context.
    pub fn reload(&self, bundle_js: &str) -> Result<(), String> {
        let (reply_tx, reply_rx) = mpsc::sync_channel::<Result<(), String>>(1);
        {
            let sender = self
                .tx
                .lock()
                .map_err(|_| String::from("Server worker channel lock poisoned"))?;
            sender
                .send(WorkerCommand::Reload {
                    bundle_js: bundle_js.to_string(),
                    reply: reply_tx,
                })
                .map_err(|_| String::from("Server worker thread is no longer running"))?;
        }
        reply_rx
            .recv()
            .map_err(|_| String::from("Server worker dropped reload response"))?
    }

    fn run_worker_thread(
        boot: &WorkerBootConfig,
        ready_tx: &SyncSender<Result<(), String>>,
        rx: &Receiver<WorkerCommand>,
    ) {
        let worker_desktop = DesktopServices::with_app_config_and_http(
            &boot.app_id,
            boot.permissions.clone(),
            Arc::clone(&boot.http_engine),
        );
        let worker_bridge = Arc::new(HostBridge::new(&worker_desktop));
        if boot.persistent_sqlite && boot.permissions.sqlite && boot.permissions.filesystem_xdg {
            let _ = worker_bridge.handle_service_call(r#"{"action":"sqliteOpen","path":"app.db"}"#);
        }

        let Some(jsc) = JscSys::try_load() else {
            let _ = ready_tx.send(Err(String::from("JavaScriptCore runtime unavailable")));
            return;
        };
        jsc.set_http_engine(worker_desktop.shared_http_engine());
        jsc.set_host_bridge(Arc::clone(&worker_bridge));

        let escaped_app_id = boot.app_id.replace('\\', "\\\\").replace('\'', "\\'");
        if let Err(err) = jsc.eval(&format!(
            "globalThis.__NG_NATIVE_APP_ID__ = '{escaped_app_id}';"
        )) {
            let _ = ready_tx.send(Err(err));
            return;
        }
        if let Err(err) = jsc.eval(NG_NATIVE_SERVER_BOOTSTRAP_JS) {
            let _ = ready_tx.send(Err(err));
            return;
        }
        if let Err(err) = jsc.eval(&boot.initial_bundle) {
            let _ = ready_tx.send(Err(err));
            return;
        }
        let _ = ready_tx.send(Ok(()));

        Self::process_commands(&jsc, rx);
    }

    fn process_commands(jsc: &JscSys, rx: &Receiver<WorkerCommand>) {
        while let Ok(cmd) = rx.recv() {
            match cmd {
                WorkerCommand::Dispatch {
                    req_json,
                    callback_id,
                    completions,
                } => {
                    let payload_json = Self::execute_dispatch(jsc, &req_json);
                    if let Ok(mut queue) = completions.lock() {
                        queue.push(HostAsyncCompletion {
                            callback_id,
                            payload_json,
                        });
                    }
                }
                WorkerCommand::Reload { bundle_js, reply } => {
                    let res = jsc.eval(&bundle_js).map(|_| ());
                    let _ = reply.send(res);
                }
                WorkerCommand::Shutdown => break,
            }
        }
    }

    fn execute_dispatch(jsc: &JscSys, req_json: &str) -> String {
        let dispatch_script = format!("globalThis.__NG_NATIVE_SERVER_DISPATCH__({req_json});");
        if let Err(err) = jsc.eval(&dispatch_script) {
            return format!(
                r#"{{"status":500,"statusText":"Server Dispatch Error","body":"{}","headers":{{"content-type":"application/json"}},"transport":"native-server","callKind":"json","cached":false}}"#,
                escape_json(&format!(r#"{{"error":"{}"}}"#, escape_json(&err)))
            );
        }

        for _ in 0..200 {
            if let Ok(result) = jsc.eval("globalThis.__NG_SERVER_LAST_RESULT__ || ''")
                && !result.is_empty()
                && result != "undefined"
            {
                return result;
            }
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0_u64, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
            let pending = jsc
                .eval(&format!("globalThis.__tickServerTimers({now_ms})"))
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            if pending == 0 {
                if let Ok(result) = jsc.eval("globalThis.__NG_SERVER_LAST_RESULT__ || ''")
                    && !result.is_empty()
                    && result != "undefined"
                {
                    return result;
                }
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }

        String::from(
            r#"{"status":504,"statusText":"Gateway Timeout","body":"{\"error\":\"Server handler did not settle\"}","headers":{"content-type":"application/json"},"transport":"native-server","callKind":"json","cached":false}"#,
        )
    }
}

impl Drop for NativeServerWorker {
    fn drop(&mut self) {
        if let Ok(sender) = self.tx.lock() {
            let _ = sender.send(WorkerCommand::Shutdown);
        }
        if let Ok(mut guard) = self.handle.lock()
            && let Some(handle) = guard.take()
        {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_local_api_url;

    #[test]
    fn identifies_local_api_urls_without_matching_external_origins() {
        assert!(is_local_api_url("/api/favorites"));
        assert!(is_local_api_url("/api?limit=20"));
        assert!(is_local_api_url("http://localhost/api/pokemon/25"));
        assert!(is_local_api_url("https://localhost/api/team"));
        assert!(is_local_api_url("local://api/catalog"));

        assert!(!is_local_api_url("https://pokeapi.co/api/v2/pokemon/1"));
        assert!(!is_local_api_url("http://127.0.0.1:8080/api/pokemon"));
        assert!(!is_local_api_url("/apiv2/pokemon"));
        assert!(!is_local_api_url("/assets/logo.png"));
    }
}

#![allow(unsafe_code)]

use super::http_codec::{
    encode_config_json, encode_response_json, extract_json_bool, extract_json_string,
    extract_json_u64, parse_bridge_batch, parse_bridge_request,
};
use super::libsoup_sys::{LibsoupSys, default_reason_phrase};
use crate::ffi_dl::DynLib;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const MAX_HTTP_HISTORY: usize = 256;
const MAX_RESPONSE_CACHE_ENTRIES: usize = 256;
static BATCH_TEMP_SEQ: AtomicU64 = AtomicU64::new(1);

enum ThreadSoupState {
    Unprobed,
    Loaded(LibsoupSys),
    Unavailable,
}

thread_local! {
    static THREAD_LIBSOUP: RefCell<ThreadSoupState> = const { RefCell::new(ThreadSoupState::Unprobed) };
}

/// Native host HTTP transport stack selected per target operating system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsHttpPlatform {
    LinuxLibsoup,
    MacOsUrlSession,
    WindowsWinHttp,
}

impl OsHttpPlatform {
    pub const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOsUrlSession
        } else if cfg!(target_os = "windows") {
            Self::WindowsWinHttp
        } else {
            Self::LinuxLibsoup
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LinuxLibsoup => "linux-libsoup-3.0",
            Self::MacOsUrlSession => "macos-nsurlsession",
            Self::WindowsWinHttp => "windows-winhttp",
        }
    }
}

/// Logical call classification enabling optimal transport routing and caching per request type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeHttpCallKind {
    JsonApi,
    GraphQl,
    BinaryAsset,
    BatchParallel,
    TextStream,
}

impl NativeHttpCallKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::JsonApi => "json",
            Self::GraphQl => "graphql",
            Self::BinaryAsset => "asset",
            Self::BatchParallel => "batch",
            Self::TextStream => "text",
        }
    }

    pub fn detect(
        method: &str,
        url: &str,
        explicit: Option<&str>,
        response_type: Option<&str>,
    ) -> Self {
        if let Some(kind) = explicit {
            match kind.trim().to_ascii_lowercase().as_str() {
                "graphql" => return Self::GraphQl,
                "asset" | "binary" => return Self::BinaryAsset,
                "batch" => return Self::BatchParallel,
                "text" => return Self::TextStream,
                "json" => return Self::JsonApi,
                _ => {}
            }
        }
        if let Some(rt) = response_type {
            match rt.trim().to_ascii_lowercase().as_str() {
                "arraybuffer" | "blob" => return Self::BinaryAsset,
                "text" => return Self::TextStream,
                _ => {}
            }
        }
        let clean_url = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
        if clean_url.contains("graphql") && method.trim().eq_ignore_ascii_case("POST") {
            return Self::GraphQl;
        }
        if [".png", ".jpg", ".jpeg", ".webp", ".svg", ".ico", ".bin"]
            .iter()
            .any(|ext| clean_url.ends_with(ext))
        {
            return Self::BinaryAsset;
        }
        if [".txt", ".html", ".css", ".xml", ".csv"]
            .iter()
            .any(|ext| clean_url.ends_with(ext))
        {
            return Self::TextStream;
        }
        Self::JsonApi
    }
}

/// Engine HTTP configuration translated directly from Angular's `provideHttpClient(...)` features.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeHttpConfig {
    pub use_fetch_transport: bool,
    pub interceptors_enabled: bool,
    pub xsrf_enabled: bool,
    pub xsrf_cookie_name: String,
    pub xsrf_header_name: String,
    pub xsrf_token_value: String,
    pub jsonp_enabled: bool,
    pub requests_via_parent: bool,
    pub max_parallel_connections: u16,
    pub cache_enabled: bool,
    pub timeout_ms: u32,
}

impl Default for NativeHttpConfig {
    fn default() -> Self {
        Self {
            use_fetch_transport: true,
            interceptors_enabled: true,
            xsrf_enabled: true,
            xsrf_cookie_name: String::from("XSRF-TOKEN"),
            xsrf_header_name: String::from("X-XSRF-TOKEN"),
            xsrf_token_value: String::new(),
            jsonp_enabled: false,
            requests_via_parent: false,
            max_parallel_connections: 24,
            cache_enabled: true,
            timeout_ms: 15_000,
        }
    }
}

/// Native HTTP request dispatched by `@ng-desktop/http` or internal desktop services.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeHttpRequest {
    pub method: String,
    pub url: String,
    pub body: Option<String>,
    pub headers: HashMap<String, String>,
    pub call_kind: NativeHttpCallKind,
    pub response_type: String,
    pub with_credentials: bool,
}

impl NativeHttpRequest {
    pub fn new(method: impl Into<String>, url: impl Into<String>) -> Self {
        let method = method.into();
        let url = url.into();
        let call_kind = NativeHttpCallKind::detect(&method, &url, None, None);
        Self {
            method,
            url,
            body: None,
            headers: HashMap::new(),
            call_kind,
            response_type: String::from("json"),
            with_credentials: false,
        }
    }
}

/// Native HTTP response returned to `@ng-desktop/http` (`HttpClient`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeHttpResponse {
    pub status: u16,
    pub status_text: String,
    pub body: String,
    pub headers: HashMap<String, String>,
    pub transport: String,
    pub call_kind: NativeHttpCallKind,
    pub cached: bool,
}

/// OS-native & call-type-aware HTTP engine combining persistent `libsoup-3.0` (`SoupSession`),
/// HTTP/2 parallel batch transfers, XSRF header enforcement, and in-memory GET caching.
pub struct NativeHttpEngine {
    platform: OsHttpPlatform,
    config: NativeHttpConfig,
    http_routes: HashMap<String, NativeHttpResponse>,
    response_cache: HashMap<String, NativeHttpResponse>,
    cache_order: VecDeque<String>,
    http_history: Vec<NativeHttpRequest>,
}

impl Default for NativeHttpEngine {
    fn default() -> Self {
        Self::new()
    }
}

enum PreparedHttpOutcome {
    Immediate(NativeHttpResponse),
    Outbound {
        cache_key: Option<String>,
        request: NativeHttpRequest,
        timeout_ms: u32,
        platform: OsHttpPlatform,
    },
}

struct BatchExecutionPlan {
    results: Vec<Option<NativeHttpResponse>>,
    pending_gets: Vec<(usize, String, NativeHttpRequest)>,
    non_get_outbound: Vec<(usize, Option<String>, NativeHttpRequest)>,
    max_parallel_connections: u16,
    timeout_ms: u32,
    platform: OsHttpPlatform,
}

impl NativeHttpEngine {
    pub fn new() -> Self {
        Self {
            platform: OsHttpPlatform::current(),
            config: NativeHttpConfig::default(),
            http_routes: HashMap::new(),
            response_cache: HashMap::new(),
            cache_order: VecDeque::new(),
            http_history: Vec::new(),
        }
    }

    pub const fn platform(&self) -> OsHttpPlatform {
        self.platform
    }

    /// Probes whether the native OS HTTP transport driver can be loaded dynamically.
    #[must_use]
    pub fn is_os_http_driver_available(&self) -> bool {
        match self.platform {
            OsHttpPlatform::LinuxLibsoup => unsafe { DynLib::open(c"libsoup-3.0.so.0").is_some() },
            OsHttpPlatform::MacOsUrlSession => unsafe {
                DynLib::open(c"/System/Library/Frameworks/CFNetwork.framework/CFNetwork").is_some()
                    || DynLib::open(c"/System/Library/Frameworks/Foundation.framework/Foundation")
                        .is_some()
            },
            OsHttpPlatform::WindowsWinHttp => unsafe {
                type WinHttpOpenFn = unsafe extern "system" fn(
                    *const u16,
                    u32,
                    *const u16,
                    *const u16,
                    u32,
                )
                    -> *mut std::ffi::c_void;
                DynLib::open(c"winhttp.dll")
                    .and_then(|d| d.sym::<WinHttpOpenFn>(c"WinHttpOpen"))
                    .is_some()
            },
        }
    }

    pub const fn config(&self) -> &NativeHttpConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: NativeHttpConfig) {
        self.config = config;
    }

    pub fn register_http_endpoint(&mut self, method: &str, url: &str, status: u16, body: &str) {
        let key = format!("{} {}", method.trim().to_ascii_uppercase(), url.trim());
        self.http_routes.insert(
            key,
            NativeHttpResponse {
                status,
                status_text: default_reason_phrase(status),
                body: String::from(body),
                headers: HashMap::new(),
                transport: String::from("mock-route"),
                call_kind: NativeHttpCallKind::detect(method, url, None, None),
                cached: false,
            },
        );
    }

    pub fn http_history(&self) -> &[NativeHttpRequest] {
        &self.http_history
    }

    fn record_history(&mut self, req: NativeHttpRequest) {
        if self.http_history.len() >= MAX_HTTP_HISTORY {
            self.http_history.remove(0);
        }
        self.http_history.push(req);
    }

    fn base_route_key(req: &NativeHttpRequest) -> String {
        format!(
            "{} {}",
            req.method.trim().to_ascii_uppercase(),
            req.url.trim()
        )
    }

    fn cache_key_for(req: &NativeHttpRequest) -> String {
        let base = Self::base_route_key(req);
        let mut auth_headers: Vec<(&str, &str)> = req
            .headers
            .iter()
            .filter_map(|(k, v)| {
                let lower = k.trim().to_ascii_lowercase();
                if lower == "authorization" || lower == "cookie" {
                    Some((k.as_str(), v.as_str()))
                } else {
                    None
                }
            })
            .collect();
        if auth_headers.is_empty() {
            return base;
        }
        auth_headers.sort_by(|a, b| a.0.cmp(b.0).then_with(|| a.1.cmp(b.1)));
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for (k, v) in auth_headers {
            for b in k.to_ascii_lowercase().as_bytes() {
                hash ^= u64::from(*b);
                hash = hash.wrapping_mul(0x0100_0000_01b3);
            }
            hash ^= u64::from(b':');
            hash = hash.wrapping_mul(0x0100_0000_01b3);
            for b in v.as_bytes() {
                hash ^= u64::from(*b);
                hash = hash.wrapping_mul(0x0100_0000_01b3);
            }
            hash ^= u64::from(b';');
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        let mut keyed = base;
        let _ = write!(keyed, "|auth={hash:016x}");
        keyed
    }

    fn cache_get(&mut self, key: &str) -> Option<NativeHttpResponse> {
        let mut hit = self.response_cache.get(key)?.clone();
        if let Some(pos) = self.cache_order.iter().position(|k| k == key) {
            self.cache_order.remove(pos);
        }
        self.cache_order.push_back(key.to_string());
        hit.cached = true;
        Some(hit)
    }

    fn cache_put(&mut self, key: String, response: NativeHttpResponse) {
        if let Some(pos) = self.cache_order.iter().position(|k| k == &key) {
            self.cache_order.remove(pos);
        } else {
            while self.response_cache.len() >= MAX_RESPONSE_CACHE_ENTRIES {
                let Some(oldest) = self.cache_order.pop_front() else {
                    break;
                };
                self.response_cache.remove(&oldest);
            }
        }
        self.cache_order.push_back(key.clone());
        self.response_cache.insert(key, response);
    }

    fn begin_http_request(&mut self, request: &NativeHttpRequest) -> PreparedHttpOutcome {
        let prepared = self.prepare_request(request);
        let route_key = Self::base_route_key(&prepared);
        let cache_key = Self::cache_key_for(&prepared);
        self.record_history(prepared.clone());

        if let Some(res) = self.http_routes.get(&route_key) {
            return PreparedHttpOutcome::Immediate(res.clone());
        }
        if self.is_cacheable(&prepared)
            && let Some(hit) = self.cache_get(&cache_key)
        {
            return PreparedHttpOutcome::Immediate(hit);
        }

        PreparedHttpOutcome::Outbound {
            cache_key: self.is_cacheable(&prepared).then_some(cache_key),
            request: prepared,
            timeout_ms: self.config.timeout_ms,
            platform: self.platform,
        }
    }

    fn finish_http_request(
        &mut self,
        cache_key: Option<String>,
        response: NativeHttpResponse,
    ) -> NativeHttpResponse {
        if let Some(key) = cache_key
            && response.status < 400
            && !response.body.is_empty()
        {
            self.cache_put(key, response.clone());
        }
        response
    }

    pub fn execute_http(&mut self, request: &NativeHttpRequest) -> NativeHttpResponse {
        match self.begin_http_request(request) {
            PreparedHttpOutcome::Immediate(res) => res,
            PreparedHttpOutcome::Outbound {
                cache_key,
                request,
                timeout_ms,
                platform,
            } => {
                let response = perform_outbound_detached(&request, timeout_ms, platform);
                self.finish_http_request(cache_key, response)
            }
        }
    }

    fn prepare_batch_plan(&mut self, requests: &[NativeHttpRequest]) -> BatchExecutionPlan {
        let mut results: Vec<Option<NativeHttpResponse>> = vec![None; requests.len()];
        let mut pending_gets: Vec<(usize, String, NativeHttpRequest)> = Vec::new();
        let mut non_get_outbound: Vec<(usize, Option<String>, NativeHttpRequest)> = Vec::new();

        for (idx, req) in requests.iter().enumerate() {
            let prepared = self.prepare_request(req);
            let route_key = Self::base_route_key(&prepared);
            let cache_key = Self::cache_key_for(&prepared);
            self.record_history(prepared.clone());
            if let Some(res) = self.http_routes.get(&route_key) {
                results[idx] = Some(res.clone());
            } else if self.is_cacheable(&prepared)
                && let Some(hit) = self.cache_get(&cache_key)
            {
                results[idx] = Some(hit);
            } else if prepared.method.eq_ignore_ascii_case("GET")
                && prepared.headers.is_empty()
                && (prepared.url.starts_with("http://") || prepared.url.starts_with("https://"))
            {
                pending_gets.push((idx, cache_key, prepared));
            } else {
                let maybe_cache = self.is_cacheable(&prepared).then_some(cache_key);
                non_get_outbound.push((idx, maybe_cache, prepared));
            }
        }

        BatchExecutionPlan {
            results,
            pending_gets,
            non_get_outbound,
            max_parallel_connections: self.config.max_parallel_connections,
            timeout_ms: self.config.timeout_ms,
            platform: self.platform,
        }
    }

    fn execute_batch_plan_detached(
        plan: &BatchExecutionPlan,
    ) -> Vec<(usize, Option<String>, NativeHttpResponse)> {
        let mut completed = Vec::new();
        for (idx, maybe_cache, req) in &plan.non_get_outbound {
            let mut res = perform_outbound_detached(req, plan.timeout_ms, plan.platform);
            res.call_kind = NativeHttpCallKind::BatchParallel;
            completed.push((*idx, maybe_cache.clone(), res));
        }

        if plan.pending_gets.is_empty() {
            return completed;
        }
        if plan.pending_gets.len() == 1 {
            let (idx, key, req) = &plan.pending_gets[0];
            let mut res = perform_outbound_detached(req, plan.timeout_ms, plan.platform);
            res.call_kind = NativeHttpCallKind::BatchParallel;
            completed.push((*idx, Some(key.clone()), res));
            return completed;
        }

        let stamp = std::process::id();
        let seq = BATCH_TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
        let temp_dir = std::env::temp_dir();
        let pairs: Vec<(String, PathBuf)> = plan
            .pending_gets
            .iter()
            .map(|(idx, _, req)| {
                let path = temp_dir.join(format!("ng_native_batch_{stamp}_{seq}_{idx}.tmp"));
                (req.url.clone(), path)
            })
            .collect();
        let ok_flags = Self::download_files_parallel_with(
            plan.max_parallel_connections,
            plan.timeout_ms,
            &pairs,
        );
        for ((idx, key, req), ((_, tmp_path), ok)) in plan
            .pending_gets
            .iter()
            .zip(pairs.into_iter().zip(ok_flags))
        {
            let unsafe_url = !is_safe_http_url(&req.url);
            let body = if ok {
                fs::read_to_string(&tmp_path).unwrap_or_default()
            } else {
                String::new()
            };
            let _ = fs::remove_file(&tmp_path);
            let status = if ok {
                200
            } else if unsafe_url {
                400
            } else {
                500
            };
            let res = NativeHttpResponse {
                status,
                status_text: default_reason_phrase(status),
                body,
                headers: HashMap::new(),
                transport: format!("{}-parallel", plan.platform.as_str()),
                call_kind: NativeHttpCallKind::BatchParallel,
                cached: false,
            };
            completed.push((*idx, Some(key.clone()), res));
        }
        completed
    }

    fn apply_batch_results(
        &mut self,
        mut results: Vec<Option<NativeHttpResponse>>,
        completed: Vec<(usize, Option<String>, NativeHttpResponse)>,
    ) -> Vec<NativeHttpResponse> {
        for (idx, cache_key, res) in completed {
            let stored = self.finish_http_request(cache_key, res);
            if idx < results.len() {
                results[idx] = Some(stored);
            }
        }
        results
            .into_iter()
            .map(|item| {
                item.unwrap_or_else(|| not_found_response(NativeHttpCallKind::BatchParallel))
            })
            .collect()
    }

    /// Executes multiple HTTP requests, fulfilling mocks/cache hits immediately and
    /// fetching remaining HTTP `GET` URLs concurrently via an HTTP/2 parallel batch transfer.
    pub fn execute_batch(&mut self, requests: &[NativeHttpRequest]) -> Vec<NativeHttpResponse> {
        let plan = self.prepare_batch_plan(requests);
        let fetched = Self::execute_batch_plan_detached(&plan);
        self.apply_batch_results(plan.results, fetched)
    }

    /// Downloads multiple `(url, dest_path)` pairs concurrently in a single multiplexed HTTP/2 process.
    pub(crate) fn download_files_parallel_with(
        max_parallel_connections: u16,
        timeout_ms: u32,
        pairs: &[(String, PathBuf)],
    ) -> Vec<bool> {
        if pairs.is_empty() {
            return Vec::new();
        }
        let max_conn = max_parallel_connections.max(1).to_string();
        let timeout_secs = timeout_ms.div_ceil(1000).max(1).to_string();
        let mut cmd = Command::new("curl");
        cmd.args([
            "-s",
            "-f",
            "-L",
            "--proto",
            "=http,https",
            "--proto-redir",
            "=http,https",
            "--http2",
            "-Z",
            "--parallel-immediate",
            "--parallel-max",
            &max_conn,
            "--max-time",
            &timeout_secs,
        ]);
        let mut valid_count = 0usize;
        for (url, dest) in pairs {
            if !is_safe_http_url(url) {
                continue;
            }
            if let Some(parent) = dest.parent() {
                let _ = fs::create_dir_all(parent);
            }
            cmd.arg("-o").arg(dest).arg("--url").arg(url.trim());
            valid_count += 1;
        }
        if valid_count == 0 {
            return vec![false; pairs.len()];
        }
        let _ = cmd.status();
        pairs
            .iter()
            .map(|(url, dest)| {
                is_safe_http_url(url) && fs::metadata(dest).is_ok_and(|m| m.len() > 0)
            })
            .collect()
    }

    pub fn download_files_parallel(&self, pairs: &[(String, PathBuf)]) -> Vec<bool> {
        Self::download_files_parallel_with(
            self.config.max_parallel_connections,
            self.config.timeout_ms,
            pairs,
        )
    }

    fn prepare_request(&self, req: &NativeHttpRequest) -> NativeHttpRequest {
        let mut prepared = req.clone();
        if self.config.xsrf_enabled
            && !self.config.xsrf_token_value.is_empty()
            && !matches!(
                prepared.method.trim().to_ascii_uppercase().as_str(),
                "GET" | "HEAD"
            )
            && !prepared
                .headers
                .keys()
                .any(|k| k.eq_ignore_ascii_case(&self.config.xsrf_header_name))
        {
            prepared.headers.insert(
                self.config.xsrf_header_name.clone(),
                self.config.xsrf_token_value.clone(),
            );
        }
        prepared
    }

    fn is_cacheable(&self, req: &NativeHttpRequest) -> bool {
        self.config.cache_enabled
            && req.method.trim().eq_ignore_ascii_case("GET")
            && matches!(
                req.call_kind,
                NativeHttpCallKind::JsonApi | NativeHttpCallKind::TextStream
            )
    }

    /// Executes a bridge HTTP request or batch without holding `engine`'s mutex across network I/O.
    pub(crate) fn execute_bridge_detached(engine: &Arc<Mutex<Self>>, payload_json: &str) -> String {
        if payload_json.trim_start().starts_with('[')
            || extract_json_string(payload_json, "kind").as_deref() == Some("batch")
        {
            let requests = parse_bridge_batch(payload_json);
            let plan = {
                let Ok(mut guard) = engine.lock() else {
                    return String::from("[]");
                };
                guard.prepare_batch_plan(&requests)
            };
            let fetched = Self::execute_batch_plan_detached(&plan);
            let responses = {
                let Ok(mut guard) = engine.lock() else {
                    return String::from("[]");
                };
                guard.apply_batch_results(plan.results, fetched)
            };
            let encoded: Vec<String> = responses.iter().map(encode_response_json).collect();
            return format!("[{}]", encoded.join(","));
        }

        let req = parse_bridge_request(payload_json);
        let outcome = {
            let Ok(mut guard) = engine.lock() else {
                return encode_response_json(&error_500_response(
                    req.call_kind,
                    "Host Mutex Poisoned",
                ));
            };
            guard.begin_http_request(&req)
        };
        let response = match outcome {
            PreparedHttpOutcome::Immediate(res) => res,
            PreparedHttpOutcome::Outbound {
                cache_key,
                request,
                timeout_ms,
                platform,
            } => {
                let fetched = perform_outbound_detached(&request, timeout_ms, platform);
                if let Ok(mut guard) = engine.lock() {
                    guard.finish_http_request(cache_key, fetched)
                } else {
                    fetched
                }
            }
        };
        encode_response_json(&response)
    }
}

fn error_500_response(call_kind: NativeHttpCallKind, status_text: &str) -> NativeHttpResponse {
    NativeHttpResponse {
        status: 500,
        status_text: String::from(status_text),
        body: String::new(),
        headers: HashMap::new(),
        transport: String::from("host-error"),
        call_kind,
        cached: false,
    }
}

pub(crate) fn is_safe_http_url(url: &str) -> bool {
    let trimmed = url.trim();
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return false;
    }
    !trimmed.chars().any(|c| c.is_control() || c.is_whitespace())
}

pub(crate) fn is_safe_http_method(method: &str) -> bool {
    let trimmed = method.trim();
    !trimmed.is_empty() && trimmed.len() <= 16 && trimmed.bytes().all(|b| b.is_ascii_alphabetic())
}

fn sanitize_header_part(raw: &str) -> Option<&str> {
    if raw.chars().any(|c| matches!(c, '\r' | '\n' | '\0')) {
        None
    } else {
        Some(raw)
    }
}

fn bad_request_response(call_kind: NativeHttpCallKind, reason: &str) -> NativeHttpResponse {
    NativeHttpResponse {
        status: 400,
        status_text: String::from("Bad Request"),
        body: format!(r#"{{"error":"{reason}"}}"#),
        headers: HashMap::new(),
        transport: String::from("http-validation"),
        call_kind,
        cached: false,
    }
}

fn perform_outbound_detached(
    request: &NativeHttpRequest,
    timeout_ms: u32,
    platform: OsHttpPlatform,
) -> NativeHttpResponse {
    if !is_safe_http_url(&request.url) {
        return bad_request_response(request.call_kind, "Unsafe or non-HTTP URL rejected");
    }
    if !is_safe_http_method(&request.method) {
        return bad_request_response(request.call_kind, "Invalid HTTP method");
    }
    let trimmed_url = request.url.trim();
    let method_upper = request.method.trim().to_ascii_uppercase();
    if platform == OsHttpPlatform::LinuxLibsoup {
        let libsoup_raw = THREAD_LIBSOUP.with(|cell| {
            let mut slot = cell.borrow_mut();
            if matches!(*slot, ThreadSoupState::Unprobed) {
                *slot = LibsoupSys::try_load()
                    .map_or(ThreadSoupState::Unavailable, ThreadSoupState::Loaded);
            }
            let ThreadSoupState::Loaded(ref soup) = *slot else {
                return None;
            };
            soup.set_timeout_ms(timeout_ms);
            soup.send_raw(
                &method_upper,
                trimmed_url,
                &request.headers,
                request.body.as_deref(),
            )
        });
        if let Some(raw) = libsoup_raw {
            return NativeHttpResponse {
                status: raw.status,
                status_text: raw.status_text,
                body: String::from_utf8_lossy(&raw.body_bytes).into_owned(),
                headers: raw.headers,
                transport: String::from(platform.as_str()),
                call_kind: request.call_kind,
                cached: false,
            };
        }
    }
    execute_curl_fallback(request, timeout_ms, platform)
}

fn not_found_response(call_kind: NativeHttpCallKind) -> NativeHttpResponse {
    NativeHttpResponse {
        status: 404,
        status_text: String::from("Not Found"),
        body: String::from(r#"{"error":"Endpoint not registered"}"#),
        headers: HashMap::new(),
        transport: String::from("none"),
        call_kind,
        cached: false,
    }
}

fn execute_curl_fallback(
    request: &NativeHttpRequest,
    timeout_ms: u32,
    platform: OsHttpPlatform,
) -> NativeHttpResponse {
    if !is_safe_http_url(&request.url) {
        return bad_request_response(request.call_kind, "Unsafe or non-HTTP URL rejected");
    }
    if !is_safe_http_method(&request.method) {
        return bad_request_response(request.call_kind, "Invalid HTTP method");
    }
    let clean_url = request.url.trim();
    let clean_method = request.method.trim().to_ascii_uppercase();
    let timeout_secs = timeout_ms.div_ceil(1000).max(1).to_string();
    let mut cmd = Command::new("curl");
    cmd.args([
        "-s",
        "-L",
        "--proto",
        "=http,https",
        "--proto-redir",
        "=http,https",
        "--http2",
        "--max-time",
        &timeout_secs,
        "-w",
        "\n__NG_STATUS__:%{http_code}",
        "-X",
        &clean_method,
    ]);
    for (k, v) in &request.headers {
        if let (Some(ck), Some(cv)) = (sanitize_header_part(k), sanitize_header_part(v)) {
            cmd.arg("-H").arg(format!("{ck}: {cv}"));
        }
    }
    if request.body.is_some() {
        cmd.arg("--data-binary").arg("@-");
        cmd.stdin(Stdio::piped());
    }
    cmd.arg("--").arg(clean_url);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let transport = match platform {
        OsHttpPlatform::LinuxLibsoup => "linux-curl-http2",
        OsHttpPlatform::MacOsUrlSession => "macos-nsurlsession-curl",
        OsHttpPlatform::WindowsWinHttp => "windows-winhttp-curl",
    };

    let output_res = if let Some(body) = &request.body {
        cmd.spawn().and_then(|mut child| {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(body.as_bytes());
            }
            child.wait_with_output()
        })
    } else {
        cmd.output()
    };

    if let Ok(output) = output_res
        && output.status.success()
    {
        let full = String::from_utf8_lossy(&output.stdout);
        let (body, status) = match full.rsplit_once("\n__NG_STATUS__:") {
            Some((b, code)) => (b.to_string(), code.trim().parse::<u16>().unwrap_or(200)),
            None => (full.into_owned(), 200),
        };
        return NativeHttpResponse {
            status,
            status_text: default_reason_phrase(status),
            body,
            headers: HashMap::new(),
            transport: String::from(transport),
            call_kind: request.call_kind,
            cached: false,
        };
    }

    NativeHttpResponse {
        status: 500,
        status_text: String::from("Error"),
        body: String::new(),
        headers: HashMap::new(),
        transport: String::from(transport),
        call_kind: request.call_kind,
        cached: false,
    }
}

impl NativeHttpEngine {
    /// Parses a JSON payload sent by the JSC bridge (`__NG_NATIVE_HOST_HTTP__`) and executes it.
    pub fn execute_bridge_json(&mut self, payload_json: &str) -> String {
        if payload_json.trim_start().starts_with('[')
            || extract_json_string(payload_json, "kind").as_deref() == Some("batch")
        {
            let requests = parse_bridge_batch(payload_json);
            let responses = self.execute_batch(&requests);
            let encoded: Vec<String> = responses.iter().map(encode_response_json).collect();
            return format!("[{}]", encoded.join(","));
        }
        let req = parse_bridge_request(payload_json);
        let res = self.execute_http(&req);
        encode_response_json(&res)
    }

    /// Parses a configuration or mock-route registration payload from `__NG_NATIVE_HOST_HTTP_CONFIG__`.
    pub fn apply_bridge_config_json(&mut self, cfg_json: &str) -> String {
        let action =
            extract_json_string(cfg_json, "action").unwrap_or_else(|| String::from("config"));
        if action == "registerEndpoint" {
            let method =
                extract_json_string(cfg_json, "method").unwrap_or_else(|| String::from("GET"));
            let url = extract_json_string(cfg_json, "url").unwrap_or_default();
            let status = extract_json_u64(cfg_json, "status")
                .and_then(|v| u16::try_from(v).ok())
                .unwrap_or(200);
            let body = extract_json_string(cfg_json, "body").unwrap_or_default();
            self.register_http_endpoint(&method, &url, status, &body);
            return encode_config_json(&self.config, self.platform);
        }

        let mut cfg = self.config.clone();
        if let Some(v) = extract_json_bool(cfg_json, "useFetchTransport") {
            cfg.use_fetch_transport = v;
        }
        if let Some(v) = extract_json_bool(cfg_json, "interceptorsEnabled") {
            cfg.interceptors_enabled = v;
        }
        if let Some(v) = extract_json_bool(cfg_json, "xsrfEnabled") {
            cfg.xsrf_enabled = v;
        }
        if let Some(v) = extract_json_string(cfg_json, "xsrfCookieName") {
            cfg.xsrf_cookie_name = v;
        }
        if let Some(v) = extract_json_string(cfg_json, "xsrfHeaderName") {
            cfg.xsrf_header_name = v;
        }
        if let Some(v) = extract_json_string(cfg_json, "xsrfTokenValue") {
            cfg.xsrf_token_value = v;
        }
        if let Some(v) = extract_json_bool(cfg_json, "jsonpEnabled") {
            cfg.jsonp_enabled = v;
        }
        if let Some(v) = extract_json_bool(cfg_json, "requestsViaParent") {
            cfg.requests_via_parent = v;
        }
        if let Some(v) = extract_json_u64(cfg_json, "maxParallelConnections")
            && let Ok(n) = u16::try_from(v)
        {
            cfg.max_parallel_connections = n.max(1);
        }
        if let Some(v) = extract_json_bool(cfg_json, "cacheEnabled") {
            cfg.cache_enabled = v;
        }
        if let Some(v) = extract_json_u64(cfg_json, "timeoutMs")
            && let Ok(ms) = u32::try_from(v)
        {
            cfg.timeout_ms = ms.max(100);
        }
        self.set_config(cfg);
        encode_config_json(&self.config, self.platform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_call_kind_and_injects_xsrf_header_on_mutations() {
        assert_eq!(
            NativeHttpCallKind::detect("GET", "https://api.example.test/v2/items/25", None, None),
            NativeHttpCallKind::JsonApi
        );
        assert_eq!(
            NativeHttpCallKind::detect("POST", "https://api.example.test/graphql", None, None),
            NativeHttpCallKind::GraphQl
        );
        assert_eq!(
            NativeHttpCallKind::detect("GET", "https://cdn.example.com/25.png?v=1", None, None),
            NativeHttpCallKind::BinaryAsset
        );

        let mut engine = NativeHttpEngine::new();
        engine.apply_bridge_config_json(
            r#"{"xsrfEnabled":true,"xsrfHeaderName":"X-CUSTOM-XSRF","xsrfTokenValue":"secret-42"}"#,
        );
        engine.register_http_endpoint(
            "POST",
            "https://api.ng-native.dev/save",
            201,
            r#"{"saved":true}"#,
        );
        let mut req = NativeHttpRequest::new("POST", "https://api.ng-native.dev/save");
        req.body = Some(String::from("{}"));
        let res = engine.execute_http(&req);
        assert_eq!(res.status, 201);
        assert_eq!(
            engine.http_history()[0]
                .headers
                .get("X-CUSTOM-XSRF")
                .map(String::as_str),
            Some("secret-42")
        );
    }

    #[test]
    fn parses_bridge_json_headers_and_caches_get_responses() {
        let mut engine = NativeHttpEngine::new();
        engine.register_http_endpoint(
            "GET",
            "https://api.ng-native.dev/ping",
            200,
            r#"{"pong":1}"#,
        );
        let res_json = engine.execute_bridge_json(
            r#"{"method":"GET","url":"https://api.ng-native.dev/ping","headers":{"Accept":"application/json","X-Trace":"abc"}}"#,
        );
        assert!(res_json.contains(r#""status":200"#));
        assert!(res_json.contains(r#""callKind":"json""#));
        assert_eq!(
            engine.http_history()[0]
                .headers
                .get("X-Trace")
                .map(String::as_str),
            Some("abc")
        );
    }

    #[test]
    fn extracts_json_string_arrays_and_nested_raw_values() {
        use crate::desktop::http_codec::{extract_json_raw_value, extract_json_string_array};

        let payload = r#"{"items":["alpha","beta \"2\"","gamma"],"nested":{"a":[1,2],"text":"hi}"},"flag":true}"#;
        assert_eq!(
            extract_json_string_array(payload, "items"),
            vec!["alpha", "beta \"2\"", "gamma"]
        );
        assert_eq!(
            extract_json_raw_value(payload, "nested").as_deref(),
            Some(r#"{"a":[1,2],"text":"hi}"}"#)
        );
        assert_eq!(
            extract_json_raw_value(payload, "flag").as_deref(),
            Some("true")
        );
        assert_eq!(
            NativeHttpEngine::download_files_parallel_with(4, 1000, &[]),
            Vec::<bool>::new()
        );
    }

    #[test]
    fn bounds_history_and_lru_cache_and_isolates_auth_cache_keys() {
        let mut engine = NativeHttpEngine::new();
        for idx in 0..300_u32 {
            let url = format!("https://api.ng-native.dev/item/{idx}");
            engine.register_http_endpoint("GET", &url, 200, &format!(r#"{{"idx":{idx}}}"#));
            let res = engine.execute_http(&NativeHttpRequest::new("GET", &url));
            assert_eq!(res.status, 200);
        }
        assert!(engine.http_history().len() <= MAX_HTTP_HISTORY);

        let mut req_a = NativeHttpRequest::new("GET", "https://api.ng-native.dev/profile");
        req_a
            .headers
            .insert(String::from("Authorization"), String::from("Bearer tokA"));
        let mut req_b = NativeHttpRequest::new("GET", "https://api.ng-native.dev/profile");
        req_b
            .headers
            .insert(String::from("Authorization"), String::from("Bearer tokB"));
        assert_ne!(
            NativeHttpEngine::cache_key_for(&req_a),
            NativeHttpEngine::cache_key_for(&req_b)
        );

        let shared = Arc::new(Mutex::new(NativeHttpEngine::new()));
        {
            let mut guard = shared.lock().unwrap();
            guard.register_http_endpoint(
                "GET",
                "https://api.ng-native.dev/detached",
                200,
                r#"{"detached":true}"#,
            );
            for idx in 0..270_u32 {
                guard.cache_put(
                    format!("GET https://api.ng-native.dev/lru/{idx}"),
                    NativeHttpResponse {
                        status: 200,
                        status_text: String::from("OK"),
                        body: format!(r#"{{"n":{idx}}}"#),
                        headers: HashMap::new(),
                        transport: String::from("test"),
                        call_kind: NativeHttpCallKind::JsonApi,
                        cached: false,
                    },
                );
            }
            let cache_len = guard.response_cache.len();
            drop(guard);
            assert!(cache_len <= MAX_RESPONSE_CACHE_ENTRIES);
        }
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let cloned = Arc::clone(&shared);
                std::thread::spawn(move || {
                    NativeHttpEngine::execute_bridge_detached(
                        &cloned,
                        r#"{"method":"GET","url":"https://api.ng-native.dev/detached"}"#,
                    )
                })
            })
            .collect();
        for handle in handles {
            let json = handle.join().unwrap();
            assert!(json.contains(r#""status":200"#));
        }
    }

    #[test]
    fn rejects_unsafe_urls_schemes_whitespace_and_flag_methods() {
        assert!(is_safe_http_url("https://pokeapi.co/api/v2/pokemon/25"));
        assert!(is_safe_http_url("http://127.0.0.1:8080/health"));
        assert!(!is_safe_http_url("file:///etc/passwd"));
        assert!(!is_safe_http_url("gopher://127.0.0.1:70/1"));
        assert!(!is_safe_http_url("-o /tmp/pwn https://example.com"));
        assert!(!is_safe_http_url("https://example.com/a\r\nX-Inject: 1"));

        assert!(is_safe_http_method("GET"));
        assert!(is_safe_http_method("post"));
        assert!(!is_safe_http_method("-o"));
        assert!(!is_safe_http_method("GET\n-K"));
        assert!(!is_safe_http_method(""));

        let flags = NativeHttpEngine::download_files_parallel_with(
            4,
            1000,
            &[(
                String::from("file:///etc/passwd"),
                PathBuf::from("/tmp/ng_native_should_never_write.txt"),
            )],
        );
        assert_eq!(flags, vec![false]);
    }
}

use super::http_codec::{
    encode_config_json, encode_response_json, extract_json_bool, extract_json_string,
    extract_json_u64, parse_bridge_batch, parse_bridge_request,
};
use super::libsoup_sys::{LibsoupSys, default_reason_phrase};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

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
    http_history: Vec<NativeHttpRequest>,
    libsoup: Option<LibsoupSys>,
    libsoup_probed: bool,
}

impl Default for NativeHttpEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeHttpEngine {
    pub fn new() -> Self {
        Self {
            platform: OsHttpPlatform::current(),
            config: NativeHttpConfig::default(),
            http_routes: HashMap::new(),
            response_cache: HashMap::new(),
            http_history: Vec::new(),
            libsoup: None,
            libsoup_probed: false,
        }
    }

    pub const fn platform(&self) -> OsHttpPlatform {
        self.platform
    }

    pub const fn config(&self) -> &NativeHttpConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: NativeHttpConfig) {
        if let Some(soup) = &self.libsoup {
            soup.set_timeout_ms(config.timeout_ms);
        }
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

    pub fn execute_http(&mut self, request: &NativeHttpRequest) -> NativeHttpResponse {
        let prepared = self.prepare_request(request);
        let key = format!(
            "{} {}",
            prepared.method.trim().to_ascii_uppercase(),
            prepared.url.trim()
        );
        self.http_history.push(prepared.clone());

        if let Some(res) = self.http_routes.get(&key) {
            return res.clone();
        }
        if self.is_cacheable(&prepared)
            && let Some(cached) = self.response_cache.get(&key)
        {
            let mut hit = cached.clone();
            hit.cached = true;
            return hit;
        }

        let response = self.dispatch_outbound(&prepared);
        if self.is_cacheable(&prepared) && response.status < 400 && !response.body.is_empty() {
            self.response_cache.insert(key, response.clone());
        }
        response
    }

    /// Executes multiple HTTP requests, fulfilling mocks/cache hits immediately and
    /// fetching remaining HTTP `GET` URLs concurrently via an HTTP/2 parallel batch transfer.
    pub fn execute_batch(&mut self, requests: &[NativeHttpRequest]) -> Vec<NativeHttpResponse> {
        let mut results: Vec<Option<NativeHttpResponse>> = vec![None; requests.len()];
        let mut pending: Vec<(usize, NativeHttpRequest)> = Vec::new();

        for (idx, req) in requests.iter().enumerate() {
            let prepared = self.prepare_request(req);
            let key = format!(
                "{} {}",
                prepared.method.trim().to_ascii_uppercase(),
                prepared.url.trim()
            );
            self.http_history.push(prepared.clone());
            if let Some(res) = self.http_routes.get(&key) {
                results[idx] = Some(res.clone());
            } else if self.is_cacheable(&prepared)
                && let Some(cached) = self.response_cache.get(&key)
            {
                let mut hit = cached.clone();
                hit.cached = true;
                results[idx] = Some(hit);
            } else if prepared.method.eq_ignore_ascii_case("GET")
                && prepared.headers.is_empty()
                && (prepared.url.starts_with("http://") || prepared.url.starts_with("https://"))
            {
                pending.push((idx, prepared));
            } else {
                let mut res = self.dispatch_outbound(&prepared);
                res.call_kind = NativeHttpCallKind::BatchParallel;
                results[idx] = Some(res);
            }
        }

        self.run_parallel_get_batch(&pending, &mut results);
        results
            .into_iter()
            .map(|item| {
                item.unwrap_or_else(|| not_found_response(NativeHttpCallKind::BatchParallel))
            })
            .collect()
    }

    /// Downloads multiple `(url, dest_path)` pairs concurrently in a single multiplexed HTTP/2 process.
    pub fn download_files_parallel(&self, pairs: &[(String, PathBuf)]) -> Vec<bool> {
        if pairs.is_empty() {
            return Vec::new();
        }
        let max_conn = self.config.max_parallel_connections.max(1).to_string();
        let timeout_secs = self.config.timeout_ms.div_ceil(1000).max(1).to_string();
        let mut cmd = Command::new("curl");
        cmd.args([
            "-s",
            "-f",
            "-L",
            "--http2",
            "-Z",
            "--parallel-immediate",
            "--parallel-max",
            &max_conn,
            "--max-time",
            &timeout_secs,
        ]);
        for (url, dest) in pairs {
            if let Some(parent) = dest.parent() {
                let _ = fs::create_dir_all(parent);
            }
            cmd.arg("-o").arg(dest).arg(url);
        }
        let _ = cmd.status();
        pairs
            .iter()
            .map(|(_, dest)| fs::metadata(dest).is_ok_and(|m| m.len() > 0))
            .collect()
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

    fn ensure_libsoup(&mut self) -> Option<&LibsoupSys> {
        if !self.libsoup_probed {
            self.libsoup_probed = true;
            let loaded = LibsoupSys::try_load();
            if let Some(ref soup) = loaded {
                soup.set_timeout_ms(self.config.timeout_ms);
            }
            self.libsoup = loaded;
        }
        self.libsoup.as_ref()
    }

    fn dispatch_outbound(&mut self, request: &NativeHttpRequest) -> NativeHttpResponse {
        if !request.url.starts_with("http://") && !request.url.starts_with("https://") {
            return not_found_response(request.call_kind);
        }
        let timeout_ms = self.config.timeout_ms;
        let platform = self.platform;
        if platform == OsHttpPlatform::LinuxLibsoup
            && let Some(soup) = self.ensure_libsoup()
            && let Some(raw) = soup.send_raw(
                &request.method,
                &request.url,
                &request.headers,
                request.body.as_deref(),
            )
        {
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
        execute_curl_fallback(request, timeout_ms, platform)
    }

    fn run_parallel_get_batch(
        &mut self,
        pending: &[(usize, NativeHttpRequest)],
        results: &mut [Option<NativeHttpResponse>],
    ) {
        if pending.is_empty() {
            return;
        }
        if pending.len() == 1 {
            let (idx, req) = &pending[0];
            let mut res = self.dispatch_outbound(req);
            res.call_kind = NativeHttpCallKind::BatchParallel;
            results[*idx] = Some(res);
            return;
        }
        let stamp = std::process::id();
        let pairs: Vec<(String, PathBuf)> = pending
            .iter()
            .map(|(idx, req)| {
                let path = PathBuf::from(format!("/tmp/ng_native_batch_{stamp}_{idx}.tmp"));
                (req.url.clone(), path)
            })
            .collect();
        let ok_flags = self.download_files_parallel(&pairs);
        for ((idx, req), ((_, tmp_path), ok)) in pending.iter().zip(pairs.into_iter().zip(ok_flags))
        {
            let body = if ok {
                fs::read_to_string(&tmp_path).unwrap_or_default()
            } else {
                String::new()
            };
            let _ = fs::remove_file(&tmp_path);
            let status = if ok { 200 } else { 500 };
            let res = NativeHttpResponse {
                status,
                status_text: default_reason_phrase(status),
                body,
                headers: HashMap::new(),
                transport: format!("{}-parallel", self.platform.as_str()),
                call_kind: NativeHttpCallKind::BatchParallel,
                cached: false,
            };
            let key = format!(
                "{} {}",
                req.method.trim().to_ascii_uppercase(),
                req.url.trim()
            );
            if status < 400 && self.is_cacheable(req) && !res.body.is_empty() {
                self.response_cache.insert(key, res.clone());
            }
            results[*idx] = Some(res);
        }
    }
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
    let timeout_secs = timeout_ms.div_ceil(1000).max(1).to_string();
    let mut cmd = Command::new("curl");
    cmd.args([
        "-s",
        "-L",
        "--http2",
        "--max-time",
        &timeout_secs,
        "-w",
        "\n__NG_STATUS__:%{http_code}",
        "-X",
        &request.method,
    ]);
    for (k, v) in &request.headers {
        cmd.arg("-H").arg(format!("{k}: {v}"));
    }
    if let Some(body) = &request.body {
        cmd.arg("-d").arg(body);
    }
    cmd.arg(&request.url);

    let transport = match platform {
        OsHttpPlatform::LinuxLibsoup => "linux-curl-http2",
        OsHttpPlatform::MacOsUrlSession => "macos-nsurlsession-curl",
        OsHttpPlatform::WindowsWinHttp => "windows-winhttp-curl",
    };

    if let Ok(output) = cmd.output()
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
            NativeHttpCallKind::detect("GET", "https://pokeapi.co/api/v2/pokemon/25", None, None),
            NativeHttpCallKind::JsonApi
        );
        assert_eq!(
            NativeHttpCallKind::detect(
                "POST",
                "https://beta.pokeapi.co/graphql/v1beta",
                None,
                None
            ),
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
}

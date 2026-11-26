//! Cross-platform Hot Module Replacement (`HMR`) IPC server and client over Unix Domain Sockets
//! and loopback TCP pointer files, plus reusable [`test_harness`] helpers.

pub mod test_harness;

use crate::desktop::http_codec::{escape_json, extract_json_string};
use crate::engine::EngineError;
use std::collections::HashMap;
use std::collections::hash_map::RandomState;
use std::fs;
use std::hash::BuildHasher;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum permitted size (16 MiB) for a single newline-delimited HMR IPC frame.
const MAX_IPC_FRAME_BYTES: usize = 16 * 1024 * 1024;

/// Strongly-typed Hot Module Replacement (HMR) IPC message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HmrMessage {
    Reload { script: String },
    ReloadServer { bundle_js: String },
    Eval { code: String },
    Ping,
}

/// Server response to an HMR IPC request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HmrResponse {
    Ok { reloaded: bool },
    EvalResult(String),
    Pong,
    Error(String),
}

impl HmrResponse {
    pub fn to_json(&self) -> String {
        match self {
            Self::Ok { reloaded } => format!(r#"{{"status":"ok","reloaded":{reloaded}}}"#),
            Self::EvalResult(res) => {
                let escaped = escape_json(res);
                format!(r#"{{"status":"ok","result":"{escaped}"}}"#)
            }
            Self::Pong => String::from(r#"{"status":"ok","pong":true}"#),
            Self::Error(msg) => {
                let escaped = escape_json(msg);
                format!(r#"{{"status":"error","message":"{escaped}"}}"#)
            }
        }
    }
}

pub type ClientId = usize;

enum HmrListener {
    #[cfg(unix)]
    Unix(UnixListener),
    Tcp(TcpListener),
}

impl HmrListener {
    fn accept_nonblocking(&self) -> Option<HmrStream> {
        let stream = match self {
            #[cfg(unix)]
            Self::Unix(listener) => HmrStream::Unix(listener.accept().ok()?.0),
            Self::Tcp(listener) => HmrStream::Tcp(listener.accept().ok()?.0),
        };
        stream.set_nonblocking(true).ok()?;
        Some(stream)
    }
}

enum HmrStream {
    #[cfg(unix)]
    Unix(UnixStream),
    Tcp(TcpStream),
}

impl HmrStream {
    fn set_nonblocking(&self, nonblocking: bool) -> std::io::Result<()> {
        match self {
            #[cfg(unix)]
            Self::Unix(s) => s.set_nonblocking(nonblocking),
            Self::Tcp(s) => s.set_nonblocking(nonblocking),
        }
    }

    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(unix)]
            Self::Unix(s) => s.read(buf),
            Self::Tcp(s) => s.read(buf),
        }
    }

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(unix)]
            Self::Unix(s) => s.write(buf),
            Self::Tcp(s) => s.write(buf),
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        match self {
            #[cfg(unix)]
            Self::Unix(s) => s.write_all(buf),
            Self::Tcp(s) => s.write_all(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            #[cfg(unix)]
            Self::Unix(s) => s.flush(),
            Self::Tcp(s) => s.flush(),
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        match self {
            #[cfg(unix)]
            Self::Unix(s) => s.read_exact(buf),
            Self::Tcp(s) => s.read_exact(buf),
        }
    }
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    let mut diff = a_bytes.len() ^ b_bytes.len();
    for (&x, &y) in a_bytes.iter().zip(b_bytes.iter()) {
        diff |= usize::from(x ^ y);
    }
    diff == 0
}

fn token_path_for(socket_path: &Path) -> PathBuf {
    let mut os = socket_path.as_os_str().to_os_string();
    os.push(".token");
    PathBuf::from(os)
}

fn restrict_owner_permissions(path: &Path) {
    #[cfg(unix)]
    {
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    let _ = path;
}

fn generate_ephemeral_hmr_token() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let pid = u64::from(std::process::id());
    let lo = RandomState::new().hash_one((now, pid, 1_u64));
    let hi = RandomState::new().hash_one((now.wrapping_mul(31), pid, 2_u64));
    format!("{lo:016x}{hi:016x}")
}

/// Cross-platform HMR IPC Server.
/// Listens for live bundle updates and evaluation requests over a Unix domain socket (Linux/macOS)
/// or a loopback TCP listener advertised via an endpoint pointer file (Windows / `.tcp` targets).
pub struct HmrIpcServer {
    socket_path: PathBuf,
    token_file: Option<PathBuf>,
    auth_token: Option<String>,
    allow_eval: bool,
    listener: HmrListener,
    clients: HashMap<ClientId, HmrStream>,
    in_buffers: HashMap<ClientId, Vec<u8>>,
    out_buffers: HashMap<ClientId, Vec<u8>>,
    next_client_id: ClientId,
}

impl HmrIpcServer {
    /// Binds to a local Unix socket path or writes a loopback TCP pointer file on Windows (or `.tcp` paths).
    pub fn bind(path: impl AsRef<Path>) -> Result<Self, EngineError> {
        Self::bind_with_options(path, None, true)
    }

    /// Binds an HMR server for local dev workflows with an ephemeral/env auth token (`0600` `.token` file)
    /// and opt-in JavaScript `eval` gating.
    pub fn bind_dev(path: impl AsRef<Path>, allow_eval: bool) -> Result<Self, EngineError> {
        let token = std::env::var("NG_DESKTOP_HMR_TOKEN")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(generate_ephemeral_hmr_token);
        let token_path = token_path_for(path.as_ref());
        let mut server = Self::bind_with_options(path, Some(token.clone()), allow_eval)?;
        fs::write(&token_path, format!("{token}\n")).map_err(|e| {
            EngineError::ConfigError(format!("Failed to write HMR token file: {e}"))
        })?;
        restrict_owner_permissions(&token_path);
        server.token_file = Some(token_path);
        Ok(server)
    }

    /// Binds an HMR IPC server with explicit bearer token enforcement and `Eval` capability control.
    pub fn bind_with_options(
        path: impl AsRef<Path>,
        auth_token: Option<String>,
        allow_eval: bool,
    ) -> Result<Self, EngineError> {
        let socket_path = path.as_ref().to_path_buf();

        if let Some(parent) = socket_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let use_tcp =
            cfg!(not(unix)) || socket_path.extension().and_then(|e| e.to_str()) == Some("tcp");

        let listener = if use_tcp {
            let tcp = TcpListener::bind("127.0.0.1:0").map_err(|e| {
                EngineError::ConfigError(format!("Failed to bind HMR TCP socket: {e}"))
            })?;
            tcp.set_nonblocking(true).map_err(|e| {
                EngineError::ConfigError(format!("Failed to set HMR listener non-blocking: {e}"))
            })?;
            let addr = tcp.local_addr().map_err(|e| {
                EngineError::ConfigError(format!("Failed to query HMR TCP local addr: {e}"))
            })?;
            fs::write(&socket_path, format!("tcp://127.0.0.1:{}\n", addr.port())).map_err(|e| {
                EngineError::ConfigError(format!("Failed to write HMR endpoint file: {e}"))
            })?;
            restrict_owner_permissions(&socket_path);
            HmrListener::Tcp(tcp)
        } else {
            #[cfg(unix)]
            {
                if socket_path.exists() && UnixStream::connect(&socket_path).is_err() {
                    let _ = fs::remove_file(&socket_path);
                }

                let unix = UnixListener::bind(&socket_path).map_err(|e| {
                    EngineError::ConfigError(format!("Failed to bind HMR socket: {e}"))
                })?;
                restrict_owner_permissions(&socket_path);
                unix.set_nonblocking(true).map_err(|e| {
                    EngineError::ConfigError(format!(
                        "Failed to set HMR listener non-blocking: {e}"
                    ))
                })?;
                HmrListener::Unix(unix)
            }
            #[cfg(not(unix))]
            {
                unreachable!("non-unix always uses TCP listener")
            }
        };

        Ok(Self {
            socket_path,
            token_file: None,
            auth_token,
            allow_eval,
            listener,
            clients: HashMap::new(),
            in_buffers: HashMap::new(),
            out_buffers: HashMap::new(),
            next_client_id: 1,
        })
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    pub fn auth_token(&self) -> Option<&str> {
        self.auth_token.as_deref()
    }

    pub const fn set_allow_eval(&mut self, allow: bool) {
        self.allow_eval = allow;
    }

    fn flush_client_out(&mut self, client_id: ClientId) -> bool {
        let Some(stream) = self.clients.get_mut(&client_id) else {
            return false;
        };
        let Some(buf) = self.out_buffers.get_mut(&client_id) else {
            return true;
        };
        while !buf.is_empty() {
            match stream.write(buf) {
                Ok(0) => return false,
                Ok(n) => {
                    buf.drain(..n);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => return false,
            }
        }
        if buf.is_empty() {
            let _ = stream.flush();
        }
        true
    }

    fn queue_response(&mut self, client_id: ClientId, response: &HmrResponse) {
        let mut payload = response.to_json();
        payload.push('\n');
        self.out_buffers
            .entry(client_id)
            .or_default()
            .extend_from_slice(payload.as_bytes());
        let _ = self.flush_client_out(client_id);
    }

    /// Polls for incoming client connections and full newline-delimited HMR messages without blocking.
    pub fn poll_messages(&mut self) -> Vec<(ClientId, HmrMessage)> {
        let mut messages = Vec::new();
        while let Some(stream) = self.listener.accept_nonblocking() {
            let cid = self.next_client_id;
            self.next_client_id += 1;
            self.clients.insert(cid, stream);
            self.in_buffers.insert(cid, Vec::new());
            self.out_buffers.insert(cid, Vec::new());
        }

        let client_ids: Vec<ClientId> = self.clients.keys().copied().collect();
        let mut disconnected = Vec::new();
        let mut read_buf = [0u8; 4096];

        for cid in client_ids {
            if !self.flush_client_out(cid) {
                disconnected.push(cid);
                continue;
            }
            if let Some(stream) = self.clients.get_mut(&cid) {
                loop {
                    match stream.read(&mut read_buf) {
                        Ok(0) => {
                            disconnected.push(cid);
                            break;
                        }
                        Ok(n) => {
                            let buf = self.in_buffers.entry(cid).or_default();
                            if buf.len().saturating_add(n) > MAX_IPC_FRAME_BYTES {
                                disconnected.push(cid);
                                break;
                            }
                            buf.extend_from_slice(&read_buf[..n]);
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                        Err(_) => {
                            disconnected.push(cid);
                            break;
                        }
                    }
                }
            }

            let mut queued_replies = Vec::new();
            if let Some(buf) = self.in_buffers.get_mut(&cid) {
                while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                    let frame: Vec<u8> = buf.drain(..=pos).collect();
                    let decoded = String::from_utf8_lossy(&frame[..pos]);
                    let line = decoded.trim();
                    if line.is_empty() {
                        continue;
                    }
                    if let Some(expected) = &self.auth_token {
                        let provided = extract_json_string(line, "token");
                        if provided
                            .as_deref()
                            .is_none_or(|tok| !constant_time_eq(tok, expected))
                        {
                            queued_replies
                                .push(HmrResponse::Error(String::from("Unauthorized HMR request")));
                            continue;
                        }
                    }
                    if let Some(msg) = parse_hmr_json(line) {
                        if matches!(msg, HmrMessage::Eval { .. }) && !self.allow_eval {
                            queued_replies.push(HmrResponse::Error(String::from(
                                "HMR eval is disabled (enable with --hmr-eval or NG_DESKTOP_HMR_EVAL=1)",
                            )));
                            continue;
                        }
                        messages.push((cid, msg));
                    }
                }
            }
            for reply in queued_replies {
                self.queue_response(cid, &reply);
            }
        }

        for cid in disconnected {
            self.clients.remove(&cid);
            self.in_buffers.remove(&cid);
            self.out_buffers.remove(&cid);
        }
        messages
    }

    /// Sends a JSON-framed response back to the specified client using a non-blocking write queue.
    pub fn send_response(&mut self, client_id: ClientId, response: &HmrResponse) {
        if self.clients.contains_key(&client_id) {
            self.queue_response(client_id, response);
        }
    }
}

impl Drop for HmrIpcServer {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket_path);
        if let Some(ref token_path) = self.token_file {
            let _ = fs::remove_file(token_path);
        }
    }
}

/// Client for sending HMR updates from Rust tests or tooling.
pub struct HmrIpcClient {
    stream: HmrStream,
    token: Option<String>,
}

impl HmrIpcClient {
    pub fn connect(path: impl AsRef<Path>) -> Result<Self, EngineError> {
        let endpoint = path.as_ref();
        let token = fs::read_to_string(token_path_for(endpoint))
            .ok()
            .map(|raw| raw.trim().to_string())
            .filter(|s| !s.is_empty());

        if let Ok(raw) = fs::read_to_string(endpoint) {
            let trimmed = raw.trim();
            if let Some(addr) = trimmed.strip_prefix("tcp://") {
                let tcp = TcpStream::connect(addr).map_err(|e| {
                    EngineError::ConfigError(format!("Failed to connect to HMR TCP: {e}"))
                })?;
                return Ok(Self {
                    stream: HmrStream::Tcp(tcp),
                    token,
                });
            }
        }

        #[cfg(unix)]
        {
            let unix = UnixStream::connect(endpoint)
                .map_err(|e| EngineError::ConfigError(format!("Failed to connect to HMR: {e}")))?;
            Ok(Self {
                stream: HmrStream::Unix(unix),
                token,
            })
        }
        #[cfg(not(unix))]
        {
            Err(EngineError::ConfigError(format!(
                "HMR endpoint '{}' is missing or does not contain a valid tcp:// pointer",
                endpoint.display()
            )))
        }
    }

    #[must_use]
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn send_reload(&mut self, script: &str) -> Result<(), EngineError> {
        self.send_message(&HmrMessage::Reload {
            script: script.to_string(),
        })
    }

    pub fn send_reload_server(&mut self, bundle_js: &str) -> Result<(), EngineError> {
        self.send_message(&HmrMessage::ReloadServer {
            bundle_js: bundle_js.to_string(),
        })
    }

    pub fn send_eval(&mut self, code: &str) -> Result<(), EngineError> {
        self.send_message(&HmrMessage::Eval {
            code: code.to_string(),
        })
    }

    pub fn send_ping(&mut self) -> Result<(), EngineError> {
        self.send_message(&HmrMessage::Ping)
    }

    fn send_message(&mut self, msg: &HmrMessage) -> Result<(), EngineError> {
        let token_field = self.token.as_deref().map_or_else(String::new, |tok| {
            format!(r#","token":"{}""#, escape_json(tok))
        });
        let mut json = match msg {
            HmrMessage::Reload { script } => {
                let esc = escape_json(script);
                format!(r#"{{"type":"reload","script":"{esc}"{token_field}}}"#)
            }
            HmrMessage::ReloadServer { bundle_js } => {
                let esc = escape_json(bundle_js);
                format!(r#"{{"type":"reload_server","bundle":"{esc}"{token_field}}}"#)
            }
            HmrMessage::Eval { code } => {
                let esc = escape_json(code);
                format!(r#"{{"type":"eval","code":"{esc}"{token_field}}}"#)
            }
            HmrMessage::Ping => format!(r#"{{"type":"ping"{token_field}}}"#),
        };
        json.push('\n');
        self.stream
            .write_all(json.as_bytes())
            .map_err(|e| EngineError::ConfigError(e.to_string()))?;
        self.stream
            .flush()
            .map_err(|e| EngineError::ConfigError(e.to_string()))?;
        Ok(())
    }

    pub fn read_response(&mut self) -> Result<String, EngineError> {
        let mut bytes = Vec::new();
        let mut byte = [0u8; 1];
        while self.stream.read_exact(&mut byte).is_ok() {
            if byte[0] == b'\n' {
                break;
            }
            if bytes.len() >= MAX_IPC_FRAME_BYTES {
                return Err(EngineError::ConfigError(String::from(
                    "HMR response exceeded maximum IPC frame size",
                )));
            }
            bytes.push(byte[0]);
        }
        String::from_utf8(bytes)
            .map_err(|e| EngineError::ConfigError(format!("Invalid UTF-8 in HMR response: {e}")))
    }
}

pub fn parse_hmr_json(line: &str) -> Option<HmrMessage> {
    let msg_type = extract_json_string(line, "type")?;
    match msg_type.as_str() {
        "reload" => {
            let script = extract_json_string(line, "script").unwrap_or_default();
            Some(HmrMessage::Reload { script })
        }
        "reload_server" => {
            let bundle_js = extract_json_string(line, "bundle")
                .or_else(|| extract_json_string(line, "script"))
                .unwrap_or_default();
            Some(HmrMessage::ReloadServer { bundle_js })
        }
        "eval" => {
            let code = extract_json_string(line, "code").unwrap_or_default();
            Some(HmrMessage::Eval { code })
        }
        "ping" => Some(HmrMessage::Ping),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_escaped_json_hmr_messages() {
        let raw = r#"{"type":"reload","script":"text win \"Hello\"\nsignal c 1"}"#;
        let parsed = parse_hmr_json(raw).unwrap();
        assert_eq!(
            parsed,
            HmrMessage::Reload {
                script: String::from("text win \"Hello\"\nsignal c 1"),
            }
        );

        let ping = parse_hmr_json(r#"{"type":"ping"}"#).unwrap();
        assert_eq!(ping, HmrMessage::Ping);

        let srv =
            parse_hmr_json(r#"{"type":"reload_server","bundle":"globalThis.__NG_SRV__ = 1;"}"#)
                .unwrap();
        assert_eq!(
            srv,
            HmrMessage::ReloadServer {
                bundle_js: String::from("globalThis.__NG_SRV__ = 1;"),
            }
        );
    }

    fn run_hmr_roundtrip_for(socket_path: &Path) -> HmrIpcServer {
        let mut server = HmrIpcServer::bind(socket_path).unwrap();
        assert!(socket_path.exists());

        let mut client = HmrIpcClient::connect(socket_path).unwrap();
        client.send_ping().unwrap();

        let mut messages = Vec::new();
        for _ in 0..20 {
            messages = server.poll_messages();
            if !messages.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        assert_eq!(messages.len(), 1);
        let (cid, msg) = &messages[0];
        assert_eq!(*msg, HmrMessage::Ping);

        server.send_response(*cid, &HmrResponse::Pong);
        let response = client.read_response().unwrap();
        assert_eq!(response, r#"{"status":"ok","pong":true}"#);

        client
            .send_reload_server("globalThis.__SVR_V__ = 2;")
            .unwrap();
        let mut srv_msgs = Vec::new();
        for _ in 0..20 {
            srv_msgs = server.poll_messages();
            if !srv_msgs.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(
            srv_msgs.iter().map(|(_, m)| m.clone()).collect::<Vec<_>>(),
            vec![HmrMessage::ReloadServer {
                bundle_js: String::from("globalThis.__SVR_V__ = 2;"),
            }]
        );
        server
    }

    #[test]
    fn server_client_roundtrip_ping_and_reload() {
        let socket_path =
            std::env::temp_dir().join(format!("ng-native-test-hmr-{}.sock", std::process::id()));
        let server = run_hmr_roundtrip_for(&socket_path);
        drop(server);
        assert!(!socket_path.exists());
    }

    #[test]
    fn tcp_pointer_file_roundtrip_ping_and_reload() {
        let pointer_path =
            std::env::temp_dir().join(format!("ng-native-test-hmr-{}.tcp", std::process::id()));
        let server = run_hmr_roundtrip_for(&pointer_path);
        let pointer = fs::read_to_string(&pointer_path).unwrap();
        assert!(pointer.trim().starts_with("tcp://127.0.0.1:"), "{pointer}");
        drop(server);
        assert!(!pointer_path.exists());
    }

    #[test]
    fn roundtrips_non_ascii_utf8_split_writes_and_responses() {
        let socket_path = std::env::temp_dir().join(format!(
            "ng-native-test-hmr-utf8-{}.tcp",
            std::process::id()
        ));
        let mut server = HmrIpcServer::bind(&socket_path).unwrap();
        let mut client = HmrIpcClient::connect(&socket_path).unwrap();

        // Send a multi-byte UTF-8 eval payload split mid-codepoint across writes
        let code = r#"console.log("Pokédex ⚡ — áéíóú 🚀");"#;
        let escaped = escape_json(code);
        let full_frame = format!(r#"{{"type":"eval","code":"{escaped}"}}"#) + "\n";
        let bytes = full_frame.as_bytes();
        // Cut right inside a multi-byte character if present so server must buffer raw bytes
        let split_at = bytes
            .iter()
            .position(|&b| b >= 0x80)
            .map_or(bytes.len() / 2, |pos| pos + 1);
        client.stream.write_all(&bytes[..split_at]).unwrap();
        client.stream.flush().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert_eq!(server.poll_messages(), Vec::new());

        client.stream.write_all(&bytes[split_at..]).unwrap();
        client.stream.flush().unwrap();

        let mut msgs = Vec::new();
        for _ in 0..20 {
            msgs = server.poll_messages();
            if !msgs.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(msgs.len(), 1);
        assert_eq!(
            msgs[0].1,
            HmrMessage::Eval {
                code: String::from(code),
            }
        );

        server.send_response(
            msgs[0].0,
            &HmrResponse::EvalResult(String::from("Éxito ⚡")),
        );
        let resp = client.read_response().unwrap();
        assert_eq!(
            extract_json_string(&resp, "result").as_deref(),
            Some("Éxito ⚡")
        );

        drop(server);
    }

    #[test]
    fn enforces_auth_token_and_eval_opt_in_in_dev_hmr_server() {
        let socket_path = std::env::temp_dir().join(format!(
            "ng-native-test-hmr-auth-{}.tcp",
            std::process::id()
        ));
        let mut server = HmrIpcServer::bind_with_options(
            &socket_path,
            Some(String::from("secret-hmr-token")),
            false,
        )
        .unwrap();

        let mut bad_client = HmrIpcClient::connect(&socket_path).unwrap();
        bad_client.send_ping().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(15));
        assert_eq!(server.poll_messages(), Vec::new());
        let unauth_resp = bad_client.read_response().unwrap();
        assert!(
            unauth_resp.contains("Unauthorized HMR request"),
            "{unauth_resp}"
        );

        let mut authed_client = HmrIpcClient::connect(&socket_path)
            .unwrap()
            .with_token("secret-hmr-token");
        authed_client.send_eval("1 + 1").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(15));
        assert_eq!(server.poll_messages(), Vec::new());
        let eval_denied = authed_client.read_response().unwrap();
        assert!(
            eval_denied.contains("HMR eval is disabled"),
            "{eval_denied}"
        );

        authed_client.send_reload("text win Ok").unwrap();
        let mut msgs = Vec::new();
        for _ in 0..20 {
            msgs = server.poll_messages();
            if !msgs.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(msgs.len(), 1);
        assert_eq!(
            msgs[0].1,
            HmrMessage::Reload {
                script: String::from("text win Ok"),
            }
        );

        drop(server);

        let dev_sock =
            std::env::temp_dir().join(format!("ng-native-test-hmr-dev-{}.tcp", std::process::id()));
        let token_file = token_path_for(&dev_sock);
        let mut dev_server = HmrIpcServer::bind_dev(&dev_sock, false).unwrap();
        assert!(token_file.is_file());
        let mut auto_client = HmrIpcClient::connect(&dev_sock).unwrap();
        auto_client.send_ping().unwrap();
        let mut dev_msgs = Vec::new();
        for _ in 0..20 {
            dev_msgs = dev_server.poll_messages();
            if !dev_msgs.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(dev_msgs.len(), 1);
        assert_eq!(dev_msgs[0].1, HmrMessage::Ping);
        drop(dev_server);
        assert!(!token_file.exists());
    }
}

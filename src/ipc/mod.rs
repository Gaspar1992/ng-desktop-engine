//! Cross-platform Hot Module Replacement (`HMR`) IPC server and client over Unix Domain Sockets
//! and loopback TCP pointer files, plus reusable [`test_harness`] helpers.

pub mod test_harness;

use crate::desktop::http_codec::{escape_json, extract_json_string};
use crate::engine::EngineError;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

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

/// Cross-platform HMR IPC Server.
/// Listens for live bundle updates and evaluation requests over a Unix domain socket (Linux/macOS)
/// or a loopback TCP listener advertised via an endpoint pointer file (Windows / `.tcp` targets).
pub struct HmrIpcServer {
    socket_path: PathBuf,
    listener: HmrListener,
    clients: HashMap<ClientId, HmrStream>,
    in_buffers: HashMap<ClientId, Vec<u8>>,
    next_client_id: ClientId,
}

impl HmrIpcServer {
    /// Binds to a local Unix socket path or writes a loopback TCP pointer file on Windows (or `.tcp` paths).
    pub fn bind(path: impl AsRef<Path>) -> Result<Self, EngineError> {
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
            listener,
            clients: HashMap::new(),
            in_buffers: HashMap::new(),
            next_client_id: 1,
        })
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Polls for incoming client connections and full newline-delimited HMR messages without blocking.
    pub fn poll_messages(&mut self) -> Vec<(ClientId, HmrMessage)> {
        let mut messages = Vec::new();
        while let Some(stream) = self.listener.accept_nonblocking() {
            let cid = self.next_client_id;
            self.next_client_id += 1;
            self.clients.insert(cid, stream);
            self.in_buffers.insert(cid, Vec::new());
        }

        let mut disconnected = Vec::new();
        let mut read_buf = [0u8; 4096];

        for (&cid, stream) in &mut self.clients {
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

            if let Some(buf) = self.in_buffers.get_mut(&cid) {
                while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                    let frame: Vec<u8> = buf.drain(..=pos).collect();
                    let decoded = String::from_utf8_lossy(&frame[..pos]);
                    let line = decoded.trim();
                    if !line.is_empty()
                        && let Some(msg) = parse_hmr_json(line)
                    {
                        messages.push((cid, msg));
                    }
                }
            }
        }

        for cid in disconnected {
            self.clients.remove(&cid);
            self.in_buffers.remove(&cid);
        }
        messages
    }

    /// Sends a JSON-framed response back to the specified client.
    pub fn send_response(&mut self, client_id: ClientId, response: &HmrResponse) {
        if let Some(stream) = self.clients.get_mut(&client_id) {
            let mut payload = response.to_json();
            payload.push('\n');
            let _ = stream.write_all(payload.as_bytes());
            let _ = stream.flush();
        }
    }
}

impl Drop for HmrIpcServer {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket_path);
    }
}

/// Client for sending HMR updates from Rust tests or tooling.
pub struct HmrIpcClient {
    stream: HmrStream,
}

impl HmrIpcClient {
    pub fn connect(path: impl AsRef<Path>) -> Result<Self, EngineError> {
        let endpoint = path.as_ref();
        if let Ok(raw) = fs::read_to_string(endpoint) {
            let trimmed = raw.trim();
            if let Some(addr) = trimmed.strip_prefix("tcp://") {
                let tcp = TcpStream::connect(addr).map_err(|e| {
                    EngineError::ConfigError(format!("Failed to connect to HMR TCP: {e}"))
                })?;
                return Ok(Self {
                    stream: HmrStream::Tcp(tcp),
                });
            }
        }

        #[cfg(unix)]
        {
            let unix = UnixStream::connect(endpoint)
                .map_err(|e| EngineError::ConfigError(format!("Failed to connect to HMR: {e}")))?;
            Ok(Self {
                stream: HmrStream::Unix(unix),
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
        let mut json = match msg {
            HmrMessage::Reload { script } => {
                let esc = escape_json(script);
                format!(r#"{{"type":"reload","script":"{esc}"}}"#)
            }
            HmrMessage::ReloadServer { bundle_js } => {
                let esc = escape_json(bundle_js);
                format!(r#"{{"type":"reload_server","bundle":"{esc}"}}"#)
            }
            HmrMessage::Eval { code } => {
                let esc = escape_json(code);
                format!(r#"{{"type":"eval","code":"{esc}"}}"#)
            }
            HmrMessage::Ping => String::from(r#"{"type":"ping"}"#),
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
}

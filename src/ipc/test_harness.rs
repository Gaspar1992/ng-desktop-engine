use crate::config::AngularNativeConfig;
use crate::desktop::http_codec::escape_json;
use crate::engine::{EventPayload, NodeId, RenderEngine};
use crate::platforms::AnyPlatformBackend;
use crate::runtime::JsRuntime;
use std::io::{BufRead, Write};
use std::thread;
use std::time::{Duration, Instant};

/// Callback invoked on demand when `--defer-boot` is active and the E2E client sends `"cmd":"boot"`.
pub type BootCallback<'a> = &'a mut dyn FnMut(
    &mut RenderEngine<AnyPlatformBackend>,
    &mut JsRuntime,
) -> Result<String, String>;

pub struct TestHarnessState<'a> {
    pub cfg: &'a AngularNativeConfig,
    pub engine: &'a mut RenderEngine<AnyPlatformBackend>,
    pub js_runtime: &'a mut JsRuntime,
    pub booted: bool,
    pub bundle_kind: String,
    pub boot_cb: BootCallback<'a>,
}

/// Emits the initial harness status line (`waiting_boot` or `ready`) to stdout.
pub fn emit_status_line(
    status: &str,
    project_name: &str,
    backend_name: &str,
    engine_name: &str,
    bundle_kind: Option<&str>,
) {
    let mut line = format!(
        r#"{{"status":"{}","project":"{}","backend":"{}","engine":"{}""#,
        escape_json(status),
        escape_json(project_name),
        escape_json(backend_name),
        escape_json(engine_name),
    );
    if let Some(kind) = bundle_kind {
        use std::fmt::Write as _;
        let _ = write!(line, r#","bundle":"{}""#, escape_json(kind));
    }
    line.push('}');
    println!("{line}");
    let _ = std::io::stdout().flush();
}

/// Emits a structured startup error JSON line to stdout.
pub fn emit_status_error(err: &str) {
    println!(r#"{{"status":"error","error":"{}"}}"#, escape_json(err));
    let _ = std::io::stdout().flush();
}

/// Runs the stdio JSON-Lines E2E automation loop against `stdin` and `stdout`.
pub fn run_stdio_harness(state: &mut TestHarnessState<'_>) -> Result<(), String> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    run_harness_loop(&mut reader, &mut writer, state)
}

/// Runs the stdio JSON-Lines E2E automation loop over any buffered reader and writer.
pub fn run_harness_loop<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    state: &mut TestHarnessState<'_>,
) -> Result<(), String> {
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|e| format!("Failed reading IPC command line: {e}"))?;
        if bytes == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let keep_running = process_command_line(trimmed, writer, state)?;
        writer
            .flush()
            .map_err(|e| format!("Failed flushing IPC response: {e}"))?;
        if !keep_running {
            break;
        }
    }
    Ok(())
}

fn process_command_line<W: Write>(
    line: &str,
    writer: &mut W,
    state: &mut TestHarnessState<'_>,
) -> Result<bool, String> {
    let Some(cmd) = extract_json_string(line, "cmd") else {
        write_error(writer, "Missing 'cmd' field in JSON request")?;
        return Ok(true);
    };
    match cmd.as_str() {
        "ping" => write_raw(writer, r#"{"ok":true,"pong":true}"#)?,
        "mock_http" => handle_mock_http(line, writer, state)?,
        "boot" => handle_boot(writer, state)?,
        "preview" => handle_preview(writer, state)?,
        "snapshot" => handle_snapshot(writer, state)?,
        "query" => handle_query(line, writer, state)?,
        "dispatch" => handle_dispatch(line, writer, state)?,
        "wait_for" => handle_wait_for(line, writer, state)?,
        "tick" => handle_tick(line, writer, state)?,
        "eval" => handle_eval(line, writer, state)?,
        "quit" => {
            write_raw(writer, r#"{"ok":true,"bye":true}"#)?;
            return Ok(false);
        }
        other => write_error(writer, &format!("Unknown command '{other}'"))?,
    }
    Ok(true)
}

fn handle_mock_http<W: Write>(
    line: &str,
    writer: &mut W,
    state: &TestHarnessState<'_>,
) -> Result<(), String> {
    let method = extract_json_string(line, "method").unwrap_or_else(|| String::from("GET"));
    let Some(url) = extract_json_string(line, "url") else {
        return write_error(writer, "mock_http requires 'url'");
    };
    let status_u64 = extract_json_u64(line, "status").unwrap_or(200);
    let status = u16::try_from(status_u64).unwrap_or(200);
    let body = extract_json_string(line, "body").unwrap_or_default();

    let shared = state.js_runtime.desktop().shared_http_engine();
    let Ok(mut http) = shared.lock() else {
        return write_error(writer, "HTTP engine lock poisoned");
    };
    http.register_http_endpoint(&method, &url, status, &body);
    write_raw(writer, r#"{"ok":true}"#)
}

fn handle_boot<W: Write>(writer: &mut W, state: &mut TestHarnessState<'_>) -> Result<(), String> {
    if state.booted {
        return write_raw(
            writer,
            &format!(
                r#"{{"ok":true,"alreadyBooted":true,"bundle":"{}"}}"#,
                escape_json(&state.bundle_kind)
            ),
        );
    }
    match (state.boot_cb)(state.engine, state.js_runtime) {
        Ok(kind) => {
            state.booted = true;
            state.bundle_kind = kind;
            state.engine.flush_frame();
            write_raw(
                writer,
                &format!(
                    r#"{{"ok":true,"bundle":"{}"}}"#,
                    escape_json(&state.bundle_kind)
                ),
            )
        }
        Err(err) => write_error(writer, &err),
    }
}

fn require_primary_window(state: &TestHarnessState<'_>) -> Result<NodeId, String> {
    if !state.booted {
        return Err(String::from("Application has not booted yet"));
    }
    state
        .js_runtime
        .resolve_primary_window()
        .map_err(|e| format!("{e:?}"))
}

fn handle_preview<W: Write>(writer: &mut W, state: &TestHarnessState<'_>) -> Result<(), String> {
    let win = match require_primary_window(state) {
        Ok(id) => id,
        Err(msg) => return write_error(writer, &msg),
    };
    match state.engine.render_window_preview(win) {
        Ok(preview) => write_raw(
            writer,
            &format!(r#"{{"ok":true,"preview":"{}"}}"#, escape_json(&preview)),
        ),
        Err(err) => write_error(writer, &format!("{err:?}")),
    }
}

fn handle_snapshot<W: Write>(writer: &mut W, state: &TestHarnessState<'_>) -> Result<(), String> {
    let win = match require_primary_window(state) {
        Ok(id) => id,
        Err(msg) => return write_error(writer, &msg),
    };
    match state.engine.snapshot_tree_json(win) {
        Ok(tree) => write_raw(writer, &format!(r#"{{"ok":true,"tree":{tree}}}"#)),
        Err(err) => write_error(writer, &format!("{err:?}")),
    }
}

fn handle_query<W: Write>(
    line: &str,
    writer: &mut W,
    state: &TestHarnessState<'_>,
) -> Result<(), String> {
    let win = match require_primary_window(state) {
        Ok(id) => id,
        Err(msg) => return write_error(writer, &msg),
    };
    let id_or_testid = extract_json_string(line, "id");
    let tag = extract_json_string(line, "tag");
    let text = extract_json_string(line, "text");
    match state.engine.query_nodes_json(
        win,
        id_or_testid.as_deref(),
        tag.as_deref(),
        text.as_deref(),
    ) {
        Ok(matches) => write_raw(writer, &format!(r#"{{"ok":true,"matches":{matches}}}"#)),
        Err(err) => write_error(writer, &format!("{err:?}")),
    }
}

fn handle_dispatch<W: Write>(
    line: &str,
    writer: &mut W,
    state: &mut TestHarnessState<'_>,
) -> Result<(), String> {
    let win = match require_primary_window(state) {
        Ok(id) => id,
        Err(msg) => return write_error(writer, &msg),
    };
    let Some(target) = extract_json_string(line, "target") else {
        return write_error(writer, "dispatch requires 'target'");
    };
    let Some(event) = extract_json_string(line, "event") else {
        return write_error(writer, "dispatch requires 'event'");
    };
    let node_id = match state.js_runtime.resolve_var(&target) {
        Ok(id) => id,
        Err(_) => match state.engine.find_node_by_attr(win, &target) {
            Some(id) => id,
            None => return write_error(writer, &format!("Unknown target node '{target}'")),
        },
    };
    let payload = parse_dispatch_payload(line);
    if let Err(err) =
        state
            .js_runtime
            .trigger_node_event_with_payload(state.engine, node_id, &event, &payload)
    {
        return write_error(writer, &format!("Dispatch failed: {err:?}"));
    }
    if let Err(err) = state.js_runtime.tick(state.engine) {
        return write_error(writer, &format!("Tick after dispatch failed: {err:?}"));
    }
    state.engine.flush_frame();
    write_raw(writer, r#"{"ok":true}"#)
}

fn handle_wait_for<W: Write>(
    line: &str,
    writer: &mut W,
    state: &mut TestHarnessState<'_>,
) -> Result<(), String> {
    let win = match require_primary_window(state) {
        Ok(id) => id,
        Err(msg) => return write_error(writer, &msg),
    };
    let Some(needle) = extract_json_string(line, "text") else {
        return write_error(writer, "wait_for requires 'text'");
    };
    let absent = extract_json_bool(line, "absent").unwrap_or(false);
    let timeout_ms = extract_json_u64(line, "timeoutMs").unwrap_or(3000);
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);

    loop {
        if let Err(err) = state.js_runtime.tick(state.engine) {
            return write_error(writer, &format!("Tick during wait_for failed: {err:?}"));
        }
        state.engine.flush_frame();
        let preview = state
            .engine
            .render_window_preview(win)
            .map_err(|e| format!("{e:?}"))?;
        let matched = if absent {
            !preview.contains(&needle)
        } else {
            preview.contains(&needle)
        };
        if matched {
            return write_raw(
                writer,
                &format!(r#"{{"ok":true,"preview":"{}"}}"#, escape_json(&preview)),
            );
        }
        if Instant::now() >= deadline {
            return write_raw(
                writer,
                &format!(
                    r#"{{"ok":false,"error":"timeout","preview":"{}"}}"#,
                    escape_json(&preview)
                ),
            );
        }
        thread::sleep(Duration::from_millis(5));
    }
}

fn handle_tick<W: Write>(
    line: &str,
    writer: &mut W,
    state: &mut TestHarnessState<'_>,
) -> Result<(), String> {
    if !state.booted {
        return write_error(writer, "Application has not booted yet");
    }
    let steps = usize::try_from(extract_json_u64(line, "steps").unwrap_or(1)).unwrap_or(1);
    let sleep_ms = extract_json_u64(line, "sleepMs").unwrap_or(0);
    for idx in 0..steps {
        if let Err(err) = state.js_runtime.tick(state.engine) {
            return write_error(writer, &format!("Tick failed: {err:?}"));
        }
        state.engine.flush_frame();
        if sleep_ms > 0 && idx + 1 < steps {
            thread::sleep(Duration::from_millis(sleep_ms));
        }
    }
    write_raw(writer, r#"{"ok":true}"#)
}

fn handle_eval<W: Write>(
    line: &str,
    writer: &mut W,
    state: &mut TestHarnessState<'_>,
) -> Result<(), String> {
    let Some(code) = extract_json_string(line, "code") else {
        return write_error(writer, "eval requires 'code'");
    };
    match state.js_runtime.eval_js(state.engine, &code) {
        Ok(res) => {
            state.engine.flush_frame();
            write_raw(
                writer,
                &format!(r#"{{"ok":true,"result":"{}"}}"#, escape_json(&res)),
            )
        }
        Err(err) => write_error(writer, &format!("Eval failed: {err:?}")),
    }
}

fn parse_dispatch_payload(line: &str) -> EventPayload {
    let Some(raw) = extract_json_raw_token(line, "value") else {
        return EventPayload::None;
    };
    if raw == "null" {
        EventPayload::None
    } else if raw == "true" {
        EventPayload::Bool(true)
    } else if raw == "false" {
        EventPayload::Bool(false)
    } else if raw.starts_with('"') {
        unquote_json_token(raw).map_or(EventPayload::None, EventPayload::Text)
    } else if let Ok(num) = raw.parse::<i64>() {
        EventPayload::Int(num)
    } else {
        EventPayload::None
    }
}

fn write_raw<W: Write>(writer: &mut W, json_line: &str) -> Result<(), String> {
    writeln!(writer, "{json_line}").map_err(|e| format!("IPC write failed: {e}"))
}

fn write_error<W: Write>(writer: &mut W, msg: &str) -> Result<(), String> {
    write_raw(
        writer,
        &format!(r#"{{"ok":false,"error":"{}"}}"#, escape_json(msg)),
    )
}

fn find_json_value_start<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\"");
    let mut offset = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let bytes = json.as_bytes();
    while offset < bytes.len() {
        let ch = bytes[offset];
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == b'\\' {
                escaped = true;
            } else if ch == b'"' {
                in_string = false;
            }
            offset += 1;
            continue;
        }
        if json[offset..].starts_with(&needle) {
            let after_key = json[offset + needle.len()..].trim_start();
            if let Some(after_colon) = after_key.strip_prefix(':') {
                return Some(after_colon.trim_start());
            }
        }
        if ch == b'"' {
            in_string = true;
        }
        offset += 1;
    }
    None
}

fn extract_json_raw_token<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let val = find_json_value_start(json, key)?;
    if val.starts_with('"') {
        let mut escaped = false;
        for (idx, ch) in val.char_indices().skip(1) {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                return Some(&val[..=idx]);
            }
        }
        return None;
    }
    let end = val
        .find(|c: char| c == ',' || c == '}' || c.is_whitespace())
        .unwrap_or(val.len());
    let token = &val[..end];
    if token.is_empty() { None } else { Some(token) }
}

fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let token = extract_json_raw_token(json, key)?;
    unquote_json_token(token)
}

fn extract_json_u64(json: &str, key: &str) -> Option<u64> {
    extract_json_raw_token(json, key)?.parse::<u64>().ok()
}

fn extract_json_bool(json: &str, key: &str) -> Option<bool> {
    match extract_json_raw_token(json, key)? {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn unquote_json_token(token: &str) -> Option<String> {
    let inner = token.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some('\\') | None => out.push('\\'),
                Some('/') => out.push('/'),
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Ok(code) = u32::from_str_radix(&hex, 16)
                        && let Some(ch) = char::from_u32(code)
                    {
                        out.push(ch);
                    }
                }
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn stdio_harness_executes_ping_mock_boot_snapshot_query_dispatch_and_quit() {
        let cfg = AngularNativeConfig::default_standalone("e2e-unit-sample");
        let mut engine = RenderEngine::new(AnyPlatformBackend::headless());
        let mut js_runtime = JsRuntime::from_config(&cfg).unwrap();

        let script = r#"
            style badge #1e293b #ffffff 6 4
            create win ng-window
            text win "Unit Harness"
            create box ng-box
            create lbl ng-label
            attr lbl data-testid "status-lbl"
            text lbl "Waiting"
            create btn ng-button
            attr btn data-testid "action-btn"
            text btn "Trigger"
            on_set btn clicked count 1
            append box lbl
            append box btn
            append win box
            signal count 0
            bind_text lbl count "Clicks:{}"
        "#;

        let mut boot_cb = |eng: &mut RenderEngine<AnyPlatformBackend>, rt: &mut JsRuntime| {
            rt.eval_tick(eng, script).map_err(|e| format!("{e:?}"))?;
            Ok(String::from("ngb"))
        };

        let commands = [
            r#"{"cmd":"ping"}"#,
            r#"{"cmd":"mock_http","method":"GET","url":"https://mock.local/ping","status":200,"body":"{\"ok\":true}"}"#,
            r#"{"cmd":"boot"}"#,
            r#"{"cmd":"query","id":"status-lbl"}"#,
            r#"{"cmd":"snapshot"}"#,
            r#"{"cmd":"dispatch","target":"action-btn","event":"clicked"}"#,
            r#"{"cmd":"wait_for","text":"Clicks:1","timeoutMs":500}"#,
            r#"{"cmd":"quit"}"#,
        ]
        .join("\n");

        let mut reader = Cursor::new(commands.into_bytes());
        let mut out_buf = Vec::new();
        let mut state = TestHarnessState {
            cfg: &cfg,
            engine: &mut engine,
            js_runtime: &mut js_runtime,
            booted: false,
            bundle_kind: String::from("pending"),
            boot_cb: &mut boot_cb,
        };

        run_harness_loop(&mut reader, &mut out_buf, &mut state).unwrap();
        let output = String::from_utf8(out_buf).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 8, "output:\n{output}");
        assert_eq!(lines[0], r#"{"ok":true,"pong":true}"#);
        assert_eq!(lines[1], r#"{"ok":true}"#);
        assert_eq!(lines[2], r#"{"ok":true,"bundle":"ngb"}"#);
        assert!(
            lines[3].contains(r#""testId":"status-lbl""#),
            "{}",
            lines[3]
        );
        assert!(lines[4].contains(r#""tag":"ng-window""#), "{}", lines[4]);
        assert_eq!(lines[5], r#"{"ok":true}"#);
        assert!(lines[6].contains("Clicks:1"), "{}", lines[6]);
        assert_eq!(lines[7], r#"{"ok":true,"bye":true}"#);

        let res = state
            .js_runtime
            .desktop()
            .shared_http_engine()
            .lock()
            .unwrap()
            .execute_http(&crate::desktop::NativeHttpRequest::new(
                "GET",
                "https://mock.local/ping",
            ));
        assert_eq!(res.status, 200);
        assert_eq!(res.body, r#"{"ok":true}"#);
    }
}

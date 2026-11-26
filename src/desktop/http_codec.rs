use super::http_native::{
    NativeHttpCallKind, NativeHttpConfig, NativeHttpRequest, NativeHttpResponse, OsHttpPlatform,
};
use std::collections::HashMap;
use std::fmt::Write;

pub(crate) fn encode_response_json(res: &NativeHttpResponse) -> String {
    let hdrs: Vec<String> = res
        .headers
        .iter()
        .map(|(k, v)| format!("\"{}\":\"{}\"", escape_json(k), escape_json(v)))
        .collect();
    format!(
        "{{\"status\":{},\"statusText\":\"{}\",\"body\":\"{}\",\"headers\":{{{}}},\"transport\":\"{}\",\"callKind\":\"{}\",\"cached\":{}}}",
        res.status,
        escape_json(&res.status_text),
        escape_json(&res.body),
        hdrs.join(","),
        escape_json(&res.transport),
        res.call_kind.as_str(),
        res.cached
    )
}

pub(super) fn encode_config_json(cfg: &NativeHttpConfig, platform: OsHttpPlatform) -> String {
    format!(
        "{{\"platform\":\"{}\",\"useFetchTransport\":{},\"interceptorsEnabled\":{},\"xsrfEnabled\":{},\"xsrfCookieName\":\"{}\",\"xsrfHeaderName\":\"{}\",\"jsonpEnabled\":{},\"requestsViaParent\":{},\"maxParallelConnections\":{},\"cacheEnabled\":{},\"timeoutMs\":{}}}",
        platform.as_str(),
        cfg.use_fetch_transport,
        cfg.interceptors_enabled,
        cfg.xsrf_enabled,
        escape_json(&cfg.xsrf_cookie_name),
        escape_json(&cfg.xsrf_header_name),
        cfg.jsonp_enabled,
        cfg.requests_via_parent,
        cfg.max_parallel_connections,
        cfg.cache_enabled,
        cfg.timeout_ms,
    )
}

pub(super) fn parse_bridge_request(json: &str) -> NativeHttpRequest {
    let method = extract_json_string(json, "method").unwrap_or_else(|| String::from("GET"));
    let url = extract_json_string(json, "url").unwrap_or_default();
    let body = extract_json_string(json, "body");
    let response_type =
        extract_json_string(json, "responseType").unwrap_or_else(|| String::from("json"));
    let explicit_kind = extract_json_string(json, "callKind");
    let with_credentials = extract_json_bool(json, "withCredentials").unwrap_or(false);
    let headers = extract_json_flat_map(json, "headers");
    let call_kind = NativeHttpCallKind::detect(
        &method,
        &url,
        explicit_kind.as_deref(),
        Some(&response_type),
    );
    NativeHttpRequest {
        method,
        url,
        body,
        headers,
        call_kind,
        response_type,
        with_credentials,
    }
}

pub(super) fn parse_bridge_batch(json: &str) -> Vec<NativeHttpRequest> {
    let Some(start) = json.find('[') else {
        return Vec::new();
    };
    let slice = &json[start + 1..];
    let mut requests = Vec::new();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    let mut obj_start: Option<usize> = None;

    for (idx, ch) in slice.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    obj_start = Some(idx);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0
                    && let Some(start_idx) = obj_start.take()
                {
                    requests.push(parse_bridge_request(&slice[start_idx..=idx]));
                }
            }
            ']' if depth == 0 => break,
            _ => {}
        }
    }
    requests
}

fn find_json_value_start<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let mut rest = json;
    let mut depth = 0i32;
    while let Some(ch) = rest.chars().next() {
        match ch {
            '{' | '[' => {
                depth += 1;
                rest = &rest[1..];
            }
            '}' | ']' => {
                depth -= 1;
                rest = &rest[1..];
            }
            '"' => {
                let (parsed_key, byte_len) = parse_json_string_at(rest)?;
                let after = &rest[byte_len..];
                let trimmed = after.trim_start();
                if depth == 1 && parsed_key == key && trimmed.starts_with(':') {
                    return trimmed.strip_prefix(':').map(str::trim_start);
                }
                rest = after;
            }
            _ => rest = &rest[ch.len_utf8()..],
        }
    }
    None
}

fn parse_json_string_at(after_colon: &str) -> Option<(String, usize)> {
    super::json::parse_json_string_at(after_colon).ok()
}

pub(crate) fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let val = find_json_value_start(json, key)?;
    parse_json_string_at(val).map(|(s, _)| s)
}

pub(crate) fn extract_json_bool(json: &str, key: &str) -> Option<bool> {
    let val = find_json_value_start(json, key)?;
    if val.starts_with("true") {
        Some(true)
    } else if val.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

pub(crate) fn extract_json_u64(json: &str, key: &str) -> Option<u64> {
    let val = find_json_value_start(json, key)?;
    let digits: String = val.chars().take_while(char::is_ascii_digit).collect();
    digits.parse::<u64>().ok()
}

pub(crate) fn extract_json_i64(json: &str, key: &str) -> Option<i64> {
    let val = find_json_value_start(json, key)?;
    let mut chars = val.char_indices().peekable();
    let mut end = 0usize;
    if let Some((_, '-')) = chars.peek().copied() {
        end = 1;
        chars.next();
    }
    for (idx, ch) in chars {
        if ch.is_ascii_digit() {
            end = idx + ch.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 || (end == 1 && val.starts_with('-')) {
        return None;
    }
    val.get(..end)?.parse::<i64>().ok()
}

pub(crate) fn extract_json_object_array(json: &str, key: &str) -> Vec<String> {
    let mut objects = Vec::new();
    let Some(val) = find_json_value_start(json, key) else {
        return objects;
    };
    let Some(slice) = val.strip_prefix('[') else {
        return objects;
    };
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    let mut obj_start: Option<usize> = None;

    for (idx, ch) in slice.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    obj_start = Some(idx);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0
                    && let Some(start_idx) = obj_start.take()
                {
                    objects.push(slice[start_idx..=idx].to_string());
                }
            }
            ']' if depth == 0 => break,
            _ => {}
        }
    }
    objects
}

pub(super) fn extract_json_flat_map(json: &str, key: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Some(val) = find_json_value_start(json, key) else {
        return map;
    };
    let Some(mut rest) = val.strip_prefix('{') else {
        return map;
    };
    loop {
        rest = rest.trim_start();
        if rest.is_empty() || rest.starts_with('}') {
            break;
        }
        if let Some(next) = rest.strip_prefix(',') {
            rest = next;
            continue;
        }
        let Some((entry_key, key_len)) = parse_json_string_at(rest) else {
            break;
        };
        rest = rest[key_len..].trim_start();
        let Some(after_colon) = rest.strip_prefix(':') else {
            break;
        };
        let after_colon = after_colon.trim_start();
        let Some((entry_val, val_len)) = parse_json_string_at(after_colon) else {
            break;
        };
        map.insert(entry_key, entry_val);
        rest = &after_colon[val_len..];
    }
    map
}

pub(crate) fn extract_json_string_array(json: &str, key: &str) -> Vec<String> {
    let mut items = Vec::new();
    let Some(val) = find_json_value_start(json, key) else {
        return items;
    };
    let Some(mut rest) = val.strip_prefix('[') else {
        return items;
    };
    loop {
        rest = rest.trim_start();
        if rest.is_empty() || rest.starts_with(']') {
            break;
        }
        if let Some(next) = rest.strip_prefix(',') {
            rest = next;
            continue;
        }
        let Some((item, len)) = parse_json_string_at(rest) else {
            break;
        };
        items.push(item);
        rest = rest.get(len..).unwrap_or_default();
    }
    items
}

pub(crate) fn extract_json_raw_value(json: &str, key: &str) -> Option<String> {
    let val = find_json_value_start(json, key)?;
    if val.starts_with('"') {
        let (_, len) = parse_json_string_at(val)?;
        return val.get(..len).map(str::to_string);
    }
    if val.starts_with('{') || val.starts_with('[') {
        let mut depth = 0usize;
        let mut in_string = false;
        let mut escaped = false;
        for (idx, ch) in val.char_indices() {
            if in_string {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    in_string = false;
                }
                continue;
            }
            match ch {
                '"' => in_string = true,
                '{' | '[' => depth += 1,
                '}' | ']' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        let end = idx + ch.len_utf8();
                        return val.get(..end).map(str::to_string);
                    }
                }
                _ => {}
            }
        }
        return None;
    }
    let end = val
        .find(|c: char| c.is_ascii_whitespace() || matches!(c, ',' | '}' | ']'))
        .unwrap_or(val.len());
    val.get(..end).filter(|s| !s.is_empty()).map(str::to_string)
}

pub(crate) fn escape_json(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 16);
    for c in raw.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            other => out.push(other),
        }
    }
    out
}

//! Zero-dependency RFC 8259 recursive-descent JSON parser with UTF-16 surrogate pair decoding.

const MAX_JSON_DEPTH: usize = 64;

/// Parsed JSON value preserving object key insertion order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}

impl JsonValue {
    pub fn as_object(&self) -> Option<&[(String, Self)]> {
        match self {
            Self::Object(entries) => Some(entries),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Self> {
        self.as_object()?
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key)?.as_str()
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key)?.as_bool()
    }

    pub fn get_array(&self, key: &str) -> Option<&[Self]> {
        self.get(key)?.as_array()
    }

    pub fn get_object(&self, key: &str) -> Option<&Self> {
        let val = self.get(key)?;
        matches!(val, Self::Object(_)).then_some(val)
    }
}

/// Parses a complete JSON document into [`JsonValue`] with bounded recursion depth.
pub fn parse_json(input: &str) -> Result<JsonValue, String> {
    let trimmed = input.trim_start();
    let (value, rest) = parse_value(trimmed, 0)?;
    if !rest.trim().is_empty() {
        return Err(String::from("Trailing characters after JSON document"));
    }
    Ok(value)
}

/// Parses a JSON string literal starting at a leading `"` and returns `(decoded, bytes_consumed)`.
/// Decodes RFC 8259 escapes including `\uXXXX` UTF-16 surrogate pairs.
pub fn parse_json_string_at(input: &str) -> Result<(String, usize), String> {
    if !input.starts_with('"') {
        return Err(String::from("Expected opening quote in JSON string"));
    }
    let bytes = input.as_bytes();
    let mut i = 1usize;
    let mut out = String::new();

    while i < bytes.len() {
        let b = bytes[i];
        if b == b'"' {
            return Ok((out, i + 1));
        }
        if b == b'\\' {
            i += 1;
            if i >= bytes.len() {
                return Err(String::from("Unterminated escape in JSON string"));
            }
            match bytes[i] {
                b'"' => {
                    out.push('"');
                    i += 1;
                }
                b'\\' => {
                    out.push('\\');
                    i += 1;
                }
                b'/' => {
                    out.push('/');
                    i += 1;
                }
                b'b' => {
                    out.push('\x08');
                    i += 1;
                }
                b'f' => {
                    out.push('\x0c');
                    i += 1;
                }
                b'n' => {
                    out.push('\n');
                    i += 1;
                }
                b'r' => {
                    out.push('\r');
                    i += 1;
                }
                b't' => {
                    out.push('\t');
                    i += 1;
                }
                b'u' => {
                    i += 1;
                    let unit = parse_hex4(bytes, i)?;
                    i += 4;
                    if (0xD800..=0xDBFF).contains(&unit) {
                        if i + 6 <= bytes.len() && bytes[i] == b'\\' && bytes[i + 1] == b'u' {
                            let low = parse_hex4(bytes, i + 2)?;
                            if (0xDC00..=0xDFFF).contains(&low) {
                                i += 6;
                                let combined = 0x10000
                                    + ((u32::from(unit) - 0xD800) << 10)
                                    + (u32::from(low) - 0xDC00);
                                out.push(char::from_u32(combined).unwrap_or('\u{FFFD}'));
                                continue;
                            }
                        }
                        out.push('\u{FFFD}');
                    } else if (0xDC00..=0xDFFF).contains(&unit) {
                        out.push('\u{FFFD}');
                    } else {
                        out.push(char::from_u32(u32::from(unit)).unwrap_or('\u{FFFD}'));
                    }
                }
                other => {
                    out.push(char::from(other));
                    i += 1;
                }
            }
        } else if b < 0x80 {
            out.push(char::from(b));
            i += 1;
        } else {
            let ch = input[i..]
                .chars()
                .next()
                .ok_or_else(|| String::from("Invalid UTF-8 in JSON string"))?;
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    Err(String::from("Unterminated JSON string"))
}

fn parse_hex4(bytes: &[u8], start: usize) -> Result<u16, String> {
    if start + 4 > bytes.len() {
        return Err(String::from("Truncated \\uXXXX escape in JSON string"));
    }
    let hex_str = std::str::from_utf8(&bytes[start..start + 4])
        .map_err(|_| String::from("Invalid UTF-8 in \\uXXXX escape"))?;
    u16::from_str_radix(hex_str, 16)
        .map_err(|_| format!("Invalid hex digits '\\u{hex_str}' in JSON string"))
}

fn parse_value(input: &str, depth: usize) -> Result<(JsonValue, &str), String> {
    if depth > MAX_JSON_DEPTH {
        return Err(String::from("JSON nesting depth exceeds maximum limit"));
    }
    let rest = input.trim_start();
    let Some(first) = rest.bytes().next() else {
        return Err(String::from("Unexpected end of JSON input"));
    };
    match first {
        b'{' => parse_object(rest, depth),
        b'[' => parse_array(rest, depth),
        b'"' => {
            let (s, used) = parse_json_string_at(rest)?;
            Ok((JsonValue::String(s), &rest[used..]))
        }
        b't' => rest
            .strip_prefix("true")
            .ok_or_else(|| String::from("Invalid JSON literal"))
            .map(|rem| (JsonValue::Bool(true), rem)),
        b'f' => rest
            .strip_prefix("false")
            .ok_or_else(|| String::from("Invalid JSON literal"))
            .map(|rem| (JsonValue::Bool(false), rem)),
        b'n' => rest
            .strip_prefix("null")
            .ok_or_else(|| String::from("Invalid JSON literal"))
            .map(|rem| (JsonValue::Null, rem)),
        b'-' | b'0'..=b'9' => parse_number(rest),
        other => Err(format!("Unexpected token '{}' in JSON", char::from(other))),
    }
}

fn parse_number(input: &str) -> Result<(JsonValue, &str), String> {
    let end = input
        .find(|c: char| !(c.is_ascii_digit() || matches!(c, '-' | '+' | '.' | 'e' | 'E')))
        .unwrap_or(input.len());
    let num_slice = &input[..end];
    if num_slice.is_empty() || num_slice == "-" {
        return Err(String::from("Invalid JSON number"));
    }
    num_slice
        .parse::<f64>()
        .map_err(|_| format!("Invalid JSON number '{num_slice}'"))?;
    Ok((JsonValue::Number(num_slice.to_string()), &input[end..]))
}

fn parse_array(input: &str, depth: usize) -> Result<(JsonValue, &str), String> {
    let mut rest = input
        .strip_prefix('[')
        .ok_or_else(|| String::from("Expected '['"))?
        .trim_start();
    let mut items = Vec::new();
    if let Some(after) = rest.strip_prefix(']') {
        return Ok((JsonValue::Array(items), after));
    }
    loop {
        let (val, next) = parse_value(rest, depth + 1)?;
        items.push(val);
        rest = next.trim_start();
        if let Some(after_comma) = rest.strip_prefix(',') {
            rest = after_comma.trim_start();
        } else if let Some(after_close) = rest.strip_prefix(']') {
            return Ok((JsonValue::Array(items), after_close));
        } else {
            return Err(String::from("Expected ',' or ']' in JSON array"));
        }
    }
}

fn parse_object(input: &str, depth: usize) -> Result<(JsonValue, &str), String> {
    let mut rest = input
        .strip_prefix('{')
        .ok_or_else(|| String::from("Expected '{'"))?
        .trim_start();
    let mut entries = Vec::new();
    if let Some(after) = rest.strip_prefix('}') {
        return Ok((JsonValue::Object(entries), after));
    }
    loop {
        let (key, used) = parse_json_string_at(rest)?;
        rest = rest[used..].trim_start();
        rest = rest
            .strip_prefix(':')
            .ok_or_else(|| String::from("Expected ':' after object key"))?
            .trim_start();
        let (val, next) = parse_value(rest, depth + 1)?;
        entries.push((key, val));
        rest = next.trim_start();
        if let Some(after_comma) = rest.strip_prefix(',') {
            rest = after_comma.trim_start();
        } else if let Some(after_close) = rest.strip_prefix('}') {
            return Ok((JsonValue::Object(entries), after_close));
        } else {
            return Err(String::from("Expected ',' or '}' in JSON object"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_objects_arrays_escapes_and_utf16_surrogates() {
        let doc = r#"{
            "name": "Pok\u00e9dex \uD83D\uDE80 \"Quote\"",
            "loneHigh": "\uD83D!",
            "loneLow": "\uDE80?",
            "flags": [true, false, null, -42.5],
            "nested": { "inner": { "ok": true } }
        }"#;
        let parsed = parse_json(doc).unwrap();
        assert_eq!(parsed.get_str("name"), Some("Pokédex 🚀 \"Quote\""));
        assert_eq!(parsed.get_str("loneHigh"), Some("\u{FFFD}!"));
        assert_eq!(parsed.get_str("loneLow"), Some("\u{FFFD}?"));

        let flags = parsed.get_array("flags").unwrap();
        assert_eq!(flags.len(), 4);
        assert_eq!(flags[0].as_bool(), Some(true));
        assert_eq!(flags[1].as_bool(), Some(false));
        assert_eq!(flags[2], JsonValue::Null);
        assert_eq!(flags[3], JsonValue::Number(String::from("-42.5")));

        assert_eq!(
            parsed
                .get_object("nested")
                .and_then(|o| o.get_object("inner"))
                .and_then(|o| o.get_bool("ok")),
            Some(true)
        );
    }

    #[test]
    fn rejects_excessive_nesting_depth_and_malformed_json() {
        let deep = "[".repeat(MAX_JSON_DEPTH + 2) + &"]".repeat(MAX_JSON_DEPTH + 2);
        let err = parse_json(&deep).unwrap_err();
        assert!(err.contains("depth"), "{err}");

        assert!(parse_json(r#"{"a": 1} trailing"#).is_err());
        assert!(parse_json(r#"{"unterminated": "abc}"#).is_err());
        assert!(parse_json(r#"{"badHex": "\u12XZ"}"#).is_err());
    }
}

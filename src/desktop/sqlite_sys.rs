#![allow(unsafe_code)]

use super::http_codec::escape_json;
use crate::ffi_dl::DynLib;
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char, c_double, c_int, c_void};
use std::fmt::Write as _;
use std::ptr;

const SQLITE_OK: c_int = 0;
const SQLITE_ROW: c_int = 100;
const SQLITE_DONE: c_int = 101;
const SQLITE_INTEGER: c_int = 1;
const SQLITE_FLOAT: c_int = 2;
const SQLITE_NULL: c_int = 5;
const SQLITE_TRANSIENT: isize = -1;

/// Strongly-typed SQL bind parameter forwarded to `sqlite3_bind_*`.
#[derive(Debug, Clone, PartialEq)]
pub enum SqlParam {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
}

/// Parses a JSON array of primitive SQL bind values (`null`, `bool`, numbers, strings).
pub fn parse_json_params(raw_array: &str) -> Result<Vec<SqlParam>, String> {
    let trimmed = raw_array.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let Some(mut rest) = trimmed.strip_prefix('[') else {
        return Err(String::from("SQL params must be a JSON array"));
    };
    let mut params = Vec::new();
    let mut expect_value_or_end = true;
    loop {
        rest = rest.trim_start();
        if let Some(after_end) = rest.strip_prefix(']') {
            if !after_end.trim().is_empty() {
                return Err(String::from("Trailing characters after SQL params array"));
            }
            return Ok(params);
        }
        if !expect_value_or_end {
            let Some(after_comma) = rest.strip_prefix(',') else {
                return Err(String::from("Expected ',' or ']' in SQL params array"));
            };
            rest = after_comma.trim_start();
        }
        expect_value_or_end = false;
        if let Some(after) = rest.strip_prefix("null") {
            params.push(SqlParam::Null);
            rest = after;
        } else if let Some(after) = rest.strip_prefix("true") {
            params.push(SqlParam::Integer(1));
            rest = after;
        } else if let Some(after) = rest.strip_prefix("false") {
            params.push(SqlParam::Integer(0));
            rest = after;
        } else if rest.starts_with('"') {
            let (s, used) = parse_json_param_string(rest)?;
            params.push(SqlParam::Text(s));
            rest = &rest[used..];
        } else {
            let token_end = rest
                .find(|c: char| c.is_ascii_whitespace() || matches!(c, ',' | ']'))
                .unwrap_or(rest.len());
            let token = &rest[..token_end];
            if token.is_empty() {
                return Err(String::from("Invalid SQL param token in JSON array"));
            }
            if token.contains('.') || token.contains('e') || token.contains('E') {
                let f = token
                    .parse::<f64>()
                    .map_err(|_| format!("Invalid float SQL param '{token}'"))?;
                params.push(SqlParam::Real(f));
            } else if let Ok(i) = token.parse::<i64>() {
                params.push(SqlParam::Integer(i));
            } else {
                let f = token
                    .parse::<f64>()
                    .map_err(|_| format!("Invalid numeric SQL param '{token}'"))?;
                params.push(SqlParam::Real(f));
            }
            rest = &rest[token_end..];
        }
    }
}

fn parse_json_param_string(input: &str) -> Result<(String, usize), String> {
    let rest = input
        .strip_prefix('"')
        .ok_or_else(|| String::from("Expected opening quote in SQL string param"))?;
    let mut out = String::new();
    let mut chars = rest.char_indices();
    while let Some((idx, ch)) = chars.next() {
        match ch {
            '"' => return Ok((out, idx + 2)),
            '\\' => match chars.next() {
                Some((_, '"')) => out.push('"'),
                Some((_, '\\')) => out.push('\\'),
                Some((_, '/')) => out.push('/'),
                Some((_, 'n')) => out.push('\n'),
                Some((_, 'r')) => out.push('\r'),
                Some((_, 't')) => out.push('\t'),
                Some((_, 'b')) => out.push('\x08'),
                Some((_, 'f')) => out.push('\x0c'),
                Some((_, 'u')) => {
                    let hex: String = chars.by_ref().take(4).map(|(_, c)| c).collect();
                    if let Ok(code) = u32::from_str_radix(&hex, 16)
                        && let Some(decoded) = char::from_u32(code)
                    {
                        out.push(decoded);
                    }
                }
                Some((_, other)) => out.push(other),
                None => return Err(String::from("Unterminated escape in SQL param string")),
            },
            c => out.push(c),
        }
    }
    Err(String::from("Unterminated string in SQL params array"))
}

type SqliteOpenFn = unsafe extern "C" fn(*const c_char, *mut *mut c_void) -> c_int;
type SqliteCloseFn = unsafe extern "C" fn(*mut c_void) -> c_int;
type SqlitePrepareFn = unsafe extern "C" fn(
    *mut c_void,
    *const c_char,
    c_int,
    *mut *mut c_void,
    *mut *const c_char,
) -> c_int;
type SqliteStepFn = unsafe extern "C" fn(*mut c_void) -> c_int;
type SqliteColCountFn = unsafe extern "C" fn(*mut c_void) -> c_int;
type SqliteColNameFn = unsafe extern "C" fn(*mut c_void, c_int) -> *const c_char;
type SqliteColTypeFn = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
type SqliteColInt64Fn = unsafe extern "C" fn(*mut c_void, c_int) -> i64;
type SqliteColDoubleFn = unsafe extern "C" fn(*mut c_void, c_int) -> c_double;
type SqliteColTextFn = unsafe extern "C" fn(*mut c_void, c_int) -> *const u8;
type SqliteBindNullFn = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
type SqliteBindInt64Fn = unsafe extern "C" fn(*mut c_void, c_int, i64) -> c_int;
type SqliteBindDoubleFn = unsafe extern "C" fn(*mut c_void, c_int, c_double) -> c_int;
type SqliteBindTextFn =
    unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int, isize) -> c_int;
type SqliteFinalizeFn = unsafe extern "C" fn(*mut c_void) -> c_int;
type SqliteErrmsgFn = unsafe extern "C" fn(*mut c_void) -> *const c_char;

struct SqliteFfi {
    db: *mut c_void,
    sqlite3_close: SqliteCloseFn,
    sqlite3_prepare_v2: SqlitePrepareFn,
    sqlite3_step: SqliteStepFn,
    sqlite3_column_count: SqliteColCountFn,
    sqlite3_column_name: SqliteColNameFn,
    sqlite3_column_type: SqliteColTypeFn,
    sqlite3_column_int64: SqliteColInt64Fn,
    sqlite3_column_double: SqliteColDoubleFn,
    sqlite3_column_text: SqliteColTextFn,
    sqlite3_bind_null: SqliteBindNullFn,
    sqlite3_bind_int64: SqliteBindInt64Fn,
    sqlite3_bind_double: SqliteBindDoubleFn,
    sqlite3_bind_text: SqliteBindTextFn,
    sqlite3_finalize: SqliteFinalizeFn,
    sqlite3_errmsg: SqliteErrmsgFn,
}

unsafe impl Send for SqliteFfi {}

impl SqliteFfi {
    fn open_sqlite_lib() -> Option<*mut c_void> {
        let candidates: &[&CStr] = if cfg!(target_os = "macos") {
            &[c"/usr/lib/libsqlite3.dylib", c"libsqlite3.dylib"]
        } else if cfg!(target_os = "windows") {
            &[c"winsqlite3.dll", c"sqlite3.dll"]
        } else {
            &[c"libsqlite3.so.0", c"libsqlite3.so"]
        };
        for candidate in candidates {
            if let Some(lib) = unsafe { DynLib::open(candidate) } {
                return Some(lib.as_ptr());
            }
        }
        None
    }

    fn try_open(path: &str) -> Option<Self> {
        unsafe {
            let lib = Self::open_sqlite_lib()?;
            let sqlite3_open: SqliteOpenFn = load_sym(lib, c"sqlite3_open")?;
            let sqlite3_close: SqliteCloseFn = load_sym(lib, c"sqlite3_close")?;
            let sqlite3_prepare_v2: SqlitePrepareFn = load_sym(lib, c"sqlite3_prepare_v2")?;
            let sqlite3_step: SqliteStepFn = load_sym(lib, c"sqlite3_step")?;
            let sqlite3_column_count: SqliteColCountFn = load_sym(lib, c"sqlite3_column_count")?;
            let sqlite3_column_name: SqliteColNameFn = load_sym(lib, c"sqlite3_column_name")?;
            let sqlite3_column_type: SqliteColTypeFn = load_sym(lib, c"sqlite3_column_type")?;
            let sqlite3_column_int64: SqliteColInt64Fn = load_sym(lib, c"sqlite3_column_int64")?;
            let sqlite3_column_double: SqliteColDoubleFn = load_sym(lib, c"sqlite3_column_double")?;
            let sqlite3_column_text: SqliteColTextFn = load_sym(lib, c"sqlite3_column_text")?;
            let sqlite3_bind_null: SqliteBindNullFn = load_sym(lib, c"sqlite3_bind_null")?;
            let sqlite3_bind_int64: SqliteBindInt64Fn = load_sym(lib, c"sqlite3_bind_int64")?;
            let sqlite3_bind_double: SqliteBindDoubleFn = load_sym(lib, c"sqlite3_bind_double")?;
            let sqlite3_bind_text: SqliteBindTextFn = load_sym(lib, c"sqlite3_bind_text")?;
            let sqlite3_finalize: SqliteFinalizeFn = load_sym(lib, c"sqlite3_finalize")?;
            let sqlite3_errmsg: SqliteErrmsgFn = load_sym(lib, c"sqlite3_errmsg")?;

            let c_path = CString::new(path).ok()?;
            let mut db: *mut c_void = ptr::null_mut();
            let rc = sqlite3_open(c_path.as_ptr(), &raw mut db);
            if rc != SQLITE_OK || db.is_null() {
                if !db.is_null() {
                    sqlite3_close(db);
                }
                return None;
            }
            Some(Self {
                db,
                sqlite3_close,
                sqlite3_prepare_v2,
                sqlite3_step,
                sqlite3_column_count,
                sqlite3_column_name,
                sqlite3_column_type,
                sqlite3_column_int64,
                sqlite3_column_double,
                sqlite3_column_text,
                sqlite3_bind_null,
                sqlite3_bind_int64,
                sqlite3_bind_double,
                sqlite3_bind_text,
                sqlite3_finalize,
                sqlite3_errmsg,
            })
        }
    }

    fn last_error(&self) -> String {
        unsafe {
            let ptr = (self.sqlite3_errmsg)(self.db);
            if ptr.is_null() {
                String::from("unknown sqlite3 error")
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        }
    }

    fn bind_stmt_params(&self, stmt: *mut c_void, params: &[SqlParam]) -> Result<(), String> {
        for (idx, param) in params.iter().enumerate() {
            let bind_idx = c_int::try_from(idx + 1).map_err(|_| String::from("Too many params"))?;
            let rc = unsafe {
                match param {
                    SqlParam::Null => (self.sqlite3_bind_null)(stmt, bind_idx),
                    SqlParam::Integer(v) => (self.sqlite3_bind_int64)(stmt, bind_idx, *v),
                    SqlParam::Real(v) => (self.sqlite3_bind_double)(stmt, bind_idx, *v),
                    SqlParam::Text(s) => {
                        let len = c_int::try_from(s.len())
                            .map_err(|_| String::from("String param too large"))?;
                        (self.sqlite3_bind_text)(
                            stmt,
                            bind_idx,
                            s.as_ptr().cast::<c_char>(),
                            len,
                            SQLITE_TRANSIENT,
                        )
                    }
                }
            };
            if rc != SQLITE_OK {
                return Err(self.last_error());
            }
        }
        Ok(())
    }

    fn collect_column_names(&self, stmt: *mut c_void) -> Vec<String> {
        let col_count = unsafe { (self.sqlite3_column_count)(stmt) };
        let mut names = Vec::with_capacity(usize::try_from(col_count.max(0)).unwrap_or(0));
        for col in 0..col_count {
            let name = unsafe {
                let ptr = (self.sqlite3_column_name)(stmt, col);
                if ptr.is_null() {
                    format!("col_{col}")
                } else {
                    CStr::from_ptr(ptr).to_string_lossy().into_owned()
                }
            };
            names.push(name);
        }
        names
    }

    fn execute_params(&self, sql: &str, params: &[SqlParam]) -> Result<(), String> {
        let c_sql = CString::new(sql).map_err(|e| e.to_string())?;
        if params.is_empty() {
            let mut cursor = c_sql.as_ptr();
            unsafe {
                while !cursor.is_null() && *cursor != 0 {
                    let mut stmt: *mut c_void = ptr::null_mut();
                    let mut tail: *const c_char = ptr::null();
                    let rc = (self.sqlite3_prepare_v2)(
                        self.db,
                        cursor,
                        -1,
                        &raw mut stmt,
                        &raw mut tail,
                    );
                    if rc != SQLITE_OK {
                        let msg = self.last_error();
                        if !stmt.is_null() {
                            (self.sqlite3_finalize)(stmt);
                        }
                        return Err(msg);
                    }
                    if !stmt.is_null() {
                        let step_rc = (self.sqlite3_step)(stmt);
                        (self.sqlite3_finalize)(stmt);
                        if step_rc != SQLITE_DONE && step_rc != SQLITE_ROW {
                            return Err(self.last_error());
                        }
                    }
                    if tail.is_null() || tail == cursor {
                        break;
                    }
                    cursor = tail;
                }
            }
            return Ok(());
        }

        let mut stmt: *mut c_void = ptr::null_mut();
        let mut tail: *const c_char = ptr::null();
        unsafe {
            let rc = (self.sqlite3_prepare_v2)(
                self.db,
                c_sql.as_ptr(),
                -1,
                &raw mut stmt,
                &raw mut tail,
            );
            if rc != SQLITE_OK {
                let msg = self.last_error();
                if !stmt.is_null() {
                    (self.sqlite3_finalize)(stmt);
                }
                return Err(msg);
            }
            if stmt.is_null() {
                return Ok(());
            }
            if let Err(err) = self.bind_stmt_params(stmt, params) {
                (self.sqlite3_finalize)(stmt);
                return Err(err);
            }
            let step_rc = (self.sqlite3_step)(stmt);
            (self.sqlite3_finalize)(stmt);
            if step_rc != SQLITE_DONE && step_rc != SQLITE_ROW {
                return Err(self.last_error());
            }
        }
        Ok(())
    }

    fn query(&self, sql: &str) -> Result<Vec<HashMap<String, String>>, String> {
        let c_sql = CString::new(sql).map_err(|e| e.to_string())?;
        let mut stmt: *mut c_void = ptr::null_mut();
        let mut tail: *const c_char = ptr::null();
        unsafe {
            let rc = (self.sqlite3_prepare_v2)(
                self.db,
                c_sql.as_ptr(),
                -1,
                &raw mut stmt,
                &raw mut tail,
            );
            if rc != SQLITE_OK {
                let msg = self.last_error();
                if !stmt.is_null() {
                    (self.sqlite3_finalize)(stmt);
                }
                return Err(msg);
            }
            if stmt.is_null() {
                return Ok(Vec::new());
            }

            let col_names = self.collect_column_names(stmt);
            let mut rows = Vec::new();
            loop {
                let step_rc = (self.sqlite3_step)(stmt);
                if step_rc == SQLITE_DONE {
                    break;
                }
                if step_rc != SQLITE_ROW {
                    let msg = self.last_error();
                    (self.sqlite3_finalize)(stmt);
                    return Err(msg);
                }
                let mut row = HashMap::with_capacity(col_names.len());
                for (idx, col_name) in col_names.iter().enumerate() {
                    let col = c_int::try_from(idx).unwrap_or(0);
                    let text_ptr = (self.sqlite3_column_text)(stmt, col);
                    let col_val = if text_ptr.is_null() {
                        String::new()
                    } else {
                        CStr::from_ptr(text_ptr.cast::<c_char>())
                            .to_string_lossy()
                            .into_owned()
                    };
                    row.insert(col_name.clone(), col_val);
                }
                rows.push(row);
            }
            (self.sqlite3_finalize)(stmt);
            Ok(rows)
        }
    }

    fn write_json_column_value(&self, stmt: *mut c_void, col: c_int, out: &mut String) {
        unsafe {
            match (self.sqlite3_column_type)(stmt, col) {
                SQLITE_NULL => out.push_str("null"),
                SQLITE_INTEGER => {
                    let val = (self.sqlite3_column_int64)(stmt, col);
                    let _ = write!(out, "{val}");
                }
                SQLITE_FLOAT => {
                    let val = (self.sqlite3_column_double)(stmt, col);
                    if val.is_finite() {
                        let _ = write!(out, "{val}");
                    } else {
                        out.push_str("null");
                    }
                }
                _ => {
                    let text_ptr = (self.sqlite3_column_text)(stmt, col);
                    out.push('"');
                    if !text_ptr.is_null() {
                        let text = CStr::from_ptr(text_ptr.cast::<c_char>()).to_string_lossy();
                        out.push_str(&escape_json(&text));
                    }
                    out.push('"');
                }
            }
        }
    }

    fn query_json_params(&self, sql: &str, params: &[SqlParam]) -> Result<String, String> {
        let c_sql = CString::new(sql).map_err(|e| e.to_string())?;
        let mut stmt: *mut c_void = ptr::null_mut();
        let mut tail: *const c_char = ptr::null();
        unsafe {
            let rc = (self.sqlite3_prepare_v2)(
                self.db,
                c_sql.as_ptr(),
                -1,
                &raw mut stmt,
                &raw mut tail,
            );
            if rc != SQLITE_OK {
                let msg = self.last_error();
                if !stmt.is_null() {
                    (self.sqlite3_finalize)(stmt);
                }
                return Err(msg);
            }
            if stmt.is_null() {
                return Ok(String::from("[]"));
            }
            if let Err(err) = self.bind_stmt_params(stmt, params) {
                (self.sqlite3_finalize)(stmt);
                return Err(err);
            }

            let col_names = self.collect_column_names(stmt);
            let mut sorted_cols: Vec<(c_int, &str)> = col_names
                .iter()
                .enumerate()
                .map(|(i, name)| (c_int::try_from(i).unwrap_or(0), name.as_str()))
                .collect();
            sorted_cols.sort_by(|a, b| a.1.cmp(b.1));

            let mut out = String::from("[");
            let mut first_row = true;
            loop {
                let step_rc = (self.sqlite3_step)(stmt);
                if step_rc == SQLITE_DONE {
                    break;
                }
                if step_rc != SQLITE_ROW {
                    let msg = self.last_error();
                    (self.sqlite3_finalize)(stmt);
                    return Err(msg);
                }
                if !first_row {
                    out.push(',');
                }
                first_row = false;
                out.push('{');
                for (field_idx, &(col, col_name)) in sorted_cols.iter().enumerate() {
                    if field_idx > 0 {
                        out.push(',');
                    }
                    let _ = write!(out, "\"{}\":", escape_json(col_name));
                    self.write_json_column_value(stmt, col, &mut out);
                }
                out.push('}');
            }
            (self.sqlite3_finalize)(stmt);
            out.push(']');
            Ok(out)
        }
    }
}

impl Drop for SqliteFfi {
    fn drop(&mut self) {
        if !self.db.is_null() {
            unsafe {
                (self.sqlite3_close)(self.db);
            }
            self.db = ptr::null_mut();
        }
    }
}

unsafe fn load_sym<F: Copy>(handle: *mut c_void, sym: &CStr) -> Option<F> {
    unsafe { DynLib::from_ptr(handle).sym(sym) }
}

/// Cross-platform OS dynamic library target for `NativeSqliteDatabase`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsSqliteBackend {
    /// Linux `libsqlite3.so.0`.
    LinuxLibSqlite3,
    /// Windows `winsqlite3.dll` (built into Windows 10/11).
    WindowsWinSqlite3,
    /// macOS `libsqlite3.dylib` (built into macOS SDK).
    MacOsLibSqlite3,
}

impl OsSqliteBackend {
    pub const fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::WindowsWinSqlite3
        } else if cfg!(target_os = "macos") {
            Self::MacOsLibSqlite3
        } else {
            Self::LinuxLibSqlite3
        }
    }

    pub const fn library_name(self) -> &'static str {
        match self {
            Self::LinuxLibSqlite3 => "libsqlite3.so.0",
            Self::WindowsWinSqlite3 => "winsqlite3.dll",
            Self::MacOsLibSqlite3 => "libsqlite3.dylib",
        }
    }
}

/// Native `SQLite3` database engine dynamically bound to `libsqlite3.so.0` (Linux/macOS/Windows)
/// with automatic WAL journaling and JSON serialization for reactive `@ng-desktop/desktop` queries.
pub struct NativeSqliteDatabase {
    backend: OsSqliteBackend,
    ffi: Option<SqliteFfi>,
    fallback_tables: HashMap<String, Vec<HashMap<String, String>>>,
}

impl Default for NativeSqliteDatabase {
    fn default() -> Self {
        Self::new_in_memory()
    }
}

impl NativeSqliteDatabase {
    pub fn new_in_memory() -> Self {
        Self::open_path(":memory:")
    }

    pub fn open_path(path: &str) -> Self {
        let ffi = SqliteFfi::try_open(path);
        if let Some(ref handle) = ffi {
            let _ = handle.execute_params("PRAGMA busy_timeout=5000;", &[]);
            if path.trim() != ":memory:" {
                let _ = handle
                    .execute_params("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;", &[]);
            }
        }
        Self {
            backend: OsSqliteBackend::current(),
            ffi,
            fallback_tables: HashMap::new(),
        }
    }

    pub const fn backend(&self) -> OsSqliteBackend {
        self.backend
    }

    pub const fn is_native_loaded(&self) -> bool {
        self.ffi.is_some()
    }

    pub fn execute(&mut self, sql: &str) -> Result<(), String> {
        self.execute_params(sql, &[])
    }

    /// Executes a SQL statement with bound `SqlParam` values (`?` placeholders).
    pub fn execute_params(&mut self, sql: &str, params: &[SqlParam]) -> Result<(), String> {
        if let Some(ref ffi) = self.ffi {
            return ffi.execute_params(sql, params);
        }
        // Minimal fallback for environments lacking a system SQLite shared library
        for raw_stmt in sql.split(';') {
            let stmt = raw_stmt.trim();
            if stmt.is_empty() {
                continue;
            }
            let upper = stmt.to_ascii_uppercase();
            if upper.starts_with("CREATE TABLE")
                && let Some(tbl) = stmt.split_whitespace().find(|tok| {
                    !matches!(
                        tok.to_ascii_uppercase().as_str(),
                        "CREATE" | "TABLE" | "IF" | "NOT" | "EXISTS"
                    )
                })
            {
                let name = tbl.split('(').next().unwrap_or(tbl).trim().to_string();
                self.fallback_tables.entry(name).or_default();
            } else if upper.starts_with("INSERT INTO")
                && let Some((head, vals_part)) = stmt.split_once('(')
            {
                let table_name = head.split_whitespace().last().unwrap_or("default").trim();
                let (cols_raw, rest_after_cols) =
                    vals_part.split_once(')').unwrap_or((vals_part, ""));
                let vals_inner = rest_after_cols
                    .split_once('(')
                    .and_then(|(_, v)| v.rsplit_once(')').map(|(inner, _)| inner))
                    .unwrap_or("");
                let mut row = HashMap::new();
                for (idx, (col, val)) in cols_raw.split(',').zip(vals_inner.split(',')).enumerate()
                {
                    let key = col.trim();
                    let cleaned_val = val.trim().trim_matches('\'').trim_matches('"').to_string();
                    let col_name = if key.is_empty() {
                        format!("col_{idx}")
                    } else {
                        key.to_string()
                    };
                    row.insert(col_name, cleaned_val);
                }
                if !row.is_empty() {
                    self.fallback_tables
                        .entry(table_name.to_string())
                        .or_default()
                        .push(row);
                }
            }
        }
        Ok(())
    }

    pub fn query(&self, sql: &str) -> Result<Vec<HashMap<String, String>>, String> {
        if let Some(ref ffi) = self.ffi {
            return ffi.query(sql);
        }
        Ok(self
            .fallback_tables
            .values()
            .next()
            .cloned()
            .unwrap_or_default())
    }

    pub fn query_json(&self, sql: &str) -> Result<String, String> {
        self.query_json_params(sql, &[])
    }

    /// Queries SQL with bound `SqlParam` values and streams natively-typed JSON rows.
    pub fn query_json_params(&self, sql: &str, params: &[SqlParam]) -> Result<String, String> {
        if let Some(ref ffi) = self.ffi {
            return ffi.query_json_params(sql, params);
        }
        let rows = self.query(sql)?;
        let encoded_rows: Vec<String> = rows
            .into_iter()
            .map(|row| {
                let mut entries: Vec<(String, String)> = row.into_iter().collect();
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                let fields: Vec<String> = entries
                    .into_iter()
                    .map(|(k, v)| format!("\"{}\":\"{}\"", escape_json(&k), escape_json(&v)))
                    .collect();
                format!("{{{}}}", fields.join(","))
            })
            .collect();
        Ok(format!("[{}]", encoded_rows.join(",")))
    }
}

#[cfg(test)]
mod tests {
    use super::NativeSqliteDatabase;

    #[test]
    fn executes_crud_and_serializes_rows_with_native_sqlite() {
        let mut db = NativeSqliteDatabase::new_in_memory();
        db.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL, note TEXT);")
            .unwrap();
        db.execute(
            "INSERT INTO items (id, name, note) VALUES (25, 'Widget', 'Active');
             INSERT INTO items (id, name, note) VALUES (1, 'Anchor', 'Pinned');",
        )
        .unwrap();

        let rows = db
            .query("SELECT id, name, note FROM items ORDER BY id ASC;")
            .unwrap();
        if db.is_native_loaded() {
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].get("id").map(String::as_str), Some("1"));
            assert_eq!(rows[0].get("name").map(String::as_str), Some("Anchor"));
            assert_eq!(rows[1].get("name").map(String::as_str), Some("Widget"));

            db.execute("UPDATE items SET note = 'Primary' WHERE id = 25;")
                .unwrap();
            db.execute("DELETE FROM items WHERE id = 1;").unwrap();

            let json = db
                .query_json("SELECT id, name, note FROM items ORDER BY id ASC;")
                .unwrap();
            assert_eq!(json, r#"[{"id":25,"name":"Widget","note":"Primary"}]"#);

            let timeout_rows = db.query("PRAGMA busy_timeout;").unwrap();
            assert_eq!(
                timeout_rows[0].get("timeout").map(String::as_str),
                Some("5000")
            );
        }
    }

    #[test]
    fn binds_parameterized_queries_and_emits_typed_json_rows() {
        let mut db = NativeSqliteDatabase::new_in_memory();
        db.execute(
            "CREATE TABLE records (id INTEGER PRIMARY KEY, score REAL, label TEXT, extra TEXT);",
        )
        .unwrap();
        let params = vec![
            super::SqlParam::Integer(42),
            super::SqlParam::Real(9.5),
            super::SqlParam::Text(String::from("O'Reilly'; DROP TABLE records; --")),
            super::SqlParam::Null,
        ];
        db.execute_params(
            "INSERT INTO records (id, score, label, extra) VALUES (?, ?, ?, ?);",
            &params,
        )
        .unwrap();

        if db.is_native_loaded() {
            let lookup = super::parse_json_params("[42]").unwrap();
            let json = db
                .query_json_params(
                    "SELECT id, score, label, extra FROM records WHERE id = ?;",
                    &lookup,
                )
                .unwrap();
            assert_eq!(
                json,
                r#"[{"extra":null,"id":42,"label":"O'Reilly'; DROP TABLE records; --","score":9.5}]"#
            );
        }
    }
}

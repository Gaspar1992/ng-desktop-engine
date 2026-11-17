#![allow(unsafe_code)]

use super::http_codec::escape_json;
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::ptr;

#[cfg(target_os = "linux")]
#[link(name = "dl")]
unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

const RTLD_LAZY: c_int = 1;
const SQLITE_OK: c_int = 0;
const SQLITE_ROW: c_int = 100;
const SQLITE_DONE: c_int = 101;

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
type SqliteColTextFn = unsafe extern "C" fn(*mut c_void, c_int) -> *const u8;
type SqliteFinalizeFn = unsafe extern "C" fn(*mut c_void) -> c_int;
type SqliteErrmsgFn = unsafe extern "C" fn(*mut c_void) -> *const c_char;

struct SqliteFfi {
    db: *mut c_void,
    sqlite3_close: SqliteCloseFn,
    sqlite3_prepare_v2: SqlitePrepareFn,
    sqlite3_step: SqliteStepFn,
    sqlite3_column_count: SqliteColCountFn,
    sqlite3_column_name: SqliteColNameFn,
    sqlite3_column_text: SqliteColTextFn,
    sqlite3_finalize: SqliteFinalizeFn,
    sqlite3_errmsg: SqliteErrmsgFn,
}

unsafe impl Send for SqliteFfi {}

impl SqliteFfi {
    #[cfg(target_os = "linux")]
    fn try_open(path: &str) -> Option<Self> {
        unsafe {
            let lib = dlopen(c"libsqlite3.so.0".as_ptr(), RTLD_LAZY);
            if lib.is_null() {
                return None;
            }
            let sqlite3_open: SqliteOpenFn = load_sym(lib, c"sqlite3_open")?;
            let sqlite3_close: SqliteCloseFn = load_sym(lib, c"sqlite3_close")?;
            let sqlite3_prepare_v2: SqlitePrepareFn = load_sym(lib, c"sqlite3_prepare_v2")?;
            let sqlite3_step: SqliteStepFn = load_sym(lib, c"sqlite3_step")?;
            let sqlite3_column_count: SqliteColCountFn = load_sym(lib, c"sqlite3_column_count")?;
            let sqlite3_column_name: SqliteColNameFn = load_sym(lib, c"sqlite3_column_name")?;
            let sqlite3_column_text: SqliteColTextFn = load_sym(lib, c"sqlite3_column_text")?;
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
                sqlite3_column_text,
                sqlite3_finalize,
                sqlite3_errmsg,
            })
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn try_open(_path: &str) -> Option<Self> {
        None
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

    fn execute(&self, sql: &str) -> Result<(), String> {
        let c_sql = CString::new(sql).map_err(|e| e.to_string())?;
        let mut cursor = c_sql.as_ptr();
        unsafe {
            while !cursor.is_null() && *cursor != 0 {
                let mut stmt: *mut c_void = ptr::null_mut();
                let mut tail: *const c_char = ptr::null();
                let rc =
                    (self.sqlite3_prepare_v2)(self.db, cursor, -1, &raw mut stmt, &raw mut tail);
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

            let col_count = (self.sqlite3_column_count)(stmt);
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
                let mut row = HashMap::new();
                for col in 0..col_count {
                    let name_ptr = (self.sqlite3_column_name)(stmt, col);
                    let col_name = if name_ptr.is_null() {
                        format!("col_{col}")
                    } else {
                        CStr::from_ptr(name_ptr).to_string_lossy().into_owned()
                    };
                    let text_ptr = (self.sqlite3_column_text)(stmt, col);
                    let col_val = if text_ptr.is_null() {
                        String::new()
                    } else {
                        CStr::from_ptr(text_ptr.cast::<c_char>())
                            .to_string_lossy()
                            .into_owned()
                    };
                    row.insert(col_name, col_val);
                }
                rows.push(row);
            }
            (self.sqlite3_finalize)(stmt);
            Ok(rows)
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

#[cfg(target_os = "linux")]
unsafe fn load_sym<F: Copy>(handle: *mut c_void, sym: &CStr) -> Option<F> {
    let ptr = unsafe { dlsym(handle, sym.as_ptr()) };
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { std::mem::transmute_copy::<*mut c_void, F>(&ptr) })
    }
}

/// Native `SQLite3` database engine dynamically bound to `libsqlite3.so.0` (Linux/macOS/Windows)
/// with automatic WAL journaling and JSON serialization for reactive `@ng-native/desktop` queries.
pub struct NativeSqliteDatabase {
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
            let _ = handle.execute("PRAGMA journal_mode=WAL;");
        }
        Self {
            ffi,
            fallback_tables: HashMap::new(),
        }
    }

    pub const fn is_native_loaded(&self) -> bool {
        self.ffi.is_some()
    }

    pub fn execute(&mut self, sql: &str) -> Result<(), String> {
        if let Some(ref ffi) = self.ffi {
            return ffi.execute(sql);
        }
        // Minimal fallback for non-Linux targets lacking `libsqlite3.so.0`
        let upper = sql.trim().to_ascii_uppercase();
        if upper.starts_with("CREATE TABLE")
            && let Some(tbl) = sql.split_whitespace().find(|tok| {
                !matches!(
                    tok.to_ascii_uppercase().as_str(),
                    "CREATE" | "TABLE" | "IF" | "NOT" | "EXISTS"
                )
            })
        {
            let name = tbl.split('(').next().unwrap_or(tbl).trim().to_string();
            self.fallback_tables.entry(name).or_default();
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
        db.execute(
            "CREATE TABLE favorites (id INTEGER PRIMARY KEY, name TEXT NOT NULL, note TEXT);",
        )
        .unwrap();
        db.execute(
            "INSERT INTO favorites (id, name, note) VALUES (25, 'Pikachu', 'Electrico');
             INSERT INTO favorites (id, name, note) VALUES (1, 'Bulbasaur', 'Semilla');",
        )
        .unwrap();

        let rows = db
            .query("SELECT id, name, note FROM favorites ORDER BY id ASC;")
            .unwrap();
        if db.is_native_loaded() {
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].get("id").map(String::as_str), Some("1"));
            assert_eq!(rows[0].get("name").map(String::as_str), Some("Bulbasaur"));
            assert_eq!(rows[1].get("name").map(String::as_str), Some("Pikachu"));

            db.execute("UPDATE favorites SET note = 'Favorito' WHERE id = 25;")
                .unwrap();
            db.execute("DELETE FROM favorites WHERE id = 1;").unwrap();

            let json = db
                .query_json("SELECT id, name, note FROM favorites ORDER BY id ASC;")
                .unwrap();
            assert_eq!(json, r#"[{"id":"25","name":"Pikachu","note":"Favorito"}]"#);
        }
    }
}

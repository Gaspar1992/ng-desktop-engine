use std::fs;

/// Reads a self-contained `.ngb` fixture bundle from disk for Rust integration tests.
#[allow(dead_code, unreachable_pub)]
pub fn load_fixture_bundle(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to load fixture bundle at '{path}': {e}"))
}

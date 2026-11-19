# Security Policy

The `NgDesktop Engine` (`ng-desktop-engine`) maintainers take security and memory safety seriously. Because this project bridges Angular 22+ TypeScript applications directly to native OS libraries via Rust FFI, we follow strict compile-time and runtime guarantees and encourage responsible disclosure.

---

## Supported Versions

Only the active release line receives security patches and dependency updates:

| Version / Branch                 | Status         | Support Scope                               |
| :------------------------------- | :------------- | :------------------------------------------ |
| `0.2.x` (`main` & `v0.2.x` tags) | ✅ Supported   | Security fixes, FFI hardening, & OS patches |
| `< 0.2.0`                        | ❌ Unsupported | Historical preview releases                 |

---

## Reporting a Vulnerability (Private Disclosure)

**Please DO NOT open public GitHub Issues, Discussions, or Pull Requests for security vulnerabilities.**

Instead, report security findings confidentially through **GitHub Private Vulnerability Reporting**:

👉 **[Report a Security Vulnerability Privately](https://github.com/Gaspar1992/ng-desktop-engine/security/advisories/new)**

Please include as much of the following information as possible:

1. Affected component (`ng-desktop-engine` Rust core, `@ng-native/platform`, `@ng-native/forms`, `@ng-native/router`, `@ng-native/http`, `@ng-native/desktop`, `@ng-native/builders`, or packaged installers `.deb` / `.exe` / `.dmg`).
2. Operating system, architecture, display server (Wayland / X11 / WinUI 3 / AppKit), and Rust / Node / Angular versions.
3. Minimal reproduction steps, `.ngb` payload, or proof-of-concept exploit.
4. Potential impact (e.g., memory corruption across FFI, path traversal, arbitrary command execution, TLS/XSRF bypass).

### Response SLAs

- **Acknowledgment:** Within **48 hours** of submission.
- **Triage & Severity Assessment:** Within **7 days**, including confirmation and CVSS/severity classification.
- **Coordinated Fix & Disclosure:** Once patched and verified across Linux, Windows, and macOS CI targets, we will publish a GitHub Security Advisory and release update crediting the reporter (unless anonymity is requested).

---

## Security Architecture & Threat Model

`ng-desktop-engine` enforces defense-in-depth across every layer of the Rust / Angular boundary:

1. **Memory Safety & Generational Arena ([`src/engine/arena.rs`](./src/engine/arena.rs)):**
   - `#![deny(unsafe_code)]` is enforced globally in [`Cargo.toml`](./Cargo.toml).
   - JavaScript never holds raw C pointers (`*mut c_void`) to GTK4, WinUI 3, or AppKit objects. Instead, Angular interacts strictly with opaque generational `NodeId { index, generation }` handles validated by runtime `BorrowState` (`Shared(usize)` vs. `Exclusive`) checks to prevent use-after-free and aliasing bugs.
2. **Isolated Dynamic C FFI ([`src/ffi_dl.rs`](./src/ffi_dl.rs) & `*_sys.rs`):**
   - All `dlopen` / `dlsym` symbol lookups (`libgtk-4.so.1`, `libadwaita-1.so.0`, `libjavascriptcoregtk-6.0.so.1`, `libsoup-3.0.so.0`, `libsqlite3.so.0`, `libc.so.6` `inotify`) are encapsulated inside audited FFI boundary modules via the centralized `DynLib` wrapper.
3. **File System & XDG Path Traversal Defense ([`src/desktop/fs_watcher.rs`](./src/desktop/fs_watcher.rs)):**
   - `NativeFsManager::resolve_xdg_path` rejects `..` (`Component::ParentDir`), leading roots (`Component::RootDir`), and empty paths before resolving XDG `Config`, `Data`, or `Cache` locations.
   - File persistence (`write_text_atomic`) uses sibling temporary files (`.tmp.<pid>`) and atomic `fs::rename` operations.
4. **Native HTTP/2 & XSRF Controls ([`src/desktop/http_native.rs`](./src/desktop/http_native.rs)):**
   - System TLS and HTTP/2 connections are managed through OS libraries (`libsoup-3.0`, `WinHTTP`, `URLSession`), supporting Angular's `withXsrfConfiguration` and header sanitization.
5. **Local Development HMR IPC ([`src/ipc/mod.rs`](./src/ipc/mod.rs)):**
   - Unix domain socket hot-reload (`dist/<project>/hmr.sock`) binds strictly to the local user's filesystem for development workflows and cleans up socket descriptors on drop.

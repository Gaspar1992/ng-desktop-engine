# Contributing to NgDesktop Engine (`ng-desktop-engine`)

Thank you for your interest in contributing to **NgDesktop Engine**, the Rust-powered native desktop rendering engine and Angular 22+ package suite (`@ng-desktop/*`)!

---

## Repository Architecture

This monorepo combines two tightly coupled layers:

- **Rust Core & Native Backends ([`src/`](./src)):**
  - `src/engine/`: 100% safe Rust scene graph, generational `Arena`, `RenderCommand` queue, style registry, and borrow checker (`unsafe_code = "deny"`).
  - `src/platforms/`: OS adapters (`GtkLinuxBackend`, `WinUiBackend`, `MacOsAppKitBackend`, `HeadlessBackend`) + isolated GTK4 FFI (`gtk4_sys.rs`, `gtk4_callbacks.rs`).
  - `src/runtime/`: Pluggable JS engines (`JscEngine` via `libjavascriptcoregtk-6.0.so.1` and zero-JIT `QuickJsEngine`), `.ngb` parser, reactivity engine, and HMR.
  - `src/desktop/`: Native OS services (HTTP/2 via `libsoup-3.0`, SQLite3 WAL via `libsqlite3`, GLib/GIO system env, XDG + `inotify` watcher, multithreaded compute pool).
  - `src/ffi_dl.rs`: Centralized `DynLib` dynamic symbol loader.
  - `src/packager/`: Multi-OS packaging (`.deb`, AppImage, Flatpak, Windows WinUI 3 MSIX manifest, macOS `.app` / `.dmg`).
- **Angular 22+ Packages ([`packages/`](./packages)) & Examples ([`examples/`](./examples)):**
  - `@ng-desktop/platform`, `@ng-desktop/forms`, `@ng-desktop/router`, `@ng-desktop/http`, `@ng-desktop/desktop`, `@ng-desktop/builders`.

---

## Prerequisites & Local Setup

- **Rust:** `>= 1.87` (`edition = "2024"`, recommended **`1.100.0`** with `clippy` and `rustfmt`)
- **Node.js:** `>= 24` & **pnpm:** `>= 11` (`corepack enable`)
- **Linux Native Libraries (for live GTK4 GUI & full FFI tests):**
  - Ubuntu / Debian: `libgtk-4-1`, `libadwaita-1-0`, `libjavascriptcoregtk-6.0-1`, `libsoup-3.0-0`, `libsqlite3-0`, `xvfb`, `xauth`
- **Multi-OS Containers (optional):** Docker or Podman for [`scripts/test-containers.sh`](./scripts/test-containers.sh)

### Development Commands

```bash
# 1. Install TypeScript & Angular dependencies
pnpm install

# 2. Build Angular bundles & compile .ngb IR
pnpm run build

# 3. Watch Angular sources & hot-reload native GTK4 window over Unix socket IPC
pnpm run dev

# 4. Auto-format both TypeScript/Markdown/YAML (oxfmt) and Rust (cargo fmt)
pnpm run format:all

# 5. Run the complete verification gate (MUST PASS before opening a PR)
pnpm run check

# 6. Run cross-OS container test suites (Ubuntu 24.04, Fedora 41, Windows Wine64, macOS cross-target)
pnpm run test:containers
```

---

## Engineering & SOLID Standards

Every pull request must uphold the following architectural rules:

1. **Zero Unsafe Code in Core Engine (`src/engine/`, `src/config/`, `src/packager/`, `src/desktop/compute_pool.rs`):**
   - `Cargo.toml` enforces `unsafe_code = "deny"`, `clippy::pedantic`, and `clippy::nursery`.
   - New OS C FFI bindings must live in an isolated `*_sys.rs` module and load symbols through [`src/ffi_dl.rs`](./src/ffi_dl.rs) (`DynLib`).
2. **Single Responsibility Principle (SRP) & Function Size:**
   - Keep functions focused, shallow, and `<= 100` lines (enforced by Clippy `too_many_lines`). Extract helpers rather than duplicating logic.
3. **Cross-Platform Parity Across Backends:**
   - Every new widget primitive or attribute must map cleanly across `PlatformBackend::map_native_widget`, `GtkLinuxBackend`, `WinUiBackend`, `MacOsAppKitBackend`, and `HeadlessBackend`.
   - New desktop OS abstractions should expose explicit OS backend identifiers (e.g., `Linux...`, `Windows...`, `MacOs...`) and update [`tests/test_cross_platform.rs`](./tests/test_cross_platform.rs).
   - Verify Windows and macOS targets compile cleanly with zero Clippy warnings:
     ```bash
     cargo clippy --target x86_64-pc-windows-gnu --all-targets -- -D warnings
     cargo clippy --target x86_64-apple-darwin --all-targets -- -D warnings
     ```
4. **Angular `>=22.0.0` Compatibility:**
   - Keep `peerDependencies` across `@ng-desktop/*` set to `">=22.0.0"` (never hard-pin a minor release) so external Angular 22+ applications can upgrade seamlessly.
5. **Preserve Existing Frame Invariants:**
   - Ensure any compiler changes continue to pass [`tests/test_pipeline_and_hmr.rs`](./tests/test_pipeline_and_hmr.rs) and all Vitest suites.

---

## Reporting Bugs, Requesting Features, & Security

- **Bugs & Regressions:** Open a [Bug Report](https://github.com/Gaspar1992/ng-desktop-engine/issues/new?template=bug_report.yml) with your OS, JS engine (`jsc`/`quickjs`), Angular/Rust versions, and minimal reproduction.
- **Feature Proposals:** Open a [Feature Request](https://github.com/Gaspar1992/ng-desktop-engine/issues/new?template=feature_request.yml) specifying how the feature maps natively on Linux, Windows, and macOS.
- **Cross-Platform Parity Gaps:** Use the [Platform Parity Issue Form](https://github.com/Gaspar1992/ng-desktop-engine/issues/new?template=platform_parity.yml).
- **Security Vulnerabilities:** **Never open a public issue.** Follow [`SECURITY.md`](./SECURITY.md) to submit a private GitHub Security Advisory.

---

## Commit Convention & Milestone Releases

This repository follows [Conventional Commits](https://www.conventionalcommits.org/):

```text
<type>(<scope>): <short imperative description>
```

- **Types:** `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `ci`, `chore`
- **Scopes:** `engine`, `platforms`, `desktop`, `http`, `router`, `forms`, `builders`, `demo`, `ci`, `release`, `governance`

### How Milestone Releases Work

When a major milestone is complete, pushing a version tag (`vX.Y.Z`) triggers [`.github/workflows/release.yml`](./.github/workflows/release.yml) and [`scripts/generate-release-notes.mjs`](./scripts/generate-release-notes.mjs) to publish a GitHub Release and verify/publish packages via **tokenless OIDC Trusted Publishing**:

1. **`vX.Y.Z` (Engine + Individual `@ng-desktop/*` Packages):** Rust binaries + standalone `.tgz` tarballs for `@ng-desktop/{platform,forms,router,http,desktop,builders}` and `@ng-desktop/engine-*` with Angular-style architecture diagrams, categorized changelogs, and Sigstore SLSA build provenance attestations (`actions/attest@v4`).
2. **Tokenless OIDC Publishing (`npm` & `crates.io`):**
   - `publish-npm` runs inside GitHub Environment `npm-publish` (`id-token: write`, `npm >= 11.5.1`, no `NPM_TOKEN` secret) once `vars.NPM_TRUSTED_PUBLISHING == 'true'`. Initial one-time package creation is assisted by `pnpm run publish:bootstrap`.
   - `publish-crates` runs inside GitHub Environment `crates-publish` using `rust-lang/crates-io-auth-action@v1` once `vars.CRATES_TRUSTED_PUBLISHING == 'true'`.

All contributors agree to abide by our [Code of Conduct](./CODE_OF_CONDUCT.md) and license their contributions under the [MIT License](./LICENSE).

## Summary

<!-- Describe what this PR changes, why it is needed, and link any related issues (e.g. Closes #12). -->

## Affected Areas

- [ ] `src/engine/` (Scene Graph, Generational Arena, Styles, Icons)
- [ ] `src/platforms/` (Linux GTK4, Windows WinUI 3, macOS AppKit, Headless)
- [ ] `src/runtime/` (JSC / QuickJS Bridges, `.ngb` Parser, Reactivity, HMR)
- [ ] `src/desktop/` (HTTP/2, SQLite3, XDG/`inotify` Watcher, Compute Pool, System Env)
- [ ] `packages/*` (`@ng-desktop/{platform,forms,router,http,desktop,builders}`)
- [ ] `examples/counter-app` (Pokédex Showcase Demo)
- [ ] `.github/`, `docker/`, `scripts/`, or documentation

## Architecture & SOLID Checklist

- [ ] Core engine changes contain **zero `unsafe` code** (`#![deny(unsafe_code)]`), and any OS FFI uses [`src/ffi_dl.rs`](./src/ffi_dl.rs) inside an isolated `*_sys.rs` module.
- [ ] New functions and modules adhere to SRP (`<= 100` lines; zero Clippy `pedantic` / `nursery` warnings).
- [ ] Any new widget tag, attribute, or OS service maintains **cross-platform parity** across `GtkLinuxBackend`, `WinUiBackend`, `MacOsAppKitBackend`, and `HeadlessBackend`.
- [ ] `@ng-desktop/*` packages preserve `peerDependencies: ">=22.0.0"` Angular compatibility.
- [ ] Unit and/or integration tests were added for new behavior (`cargo test` and Vitest).

## Verification Results

<!-- Paste or confirm that all checks passed locally: -->

- [ ] `pnpm run format:all` (`oxfmt` + `cargo fmt --all`)
- [ ] `pnpm run check` (`format:check`, `oxlint`, `eslint`, Vitest, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`)
- [ ] `cargo clippy --target x86_64-pc-windows-gnu --all-targets -- -D warnings`
- [ ] `cargo clippy --target x86_64-apple-darwin --all-targets -- -D warnings`

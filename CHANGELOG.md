# Changelog

All notable changes and completed roadmap tasks for `ng-native-engine` are documented in this file.
This project adheres to [Conventional Commits](https://www.conventionalcommits.org/) and [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Features (Phase 10 / Nivel 3 — Multithreaded Compute Pool, XDG & `inotify` Watcher, Native Drag & Drop, Cross-Platform Parity, & Multi-Container Testing)

- **compute & fs (Nivel 3):** add `src/desktop/compute_pool.rs` (`NativeComputePool` multithreaded fuzzy search + 64-bit FNV-1a hashing), `src/desktop/fs_watcher.rs` (`NativeFsManager` with XDG Base Directory traversal validation, atomic file writes, and non-blocking Linux `inotify` kernel watcher via `libc.so.6`), `packages/desktop-native/src/compute-service.ts` (`NativeComputeService`), `packages/desktop-native/src/fs-service.ts` (`NativeFileSystemService`), and `packages/desktop-native/src/drop-target.directive.ts` (`NativeDropTargetDirective`).
- **cross-platform & solid:** extract `src/platforms/gtk4_controls.rs` from `gtk4_widgets.rs` (SRP); map all 27 primitives in `PlatformBackend::map_native_widget`; synchronize `drop-target`, `scroll-to`, `scroll-anchor`, and native event queuing across `GtkLinuxBackend`, `WinUiBackend`, and `MacOsAppKitBackend` (`NativeSceneState`); add explicit OS service driver enums (`OsFileWatcherBackend`, `OsSqliteBackend`, `OsSystemEnvBackend`, `OsHttpPlatform`).
- **containers & ci:** add multi-container test harness (`docker/Dockerfile.ubuntu-gtk4`, `docker/Dockerfile.fedora-gtk4`, `scripts/test-containers.sh`), run `xvfb-run -a cargo test` in `.github/workflows/ci.yml`, and expand test suite to **102 passing tests** (40 Rust unit + 19 Rust integration + 43 Vitest).

### Features (Phase 9 — OS-Native HTTP/2, `@ng-native/builders`, Reactive Control Flow, SQLite FFI, & Angular Platform Integration)

- **http:** add OS-native `libsoup-3.0` HTTP/2 transport (`src/desktop/libsoup_sys.rs`, `src/desktop/http_native.rs`, `src/desktop/http_codec.rs`), JSC FFI callbacks (`__NG_NATIVE_HOST_HTTP__`, `__NG_NATIVE_HOST_HTTP_CONFIG__`), `provideHttpClient(...)` Angular feature mapping + `createNativeHttpResource` in `@ng-native/http`, and parallel HTTP/2 batch transfers.
- **builders:** extract compiler toolchain (`ast-metadata-extractor`, `template-compiler`, `style-compiler`, `icon-compiler`, `build-bundle`, `dev-server`, `hmr-client`) from `examples/counter-app` into `@ng-native/builders`; add minimal OS/CPU `angular-native-schema.json` + `native-builder-schema.json`; migrate Angular pipes (`uppercase`, `lowercase`, `titlecase`, `number`, `currency`, `percent`, `slice`, `json`, `default`), control flow (`@let`, `@for`/`@empty`, `@switch`, `@defer`/`@error`), `computed()`, `linkedSignal()`, `input()`, `model()`, `output()`, `httpResource()`, and runtime `bind_visible`/`bind_attr` opcodes.
- **desktop & platform (Nivel 1 & Nivel 2):** add `libsqlite3.so.0` FFI engine (`src/desktop/sqlite_sys.rs`) + `NativeSqliteService` (`liveQuery`), `GNetworkMonitor` + `GLib` locale/timezone probe (`src/desktop/system_env.rs`) + `NativeNetworkService`, `provideNativeImageLoader` (`IMAGE_LOADER`), `NativeTitleService` (`Title`) + `NativeTitleStrategy` (`TitleStrategy`), `NativeStackRouteReuseStrategy` (`RouteReuseStrategy`), `NativeViewportScroller` (`ViewportScroller`), `NativeKeyEventsPlugin` (`EVENT_MANAGER_PLUGINS`), `provideSystemLocale` (`LOCALE_ID` + `DATE_PIPE_DEFAULT_TIMEZONE`), and `NativeSystemErrorHandler` (`ErrorHandler`).
- **pokedex & ci:** wire `PokedexService` (`examples/counter-app/src/app/pokedex.service.ts`) with SQLite favorites, native window titles, and HTTP batching; install Linux native shared libraries (`libgtk-4-1`, `libadwaita-1-0`, `libjavascriptcoregtk-6.0-1`, `libsoup-3.0-0`, `libsqlite3-0`) and Vitest in `.github/workflows/ci.yml`; expand test coverage to 93 tests (38 Rust unit + 16 Rust integration + 39 Vitest).

### Features (Phase 8 — Advanced Layout Containers, Extended Primitives, & Pokédex Split View)

- **widgets:** add `<ng-paned>` (`GtkPaned` / `SplitView` / `NSSplitView`), `<ng-stack>` (`GtkStack` / `NavigationView` / `NSTabView`), `<ng-expander>` (`GtkExpander` / `Expander` / `NSDisclosureButton`), `<ng-text-view>` (`GtkTextView` / `RichEditBox` / `NSTextView`), `<ng-password-entry>` (`GtkPasswordEntry` / `PasswordBox` / `NSSecureTextField`), `<ng-search-entry>` (`GtkSearchEntry` / `AutoSuggestBox` / `NSSearchField`), `<ng-spin-button>` (`GtkSpinButton` / `NumberBox` / `NSStepper`), and `<ng-clamp>` (`AdwClamp` / `Grid` / `NSBox`) across `WidgetKind`, `widget_attrs.rs`, `preview.rs`, `Gtk4Sys`, `GtkLinuxBackend`, `WinUiBackend`, and `MacOsAppKitBackend`.
- **forms:** extend `NativeTextValueAccessor` (`<ng-search-entry>`, `<ng-password-entry>`, `<ng-text-view>`) and `NativeNumberValueAccessor` (`<ng-spin-button>`) in `@ng-native/forms`.
- **pokedex:** upgrade `examples/counter-app` and `PokedexController` with `<ng-clamp>`, `<ng-search-entry>`, `<ng-spin-button>` ID stepper, `<ng-paned>` master-detail split layout, collapsible `<ng-expander>` panels, and `<ng-text-view>` trainer notes.
- **testing:** add unit, `@ng-native/forms`, cross-platform, and scene-graph/pipeline E2E tests for all 27 primitives, bringing total coverage to 70 automated tests (27 Rust unit tests + 15 Rust E2E tests + 28 Vitest tests).

### Features (Phase 7 — AST-Driven Toolchain, Real-Time Socket HMR, & SRP Hardening)

- **ipc:** create `src/ipc/mod.rs` implementing non-blocking Unix Domain Socket `HmrIpcServer` & `HmrIpcClient` with framed JSON messages (`HmrMessage::Reload`, `HmrMessage::Eval`, `HmrMessage::Ping`) and automatic cleanup of stale socket files.
- **hmr:** add `examples/counter-app/hmr-client.mjs` and wire live bundle push directly into `build-bundle.mjs` and `dev-server.mjs`, eliminating file-system polling race conditions during development.
- **compiler:** implement `ast-metadata-extractor.mjs` using Rust-based `oxc-parser` (`parseSync`) for TypeScript 6.0 AST extraction, `template-compiler.mjs` using `@angular/compiler`'s official `parseTemplate` AST, and `style-compiler.mjs` using `postcss` AST (zero regex parsing).
- **styles:** extend `StyleRule` and GTK4 CSS generator with `border-radius`, `font-size`, and `font-weight`, maintaining 100% backward compatibility with legacy 6-token style instructions.
- **platforms:** extract DRY cross-platform state machine `src/platforms/scene_state.rs` (`NativeSceneState` + `NativeSceneNode`) shared by `WinUiBackend` and `MacOsAppKitBackend`.
- **runtime:** refactor `src/runtime/mod.rs` into `js_bridge.rs` (JS eval & event routing), `reactivity.rs` (Signal propagation, text bindings & `set_signal`), and `scene_exec.rs` (scene instruction execution).
- **engine:** extract layout and interactive attribute mutators into `src/engine/widget_attrs.rs`.
- **widgets:** add `<ng-spinner>` (`GtkSpinner`), standalone `<ng-progress-bar>` (`GtkProgressBar` with fraction clamping and indeterminate pulsing), `<ng-image>` (`GtkPicture`), `<ng-level-bar>` (`GtkLevelBar`), and `<ng-flow>` (`GtkFlowBox`) across `WidgetKind`, `GtkLinuxBackend`, `WinUiBackend`, and `MacOsAppKitBackend`.
- **desktop:** implement live PokéAPI HTTP client (`src/desktop/pokedex.rs`) and async `PokedexController` (`src/desktop/pokedex_ui.rs`) with worker-thread HTTP requests, 500ms search debounce, `<ng-spinner>` loading indicator, and `JsRuntime::set_signal`.
- **compiler:** support modular Angular components with `<ng-content>` projection and shared `idState` counter in `template-compiler.mjs` (`template-compiler.d.mts` + `template-compiler.spec.ts`) so auto-generated node IDs never collide across nested component expansions.
- **testing:** split integration tests into 5 modular suites under `tests/` (`test_scene_graph.rs`, `test_pipeline_and_hmr.rs`, `test_forms_and_desktop.rs`, `test_js_engines.rs`, `test_cross_platform.rs`) and expand coverage to 66 automated tests (25 Rust unit tests + 14 Rust E2E tests + 27 Vitest tests).

### Features (Phase 6 — `@ng-native/builders`, Linux Distribution Packager, & Stateful Cross-Platform Backends)

- **packager:** add `src/packager/mod.rs` (`stage_linux_distribution`, `generate_debian_control`, `generate_desktop_entry`, `generate_appimage_apprun`, `generate_flatpak_manifest`) and `--package` CLI flag (`pnpm run package:linux`).
- **builders:** create `@ng-native/builders` (`packages/builders-native`) providing `createNativeTargetConfig` and `executeNativeBuilder` for Angular 22 CLI Architect integration.
- **platforms:** upgrade `WinUiBackend` (`NativeWinUiControl`) and `MacOsAppKitBackend` (`NativeAppKitView`) to maintain full stateful native widget trees and attributes.
- **compiler:** modularize `examples/counter-app/build-bundle.mjs` by extracting `examples/counter-app/icon-compiler.mjs` and adding Phase 4 interactive event mappings (`change` -> `toggled`, `valueChange` -> `value-changed`).

### Features (Angular 22 Upgrade & Post-Phase 5 Hardening)

- **angular:** upgrade workspace and all `@ng-native/*` packages (`platform`, `forms`, `router`, `http`, `desktop`) to **Angular 22 (`22.2.5`)**, `@angular/build` `22.2.6` (powered by Rust `rolldown` `1.2.16` + `oxc-parser` `0.156.0` + `vite` `8.3.5`), and `typescript` `6.0.3` (removing deprecated `baseUrl` in `tsconfig.json` for TS 6/7 compatibility).
- **runtime:** synchronize `DesktopServices` directly inside `JsRuntime` (`theme` & `clipboard` opcodes), extract `src/runtime/bootstrap_js.rs`, and extract GTK4 CSS/attribute styling into `src/platforms/gtk4_styling.rs` (`8d4aab9`).

### Features (Phase 5 — `@ng-native/router`, `@ng-native/http`, & `@ng-native/desktop`)

- **router:** create `@ng-native/router` (`packages/router-native`) implementing DOM-free `NativePlatformLocation` (in-memory URL/state history stack, `pushState`, `replaceState`, `back`, `forward`, `onPopState`) and `provideNativeRouter()`.
- **http:** create `@ng-native/http` (`packages/http-native`) implementing `NativeHttpBackend` and `provideNativeHttpClient()`, compatible with `HttpClient`, `withInterceptors()`, and `httpResource()`.
- **desktop:** create `src/desktop/mod.rs` (`DesktopServices`) and `@ng-native/desktop` (`packages/desktop-native`) providing `NativeDesktopService` with reactive `theme` Signal (`'dark' | 'light'`), clipboard access, desktop notifications, and file dialogs.
- **platforms:** extract GTK4 C signal trampolines into `src/platforms/gtk4_callbacks.rs`, extract `QuickJsEngine` into `src/runtime/quickjs_engine.rs`, and wire live GTK4 widget getters (`gtk_editable_get_text`, `gtk_switch_get_active`, `gtk_check_button_get_active`, `gtk_range_get_value`, `gtk_drop_down_get_selected`, `gtk_string_list_new`) into `GtkLinuxBackend` (`8e467a7`).

### Features (Phase 4 — Interactive Controls & `@ng-native/forms`)

- **widgets:** add `<ng-entry>` (`GtkEntry`), `<ng-switch>` (`GtkSwitch`), `<ng-checkbox>` (`GtkCheckButton`), `<ng-slider>` (`GtkScale`), and `<ng-select>` (`GtkDropDown`) across `WidgetKind`, `RenderEngine` preview, `Gtk4Sys`, `GtkLinuxBackend`, `WinUiBackend`, and `MacOsAppKitBackend`.
- **forms:** create `@ng-native/forms` (`packages/forms-native`) providing `NativeTextValueAccessor`, `NativeBooleanValueAccessor`, and `NativeNumberValueAccessor` (`ControlValueAccessor` directives) for two-way reactive and template-driven Angular forms over `NativeRenderer2`.
- **events:** support typed `EventPayload` (`None`, `Text`, `Bool`, `Int`) across `EventRegistry`, `JsRuntime`, and `JsEngineBackend` (`__dispatchEvent`).

### Features (Phase 3 — Pluggable JS Engines & `@ng-native/platform`)

- **runtime:** add hexagonal `JsEngineBackend` trait (`src/runtime/js_engine.rs`) with dual adapters: Apple/WebKit **JavaScriptCore (`JscEngine`)** via isolated dynamic FFI (`src/runtime/jsc_sys.rs`, `libjavascriptcoregtk-6.0.so.1` 4-tier JIT) and portable **QuickJS (`QuickJsEngine`)**, selectable via `angular.json` (`"jsEngine": "jsc" | "quickjs"`).
- **platform:** create `@ng-native/platform` (`packages/platform-native`) implementing `NativeRenderer2`, `NativeRendererFactory2`, `NativeElementNode`, `provideNativePlatform()`, and `bootstrapNativeApplication()`.

### Features (Phase 2 — Scene Graph Completeness & Layout Primitives)

- **engine:** add `Anchor` / `Comment` nodes, `insert_before`, `remove_child`, and `set_attribute` / `get_attribute` to `RenderEngine`, `WidgetKind`, and `RenderCommand`.
- **widgets:** expand cross-platform and GTK4 FFI catalog (`Gtk4Sys` & `GtkLinuxBackend`) with `<ng-header-bar>`, `<ng-scroll>`, `<ng-grid>`, and `<ng-separator>`.
- **architecture:** extract scene graph diagnostic preview into `src/engine/preview.rs` and decouple `.ngb` syntax parsing into `src/runtime/parser.rs` (`Instruction` IR).
- **dx:** emit HTML layout attributes automatically from `build-bundle.mjs`, auto-compile missing `.ngb` bundles on `cargo run`, and add unified `pnpm run check` / `pnpm run format:all` scripts.

---

## [0.1.0] - 2026-11-08

### Features

- **dev:** add state-preserving live Hot Reload (`JsRuntime::hot_reload`), non-blocking 60 FPS `mtime` watcher in the native GTK4 loop, `--watch` mode in `build-bundle.mjs`, and `pnpm run dev` (`examples/counter-app/dev-server.mjs`).
- **icons:** support multi-resolution Angular `favicon.ico` (`16x16`, `32x32`, `48x48`) in `WindowIconAsset`, generate Freedesktop `hicolor` PNG pyramid (`16x16`..`256x256`), and wire Wayland/KDE Plasma `app_id` (`g_set_prgname`) + `GtkIconTheme` taskbar integration (`0af3563`).
- **angular:** replace mock setup with an official **Angular 21** CLI workspace (`@angular/core` 21.2, `CUSTOM_ELEMENTS_SCHEMA`, `signal()`) and a declarative template/CSS/event compiler (`build-bundle.mjs`) (`974b95b`).
- **engine:** implement dynamic Linux GTK4 native window driver (`Gtk4Sys` loading `libgtk-4.so.1`, `libgobject-2.0.so.0`, `libgio-2.0.so.0`, `libglib-2.0.so.0`), real `GtkCssProvider` styling, and reactive Signal/event dispatch in `JsRuntime` (`d535518`).
- **core:** initialize `ng-native-engine` with Generational `Arena`, runtime `BorrowState` checker, `CommandQueue`, `StyleRegistry`, and cross-platform `PlatformBackend` adapters (`GtkLinuxBackend`, `WinUiBackend`, `MacOsAppKitBackend`, `HeadlessBackend`) (`3642561`).

### Tooling & CI

- **tooling:** configure `oxfmt`, `oxlint`, `eslint` with `angular-eslint` and `eslint-plugin-oxlint`, and `.vscode/settings.json` format-on-save (`02a915d`).
- **ci:** configure GitHub Actions (`ubuntu-24.04`, Node 24, `pnpm`, `oxfmt`, `oxlint`, `eslint`, `rustfmt`, `clippy`, and `cargo test` with `HeadlessBackend`) (`dc94445`).

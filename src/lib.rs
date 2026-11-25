//! # ng-native-engine
//!
//! Native multi-OS rendering, runtime, desktop services, and installer packager engine for
//! **Angular 22+** applications running directly on Linux (GTK4 + Libadwaita), Windows (WinUI 3),
//! and macOS (AppKit / Cocoa) without an embedded `WebView` or Chromium process.
//!
//! ## Crate Architecture
//!
//! - [`engine`]: OS-agnostic scene graph (`Arena`), command buffer (`CommandQueue`), style
//!   registry (`StyleRegistry`), widget attributes, and event coalescing (`EventMask`).
//! - [`platforms`]: Platform backend drivers (`GtkLinuxBackend`, `WinUiBackend`,
//!   `MacOsAppKitBackend`, `HeadlessBackend`) plus the shared layout solver (`layout_solver`).
//! - [`runtime`]: Embedded JS engines (`JavaScriptCore` JIT and `QuickJS-NG`), `.ngb`
//!   instruction parser, reactive signal/attribute propagator, `HostBridge`, and `NativeServerWorker`.
//! - [`desktop`]: OS-native integrations (`NativeHttpEngine`, `NativeSqliteDatabase`,
//!   `NativeFsManager` watchers, `NativeComputePool`, portals, and system environment).
//! - [`ipc`]: Cross-platform Hot Module Replacement (HMR) over Unix Domain Sockets and loopback TCP.
//! - [`config`] & [`packager`]: Workspace manifest resolution, static permission analyzer, and
//!   native `.deb`, `AppImage`, `Flatpak`, `.msix`, and `.dmg` installer builders.

pub mod config;
pub mod desktop;
pub mod engine;
mod ffi_dl;
pub mod ipc;
pub mod packager;
pub mod platforms;
pub mod runtime;

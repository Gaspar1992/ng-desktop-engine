# `@ng-desktop/builders`

Angular CLI and Nx native desktop builder, AST-driven `.ngb` compiler (`oxc-parser`, `@angular/compiler`, `postcss`), server route bundler, static least-privilege permission analyzer, `ng add @ng-desktop/builders` schematic, `ng-desktop` CLI, and prebuilt `@ng-desktop/engine-*` binary resolution.

## Install

Add cleanly via Angular schematic (recommended):

```bash
ng add @ng-desktop/builders
```

Or install manually as a dev dependency:

```bash
pnpm add -D @ng-desktop/builders
```

Prebuilt native engine binaries (`@ng-desktop/engine-linux-x64-gnu`, `@ng-desktop/engine-linux-arm64-gnu`, `@ng-desktop/engine-win32-x64-msvc`, `@ng-desktop/engine-darwin-arm64`, `@ng-desktop/engine-darwin-x64`) are listed as `optionalDependencies` and resolved automatically (or override with `NG_DESKTOP_ENGINE_BIN`).

## CLI Commands

```bash
npx ng-desktop init --target=linux-gtk4     # Wire native target in angular.json or project.json
npx ng-desktop permissions [--strict]      # Inspect AST-inferred OS capabilities
npx ng-desktop build [--watch]             # Compile Angular + .ngb IR + permissions.manifest.json
npx ng-desktop run [--batch]               # Launch the native desktop window
npx ng-desktop package [--all]             # Stage OS distribution packages
```

## Key Exports

- **Architect / Nx Builders:** `applicationBuilder`, `devServerBuilder`, `createArchitectBuilder`, `executeNativeBuilder`, `createNativeTargetConfig`
- **Compilers & Workspace:** `compileBundle`, `watchBundle`, `compileServerBundle`, `analyzeProjectPermissions`, `resolveWorkspaceProject`, `resolveEngineBinaryPath`, `ENGINE_BINARY_PACKAGES`
- **HMR IPC:** `pushHmrReload`, `pushHmrServerReload`, `pushHmrEval`, `pingHmr`

## Documentation

- [Getting Started with External Angular 22+ & Nx Apps](../../docs/GETTING_STARTED.md)
- [Ecosystem Compatibility Guide](../../docs/ECOSYSTEM_COMPATIBILITY.md)
- [Architectural Roadmap & OS Maturity Matrix](../../ROADMAP.md)

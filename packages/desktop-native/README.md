# `@ng-desktop/desktop`

Angular desktop OS services powered by the Rust host bridge: theme signals, notifications, clipboard, file/confirm dialogs, `(closeRequest)` guards, scroll telemetry, SQLite WAL live queries, XDG filesystem + kernel file watching, network monitoring, drag-and-drop, and multithreaded fuzzy compute.

## Install

```bash
pnpm add @ng-desktop/desktop @ng-desktop/platform
```

Peer dependencies: `@angular/core >= 22.0.0`, `@ng-desktop/platform`.

## Quick Usage

```ts
import { inject, Injectable } from '@angular/core';
import {
  NativeDesktopService,
  NativeFileSystemService,
  NativeSqliteService,
} from '@ng-desktop/desktop';

@Injectable({ providedIn: 'root' })
export class WorkspaceStore {
  private readonly desktop = inject(NativeDesktopService);
  private readonly sqlite = inject(NativeSqliteService);
  private readonly fs = inject(NativeFileSystemService);

  readonly tasks = this.sqlite.liveQuery<{ id: string; title: string }>(
    'SELECT id, title FROM tasks ORDER BY id DESC;',
  );
  readonly config = this.fs.watchJson(this.fs.resolveXdgPath('config', 'settings.json'), {
    dark: true,
  });

  notifySaved(title: string): void {
    this.desktop.notify('Saved', title);
  }
}
```

Add `provideNativeDesktop()` to `appConfig.providers`.

## Key Exports

- **OS Hub & Lifecycle:** `provideNativeDesktop`, `NativeDesktopService` (`theme`, `notify`, `confirmDialog`, `openFileDialog`, `saveFileDialog`, `onScroll`, `onScrollBottom`, `onWindowCloseRequest`, `setPreventClose`, `closeWindow`, `setAppMenu`, `setTray`, `registerShortcut`)
- **SQLite & Storage:** `NativeSqliteService` (`execute`, `query`, `liveQuery`)
- **Filesystem & Watcher:** `NativeFileSystemService` (`resolveXdgPath`, `readText`, `writeText`, `readJson`, `writeJson`, `watchFile`, `watchJson`)
- **Networking, Compute & Drop:** `NativeNetworkService`, `NativeComputeService` (`fuzzyFilter`, `hash`), `NativeDropTargetDirective`

## Documentation

- [Getting Started with External Angular 22+ & Nx Apps](../../docs/GETTING_STARTED.md)
- [Architectural Roadmap & OS Maturity Matrix](../../ROADMAP.md)

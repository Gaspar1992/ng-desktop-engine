# Getting Started with `@ng-desktop/*` in External Angular 22+ & Nx Apps

Build native desktop apps from an existing or new **Angular 22+** application (`Angular CLI` or `Nx`) without bundling Chromium or a WebView. This guide walks you through installing the `@ng-desktop/*` packages, configuring an external workspace, writing templates without `CUSTOM_ELEMENTS_SCHEMA`, wiring routing/forms/virtual lists/OS services, and running native builds.

---

## 1. Platform Support & Prerequisites (`v0.4.0`)

Before adding `@ng-desktop/*` to an external project, check the current runtime maturity per operating system (see [`ROADMAP.md`](../ROADMAP.md) for the `v0.5.0` Windows/macOS runtime parity plan):

| Target OS                 | Runtime Status (`v0.4.0`)                                            | What Works Today                                                                                                    |
| :------------------------ | :------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------ |
| **Linux (`linux-gtk4`)**  | **Full Ivy Runtime** (`GTK4 / Libadwaita` + `JavaScriptCore`)        | Compiled Angular 22+ apps, signals, forms, router, HTTP/2 (`libsoup-3.0`), SQLite, XDG + `inotify`, Unix socket HMR |
| **Windows (`win32-x64`)** | **Live Win32 FFI (`user32.dll`), JS Parity in `v0.5.0`**             | Window/control realization via `Win32Sys` & `AppxManifest` staging; compiled Ivy JS execution arrives in `v0.5.0`   |
| **macOS (`darwin-*`)**    | **Live Objective-C AppKit FFI (`AppKitSys`), JS Parity in `v0.5.0`** | Window/control realization via `AppKitSys` & `.app` bundle staging; compiled Ivy JS execution arrives in `v0.5.0`   |

### Required Tooling

- **Angular:** `>= 22.0.0` (Zoneless change detection)
- **Node.js:** `>= 24` and a Node package manager (`pnpm`, `npm`, or `yarn`)
- **Linux Runtime Shared Libraries (Ubuntu/Debian example):** `libgtk-4-1`, `libadwaita-1-0`, `libjavascriptcoregtk-6.0-1`, `libsoup-3.0-0`, `libsqlite3-0`
- **Rust Toolchain (optional):** Only needed if no prebuilt `@ng-desktop/engine-*` binary matches your platform or when compiling from source (`cargo`).

---

## 2. Install `@ng-desktop/*` in Your Workspace

### Option A: Automated Setup (`ng add`)

For a standard Angular CLI or Nx workspace, run the schematic from the root of your workspace:

```bash
ng add @ng-desktop/builders
```

This adds `@ng-desktop/platform` and `@ng-desktop/builders` to `package.json` and registers a `native` target (`@ng-desktop/builders:application`) in `angular.json` (or `project.json` for Nx). If your workspace already has dependencies installed and only needs target wiring, run:

```bash
npx ng-desktop init --target=linux-gtk4
```

Pass `--project=<app-name>` in multi-project workspaces.

### Option B: Manual Modular Installation

Every `@ng-desktop/*` library is opt-in beyond `@ng-desktop/platform` and `@ng-desktop/builders`. Install only the modules your app uses:

| Package                | Kind              | Purpose                                                                                                                                 |
| :--------------------- | :---------------- | :-------------------------------------------------------------------------------------------------------------------------------------- |
| `@ng-desktop/platform` | `dependencies`    | Core `Renderer2`, `bootstrapNativeApplication`, `NATIVE_UI_DIRECTIVES`, `<ng-virtual-list>`, ecosystem bridges                          |
| `@ng-desktop/forms`    | `dependencies`    | `ControlValueAccessor` directives (`NATIVE_FORM_DIRECTIVES`) for `ReactiveFormsModule` / `FormsModule`                                  |
| `@ng-desktop/router`   | `dependencies`    | `provideNativeRouter`, `NativePlatformLocation`, `withNativeTitleStrategy`, `RouterOutlet`                                              |
| `@ng-desktop/http`     | `dependencies`    | `provideHttpClient` / `provideNativeHttpClient` over OS HTTP/2 + `createNativeHttpResource`                                             |
| `@ng-desktop/desktop`  | `dependencies`    | `NativeDesktopService`, `NativeSqliteService`, `NativeFileSystemService`, `NativeComputeService`, `NativeNetworkService`, drag-and-drop |
| `@ng-desktop/server`   | `dependencies`    | Local Analog/Nitro-style server routes (`defineNativeEventHandler`) running inside the engine                                           |
| `@ng-desktop/builders` | `devDependencies` | Architect/Nx builder, AST permission analyzer, `ng-desktop` CLI, and optional prebuilt `@ng-desktop/engine-*` binaries                  |

```bash
pnpm add @ng-desktop/platform @ng-desktop/forms @ng-desktop/router @ng-desktop/http @ng-desktop/desktop
pnpm add -D @ng-desktop/builders
```

> **Note on engine binaries:** `@ng-desktop/builders` declares `@ng-desktop/engine-linux-x64-gnu`, `@ng-desktop/engine-linux-arm64-gnu`, `@ng-desktop/engine-win32-x64-msvc`, `@ng-desktop/engine-darwin-arm64`, and `@ng-desktop/engine-darwin-x64` as `optionalDependencies`. You can also override the engine binary explicitly with `NG_DESKTOP_ENGINE_BIN=/path/to/ng-native-engine`.

---

## 3. Configure Bootstrap (`app.config.ts` & `main.ts`)

Wire native providers in `src/app/app.config.ts`:

```ts
import { ApplicationConfig, provideZonelessChangeDetection } from '@angular/core';
import { provideNativeDesktop } from '@ng-desktop/desktop';
import { provideHttpClient, withFetch, withXsrfConfiguration } from '@ng-desktop/http';
import { provideNativeEcosystemCompat } from '@ng-desktop/platform';
import {
  provideNativeRouter,
  withComponentInputBinding,
  withNativeTitleStrategy,
} from '@ng-desktop/router';
import { appRoutes } from './app.routes';

export const appConfig: ApplicationConfig = {
  providers: [
    provideZonelessChangeDetection(),
    provideNativeDesktop(),
    provideNativeRouter(appRoutes, withComponentInputBinding()),
    withNativeTitleStrategy('My Desktop App'),
    provideHttpClient(
      withFetch(),
      withXsrfConfiguration({
        cookieName: 'XSRF-TOKEN',
        headerName: 'X-XSRF-TOKEN',
      }),
    ),
    // Optional: enables NativeStorage, NativeAriaAnnouncer, NativeDevToolsHook, NATIVE_DOCUMENT
    provideNativeEcosystemCompat(),
  ],
};
```

Replace the browser bootstrap call in `src/main.ts` with `bootstrapNativeApplication` (which automatically installs `provideNativePlatform()`):

```ts
import { bootstrapNativeApplication } from '@ng-desktop/platform';
import { AppComponent } from './app/app.component';
import { appConfig } from './app/app.config';

bootstrapNativeApplication(AppComponent, appConfig).appRefPromise?.catch((err) =>
  console.error(err),
);
```

---

## 4. Write Native Templates Without `CUSTOM_ELEMENTS_SCHEMA`

Import `NATIVE_UI_DIRECTIVES` from `@ng-desktop/platform` into standalone components. This provides full Angular template type checking for both native tags (`<ng-window>`, `<ng-box>`, `<ng-button>`, etc.) and standard HTML elements (`<div>`, `<span>`, `<p>`, `<button>`, `<input>`, `<img>`, `<table>`, `<dialog>`, `<menu>` via `HTML_TO_NATIVE_TAG`), with **no `CUSTOM_ELEMENTS_SCHEMA` required**.

You can also style containers using standard CSS Flexbox/Grid properties (`display: flex`, `display: grid`, `flex-direction`, `flex-wrap`, `gap`, `min-width`, `min-height`, `border-radius`, `padding`), which `@ng-desktop/builders` and `@ng-desktop/platform` project directly onto native `GtkFlowBox`, `GtkBox`, and `GtkGrid` layouts.

```ts
import { Component, signal } from '@angular/core';
import { NATIVE_UI_DIRECTIVES } from '@ng-desktop/platform';

@Component({
  selector: 'app-home',
  standalone: true,
  imports: [NATIVE_UI_DIRECTIVES],
  styles: `
    .toolbar {
      display: flex;
      flex-direction: row;
      gap: 12px;
      padding: 12px;
      border-radius: 8px;
    }
    .cards {
      display: flex;
      flex-wrap: wrap;
      gap: 12px;
      min-width: 240px;
    }
  `,
  template: `
    <ng-box orientation="vertical" spacing="12px" hexpand="true" vexpand="true">
      <div class="toolbar">
        <h2>Tasks ({{ count() }})</h2>
        <button (click)="increment()">Add Item</button>
      </div>
      <div class="cards">
        <ng-label text="Native GTK4 layout projected from Flexbox CSS" />
      </div>
    </ng-box>
  `,
})
export class HomeComponent {
  readonly count = signal(0);

  increment(): void {
    this.count.update((n) => n + 1);
  }
}
```

---

## 5. Core Feature Recipes

### A. Forms — Signal Forms & Reactive Forms (`@ng-desktop/forms`)

**Angular Signal Forms (`[formField]`):** Import `NATIVE_SIGNAL_FORM_DIRECTIVES` and `NATIVE_UI_DIRECTIVES`. Schema constraints (`required`, `min`, `max`, `maxLength`, `disabled`, `readonly`, `hidden`) sync automatically to native GTK4/OS attributes, ARIA state, and `ng-invalid`/`ng-touched`/`ng-dirty` classes:

```ts
import { Component, signal } from '@angular/core';
import { form, max, min, NATIVE_SIGNAL_FORM_DIRECTIVES, required } from '@ng-desktop/forms';
import { NATIVE_UI_DIRECTIVES } from '@ng-desktop/platform';

@Component({
  selector: 'app-settings',
  standalone: true,
  imports: [NATIVE_UI_DIRECTIVES, NATIVE_SIGNAL_FORM_DIRECTIVES],
  template: `
    <ng-box orientation="vertical" spacing="10px">
      <ng-entry placeholder="Workspace name" [formField]="settingsForm.workspace" />
      <ng-switch [formField]="settingsForm.notifications" />
      <ng-slider [formField]="settingsForm.refreshRate" />
    </ng-box>
  `,
})
export class SettingsComponent {
  readonly model = signal({
    workspace: 'Desktop CRM',
    notifications: true,
    refreshRate: 30,
  });

  readonly settingsForm = form(this.model, (path) => {
    required(path.workspace);
    min(path.refreshRate, 10);
    max(path.refreshRate, 100);
  });
}
```

**Reactive & Template Forms (`[formGroup]`, `[(ngModel)]`):** Import `ReactiveFormsModule` (or `FormsModule`) alongside `NATIVE_FORM_DIRECTIVES` and `NATIVE_UI_DIRECTIVES`:

```ts
import { Component } from '@angular/core';
import { FormControl, FormGroup, ReactiveFormsModule, Validators } from '@angular/forms';
import { NATIVE_FORM_DIRECTIVES } from '@ng-desktop/forms';
import { NATIVE_UI_DIRECTIVES } from '@ng-desktop/platform';

@Component({
  selector: 'app-reactive-settings',
  standalone: true,
  imports: [ReactiveFormsModule, NATIVE_FORM_DIRECTIVES, NATIVE_UI_DIRECTIVES],
  template: `
    <ng-box [formGroup]="form" orientation="vertical" spacing="10px">
      <ng-entry placeholder="Workspace name" formControlName="workspace" />
      <ng-switch formControlName="notifications" />
      <ng-slider min="10" max="100" formControlName="refreshRate" />
    </ng-box>
  `,
})
export class ReactiveSettingsComponent {
  readonly form = new FormGroup({
    workspace: new FormControl('Desktop CRM', {
      nonNullable: true,
      validators: [Validators.required],
    }),
    notifications: new FormControl(true, { nonNullable: true }),
    refreshRate: new FormControl(30, { nonNullable: true }),
  });
}
```

### B. Multi-View Routing (`@ng-desktop/router`)

`<router-outlet />`, `RouterLink`, and route param input binding (`withComponentInputBinding()`) work directly inside native containers:

```html
<!-- app.component.html -->
<ng-window id="win" title="My Desktop App">
  <ng-header-bar title="Workspace" />
  <ng-box orientation="vertical" spacing="12px" hexpand="true" vexpand="true">
    <ng-box orientation="horizontal" spacing="8px">
      <ng-button routerLink="/items">Catalog</ng-button>
      <ng-button routerLink="/settings">Settings</ng-button>
    </ng-box>
    <router-outlet />
  </ng-box>
</ng-window>
```

### C. Virtualized Lists, Popovers, & Dialogs (`@ng-desktop/platform`)

For large datasets, import `NativeVirtualListComponent` so only the visible slice + overscan stays realized in the OS widget tree. Combine with `<ng-popover>` and `<ng-dialog>` for menus and modal confirmations:

```ts
import { Component, signal } from '@angular/core';
import {
  NATIVE_UI_DIRECTIVES,
  NativeVirtualListComponent,
  type NativeVirtualRowActivatedEvent,
} from '@ng-desktop/platform';

interface TaskRow {
  id: number;
  title: string;
}

@Component({
  selector: 'app-task-list',
  standalone: true,
  imports: [NATIVE_UI_DIRECTIVES, NativeVirtualListComponent],
  template: `
    <ng-box orientation="vertical" spacing="8px" hexpand="true" vexpand="true">
      <ng-button id="filter_anchor" (click)="menuOpen.set(!menuOpen())">Options</ng-button>

      <ng-popover
        anchor="filter_anchor"
        [open]="menuOpen()"
        position="bottom"
        [autohide]="true"
        (closed)="menuOpen.set(false)"
      >
        <ng-list selectionMode="single" [showSeparators]="true" (rowActivated)="onMenuPick()">
          <ng-label text="Refresh items" />
          <ng-label text="Clear completed" />
        </ng-list>
      </ng-popover>

      <ng-virtual-list
        [items]="tasks()"
        [itemHeight]="48"
        [viewportHeight]="420"
        (rowActivated)="onTaskActivated($event)"
      >
        <ng-template let-item let-index="index">
          <ng-label [text]="index + 1 + '. ' + item.title" />
        </ng-template>
      </ng-virtual-list>
    </ng-box>
  `,
})
export class TaskListComponent {
  readonly menuOpen = signal(false);
  readonly tasks = signal<TaskRow[]>(
    Array.from({ length: 5000 }, (_, i) => ({ id: i + 1, title: `Task #${i + 1}` })),
  );

  onMenuPick(): void {
    this.menuOpen.set(false);
  }

  onTaskActivated(event: NativeVirtualRowActivatedEvent<TaskRow>): void {
    console.log('Activated row:', event.index, event.item.title);
  }
}
```

### D. Intercepting Window Close (`(closeRequest)` / `(beforeunload)`)

Guard unsaved changes with `[preventClose]` and `(closeRequest)` on `<ng-window>`:

```html
<ng-window
  id="win"
  title="Notes Editor"
  [preventClose]="hasUnsavedChanges()"
  (closeRequest)="onCloseRequest()"
>
  <!-- App content -->
</ng-window>
```

```ts
import { Component, inject, signal } from '@angular/core';
import { NativeDesktopService } from '@ng-desktop/desktop';

@Component({/* ... */})
export class EditorRootComponent {
  private readonly desktop = inject(NativeDesktopService);
  readonly hasUnsavedChanges = signal(true);

  onCloseRequest(): void {
    const confirmed = this.desktop.confirmDialog({
      title: 'Discard unsaved edits?',
      message: 'You have unsaved notes that will be lost.',
      confirmLabel: 'Close anyway',
      cancelLabel: 'Keep editing',
    });
    if (confirmed) {
      this.hasUnsavedChanges.set(false);
      this.desktop.closeWindow('win');
    }
  }
}
```

> **Note:** On `v0.4.0`, interactive `confirmDialog`, `openFileDialog`, `saveFileDialog`, clipboard, and theme signals use `DesktopServices` in-memory/host-bridge state; full native XDG Desktop Portal, Win32 Common Item Dialog, and Cocoa panel integrations land in `v0.5.0`.

### E. OS Storage, SQLite Live Queries, & File Watching (`@ng-desktop/desktop`)

```ts
import { inject, Injectable } from '@angular/core';
import {
  NativeDesktopService,
  NativeFileSystemService,
  NativeSqliteService,
} from '@ng-desktop/desktop';

@Injectable({ providedIn: 'root' })
export class NotesStore {
  private readonly desktop = inject(NativeDesktopService);
  private readonly sqlite = inject(NativeSqliteService);
  private readonly fs = inject(NativeFileSystemService);

  readonly notes = this.sqlite.liveQuery<{ id: string; title: string }>(
    'SELECT id, title FROM notes ORDER BY id DESC;',
  );
  readonly configFile = this.fs.watchJson(this.fs.resolveXdgPath('config', 'settings.json'), {
    compactMode: false,
  });

  initDb(): void {
    this.sqlite.execute('CREATE TABLE IF NOT EXISTS notes (id INTEGER PRIMARY KEY, title TEXT);');
  }

  saveNote(title: string): void {
    const escaped = title.replaceAll("'", "''");
    this.sqlite.execute(`INSERT INTO notes (title) VALUES ('${escaped}');`);
    this.desktop.notify('Note Saved', title);
  }
}
```

### F. Embedded Local Server Routes (`@ng-desktop/server`)

Place file-based handlers under `<sourceRoot>/server/routes/` (for example `src/server/routes/api/notes/[id].put.ts`). `@ng-desktop/builders` compiles them into `<outDir>/server.bundle.js` and runs them inside the Rust host worker, reachable through Angular's `HttpClient`:

```ts
import {
  createError,
  defineNativeEventHandler,
  getRouterParam,
  readBody,
} from '@ng-desktop/server';

export default defineNativeEventHandler((event) => {
  const id = Number(getRouterParam(event, 'id'));
  if (!Number.isInteger(id) || id < 1) {
    throw createError({ statusCode: 400, message: 'Invalid note ID' });
  }

  const body = readBody<{ title?: string }>(event);
  event.context.db.execute('INSERT OR REPLACE INTO notes (id, title) VALUES (?, ?);', [
    id,
    String(body.title ?? ''),
  ]);
  return { saved: true, id };
});
```

---

## 6. Permissions, Dev Loop, & Packaging (`ng-desktop` CLI)

`@ng-desktop/builders` ships the `ng-desktop` binary for Angular CLI (`angular.json`) and Nx (`project.json`) workspaces:

| Step                 | Command                               | Outcome                                                                                                      |
| :------------------- | :------------------------------------ | :----------------------------------------------------------------------------------------------------------- |
| Inspect capabilities | `npx ng-desktop permissions`          | Runs AST static analysis via `oxc-parser` and prints inferred capabilities (`--strict` enforces strict mode) |
| Build `.ngb` bundle  | `npx ng-desktop build`                | Compiles Angular + native `.ngb` IR, `.server.bundle.js` (if present), and `permissions.manifest.json`       |
| Watch & HMR          | `npx ng-desktop build --watch`        | Recompiles on save and pushes live updates over the Unix domain socket (`hmr.sock`)                          |
| Launch native window | `npx ng-desktop run`                  | Resolves the native binary and opens your app window (pass `--batch` for headless CI runs)                   |
| Stage OS packages    | `npx ng-desktop package` (or `--all`) | Stages `.deb`/`AppImage`/`Flatpak` (or `--all` for Windows `AppxManifest` & macOS `.app`)                    |

Customize least-privilege permissions inside your `native` target options (`angular.json` or Nx `project.json`):

```json
{
  "builder": "@ng-desktop/builders:application",
  "options": {
    "targetPlatform": "linux-gtk4",
    "permissions": {
      "mode": "auto",
      "deny": ["full-system-access"]
    }
  }
}
```

Supported capability tokens: `network`, `sqlite`, `filesystem:xdg`, `filesystem:dialogs`, `filesystem:drop`, `clipboard`, `notifications`, `compute-threads`, and `full-system-access`.

---

## 7. Quick Troubleshooting

| Symptom                                                             | Likely Cause / Fix                                                                                                                         |
| :------------------------------------------------------------------ | :----------------------------------------------------------------------------------------------------------------------------------------- |
| `Native engine not found for <target>`                              | No matching `@ng-desktop/engine-*` package or `cargo` binary found. Install platform optional dependencies or set `NG_DESKTOP_ENGINE_BIN`. |
| Missing `libgtk-4.so.1` or `libjavascriptcoregtk-6.0.so.1` on Linux | Install runtime packages (`libgtk-4-1`, `libadwaita-1-0`, `libjavascriptcoregtk-6.0-1`, `libsoup-3.0-0`, `libsqlite3-0`).                  |
| Permission denied at runtime (`mode: 'strict'`)                     | Run `npx ng-desktop permissions` to inspect inferred capabilities and add required tokens to `permissions.allow`.                          |
| Windows or macOS fails to execute Angular JS bundles                | Expected in `v0.4.0`: live Win32/AppKit UI FFI exists, while full multi-OS ES2024 JS engine parity is scheduled for `v0.5.0`.              |

### Further Reading

- [**Ecosystem Compatibility Guide**](./ECOSYSTEM_COMPATIBILITY.md) — Integrating NgRx, Angular Aria, Angular CDK, and Nx (plus why Material MDC templates are incompatible).
- [**Architectural Roadmap**](./../ROADMAP.md) — Current `v0.4.0` maturity matrix and `v0.5.0` / `v0.6.0` milestones.

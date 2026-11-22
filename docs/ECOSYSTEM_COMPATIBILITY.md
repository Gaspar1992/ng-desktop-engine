# Ecosystem Compatibility Guide (`NgRx`, `Angular Aria`, `CDK`, `Angular Material`, `Nx`)

`ng-desktop-engine` renders Angular 22+ applications directly into native OS UI widgets (`GTK4` / `Libadwaita`, `WinUI 3`, `AppKit`) over a Rust runtime (`RenderEngine`) without embedding Chromium or a browser DOM.

Understanding which libraries run out of the box, which need lightweight host bridges, and which rely on browser CSS layout prevents surprises when integrating third-party Angular packages.

---

## 1. Compatibility Matrix

| Ecosystem Library                                  | Status                                   | Integration Surface                                                   | Notes                                                                                                                                                                                                                               |
| -------------------------------------------------- | ---------------------------------------- | --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **`@ngrx/signals`** (`signalStore`, `rxMethod`)    | **Compatible**                           | Built-in Angular Signals + `provideZonelessChangeDetection()`         | Zero DOM dependency; works natively out of the box.                                                                                                                                                                                 |
| **`@ngrx/store` & `@ngrx/effects`**                | **Compatible**                           | Core RxJS + Angular DI                                                | Use `NativeStorage` (`NATIVE_STORAGE`) for persistence meta-reducers instead of browser `window.localStorage`.                                                                                                                      |
| **`@ngrx/store-devtools`**                         | **Compatible (via Bridge)**              | `NativeDevToolsHook` / `provideNativeEcosystemCompat()`               | Exposes `globalThis.__REDUX_DEVTOOLS_EXTENSION__` safely in JavaScriptCore / QuickJS.                                                                                                                                               |
| **`@angular/aria`** (headless ARIA widgets)        | **Compatible**                           | `NativeElementNode` WAI-ARIA + focus APIs                             | Headless state/keyboard patterns map onto `<ng-*>` native primitives (`role`, `tabIndex`, `focus()`, `aria-*`, keyboard events).                                                                                                    |
| **`@angular/cdk`** (`a11y`, `collections`, `bidi`) | **Partial / Compatible (Headless)**      | `NativeAriaAnnouncer`, `createNativeDocument()`, `NATIVE_DOCUMENT`    | FocusKeyManager, ListKeyManager, SelectionModel, and LiveAnnouncer patterns work with `NativeElementNode`. Browser-only CSS positioning (`Overlay` DOM portals / `CdkDragDrop` HTML events) require native window/drop equivalents. |
| **`@angular/material`**                            | **Not Directly Compatible (Design Gap)** | See analysis below                                                    | Relies on HTML `<div>`/`<span>`, CSS Flexbox/Grid, MDC CSS tokens, inline SVG ripples, and browser DOM geometry.                                                                                                                    |
| **Nx (`nx.json` / `project.json`)**                | **Supported**                            | `@ng-desktop/builders` (`resolveWorkspaceProject`) & `ng-desktop` CLI | Supported alongside Angular CLI (`angular.json`).                                                                                                                                                                                   |

---

## 2. Enabling Ecosystem Compatibility Providers & Standard HTML DX

Import `provideNativeEcosystemCompat()`, `provideNativePlatform()`, and `NATIVE_UI_DIRECTIVES` from `@ng-desktop/platform` in your application:

```ts
import { ApplicationConfig } from '@angular/core';
import { provideNativeEcosystemCompat, provideNativePlatform } from '@ng-desktop/platform';

export const appConfig: ApplicationConfig = {
  providers: [
    provideNativePlatform(),
    provideNativeEcosystemCompat(), // installs NativeStorage, NativeAriaAnnouncer, NativeDevToolsHook, NATIVE_DOCUMENT
  ],
};
```

Components can import `NATIVE_UI_DIRECTIVES` (and `NativeVirtualListComponent` when virtualizing large lists) to use strongly typed `<ng-*>` bindings, standard HTML elements (`<div>`, `<span>`, `<button>`, `<input>`, `<table>`, `<dialog>`, `<menu>` via `HTML_TO_NATIVE_TAG`), CSS Flexbox/Grid layouts, and desktop events (`(scrollBottom)`, `(closeRequest)`, `(beforeunload)`, `(rowActivated)`) without `CUSTOM_ELEMENTS_SCHEMA`.

### Exposed Tokens & Services (`@ng-desktop/platform`)

- **`NativeStorage` / `NATIVE_STORAGE`**: Implements Web `Storage` (`getItem`, `setItem`, `removeItem`, `clear`, `key`, `length`) backed by native SQLite (`ng_native_storage`) + in-memory cache.
- **`NativeAriaAnnouncer`**: Mirrors `@angular/cdk/a11y` `LiveAnnouncer` (`announce(message, politeness)`), queuing accessibility status events into the native bridge.
- **`NativeDevToolsHook` / `NATIVE_DEVTOOLS_HOOK`**: Implements the `connect()`, `init()`, `send()`, and `subscribe()` DevTools extension contract for `@ngrx/store-devtools`.
- **`NativeElementNode`**: Provides synchronous `tagName`, `nodeName`, `parentNode`, `firstChild`, `lastChild`, `previousSibling`, `nextSibling`, `childNodes`, `tabIndex`, `role`, `classList`, `getAttribute`/`setAttribute`/`removeAttribute`/`hasAttribute`, `focus()`, `blur()`, `contains()`, `addEventListener`/`removeEventListener`/`dispatchEvent`, and `getBoundingClientRect()`.
- **`NATIVE_DOCUMENT` / `createNativeDocument()`**: Headless `Document` facade backed by `NativeElementNode` so `DOCUMENT`-injecting headless utilities can query elements and track `activeElement`.

---

## 3. Why Angular Material Cannot Render Unchanged (Gap Analysis)

`@angular/material` components (e.g., `MatButton`, `MatTable`, `MatDialog`, `MatSelect`) ship HTML templates made of nested `<span class="mdc-button__ripple">`, `<svg>` icons, CSS custom properties (`--mdc-*`), and `document.createElement` DOM overlay containers evaluated by a Blink/WebKit CSS box engine.

Because `ng-desktop-engine` intentionally omits WebView/Chromium to stay lightweight and use genuine OS controls (`GtkButton`, `WinUI Button`, `NSButton`), raw MDC HTML/CSS has no browser CSS layout engine.

### Recommended Architecture for Complex UI Libraries

1. **Logic & State:** Use `@ngrx/signals`, `@ngrx/store`, and `@ngrx/effects` directly.
2. **Headless Accessibility & Interaction:** Pair **`@angular/aria`** and **`@angular/cdk/collections` / `@angular/cdk/a11y`** controllers with native `<ng-button>`, `<ng-entry>`, `<ng-list>` / `<ng-virtual-list>`, `<ng-popover>`, `<ng-select>`, `<ng-expander>`, `<ng-stack>`, `<ng-dialog>`, and `<ng-window>` elements.
3. **Desktop Styling:** Style native controls with CSS rule subsets supported by the engine's style compiler and `parseRuntimeStyles` (`display: flex` / `grid`, `flex-direction`, `flex-wrap`, `gap`, `min-width`, `min-height`, `border-radius`, `background-color`, `color`, `padding`, `spacing`, font sizing, Libadwaita/Fluent/Cocoa system themes).

---

## 4. Using Nx Monorepos (`project.json`)

Both the TypeScript builder (`packages/builders-native`) and the Rust host runtime inspect workspaces in priority order without hardcoded repository literals:

1. Explicit `--config=<path>` / `NG_DESKTOP_CONFIG` (`angular.json`, `project.json`, or `ng-desktop.runtime.json`)
2. Angular CLI `angular.json` (`projects.<name>.architect.native`)
3. Nx `project.json` or `nx.json` (`apps/*/project.json`, `targets.native.options`)
4. Runtime manifest `ng-desktop.runtime.json` (bundled automatically inside `.deb`, `.exe`, and `.dmg` artifacts)

Example Nx `apps/desktop-crm/project.json` target:

```json
{
  "name": "desktop-crm",
  "projectType": "application",
  "sourceRoot": "apps/desktop-crm/src",
  "targets": {
    "native": {
      "executor": "@ng-desktop/builders:application",
      "options": {
        "targetPlatform": "linux-gtk4",
        "outputPath": "dist/apps/desktop-crm",
        "permissions": {
          "mode": "auto",
          "deny": ["full-system-access"]
        }
      }
    }
  }
}
```

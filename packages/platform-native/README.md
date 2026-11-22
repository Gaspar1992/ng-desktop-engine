# `@ng-desktop/platform`

Custom Zoneless Angular `Renderer2` platform, typed native widget directives (`NATIVE_UI_DIRECTIVES`), CSS Flexbox/Grid projector, virtualized list (`<ng-virtual-list>`), and headless ecosystem compatibility bridges (`NgRx`, `Angular Aria`, `CDK`).

## Install

```bash
pnpm add @ng-desktop/platform
```

Peer dependency: `@angular/core >= 22.0.0`.

## Quick Usage

```ts
import { Component, signal } from '@angular/core';
import { bootstrapNativeApplication, NATIVE_UI_DIRECTIVES } from '@ng-desktop/platform';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [NATIVE_UI_DIRECTIVES],
  template: `
    <ng-window id="win" title="Desktop App">
      <ng-box orientation="vertical" spacing="12px">
        <ng-label [text]="'Count: ' + count()" />
        <ng-button (click)="count.update((n) => n + 1)">Increment</ng-button>
      </ng-box>
    </ng-window>
  `,
})
export class AppComponent {
  readonly count = signal(0);
}

bootstrapNativeApplication(AppComponent);
```

## Key Exports

- **Bootstrap & Platform:** `bootstrapNativeApplication`, `provideNativePlatform`, `NativeRendererFactory2`, `NativeRenderer2`
- **No-Schema Directives & Virtual List:** `NATIVE_UI_DIRECTIVES`, `HTML_TO_NATIVE_TAG`, `NativeVirtualListComponent`, `computeVirtualSliceRange`
- **Styles & System:** `parseRuntimeStyles`, `formatParsedStyleExtensions`, `provideNativeImageLoader`, `provideNativeTitle`, `provideSystemLocale`
- **Ecosystem Compat:** `provideNativeEcosystemCompat`, `NativeStorage`, `NativeAriaAnnouncer`, `NativeDevToolsHook`, `createNativeDocument`

## Documentation

- [Getting Started with External Angular 22+ & Nx Apps](../../docs/GETTING_STARTED.md)
- [Ecosystem Compatibility Guide](../../docs/ECOSYSTEM_COMPATIBILITY.md)
- [Architectural Roadmap & OS Maturity Matrix](../../ROADMAP.md)

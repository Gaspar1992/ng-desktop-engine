# `@ng-desktop/router`

Native in-memory `PlatformLocation` (`NativePlatformLocation`), route reuse strategy, and window title synchronization enabling `@angular/router` (`<router-outlet />`, `routerLink`, `withComponentInputBinding`, guards, and lazy routes) without a browser URL bar.

## Install

```bash
pnpm add @ng-desktop/router @ng-desktop/platform
```

Peer dependencies: `@angular/core >= 22.0.0`, `@angular/router >= 22.0.0`, `@ng-desktop/platform`.

## Quick Usage

```ts
import { ApplicationConfig } from '@angular/core';
import {
  provideNativeRouter,
  withComponentInputBinding,
  withNativeTitleStrategy,
} from '@ng-desktop/router';
import { appRoutes } from './app.routes';

export const appConfig: ApplicationConfig = {
  providers: [
    provideNativeRouter(appRoutes, withComponentInputBinding()),
    withNativeTitleStrategy('My Desktop App'),
  ],
};
```

Inside your root template:

```html
<ng-window id="win" title="My Desktop App">
  <ng-box orientation="vertical" spacing="12px">
    <ng-button routerLink="/items">Catalog</ng-button>
    <router-outlet />
  </ng-box>
</ng-window>
```

## Key Exports

- `provideNativeRouter`, `NativePlatformLocation`
- `withNativeTitleStrategy`, `NativeTitleStrategy`, `NATIVE_TITLE_PREFIX`
- `withNativeRouteReuse`, `NativeStackRouteReuseStrategy`
- Re-exported Angular router primitives: `Router`, `RouterOutlet`, `RouterLink`, `RouterLinkActive`, `ActivatedRoute`, `withComponentInputBinding`

## Documentation

- [Getting Started with External Angular 22+ & Nx Apps](../../docs/GETTING_STARTED.md)
- [Architectural Roadmap & OS Maturity Matrix](../../ROADMAP.md)

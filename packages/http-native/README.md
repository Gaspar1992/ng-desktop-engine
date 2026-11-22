# `@ng-desktop/http`

OS-native HTTP/2 backend (`NativeHttpBackend`) and reactive `createNativeHttpResource()` implementing `@angular/common/http` (`provideHttpClient`, interceptors, caching, parallel batching, and XSRF configuration) over Rust native transport (`libsoup-3.0` on Linux).

## Install

```bash
pnpm add @ng-desktop/http @ng-desktop/platform
```

Peer dependencies: `@angular/core >= 22.0.0`, `@angular/common >= 22.0.0`, `@ng-desktop/platform`.

## Quick Usage

```ts
import { ApplicationConfig } from '@angular/core';
import { provideHttpClient, withFetch, withXsrfConfiguration } from '@ng-desktop/http';

export const appConfig: ApplicationConfig = {
  providers: [
    provideHttpClient(
      withFetch(),
      withXsrfConfiguration({
        cookieName: 'XSRF-TOKEN',
        headerName: 'X-XSRF-TOKEN',
      }),
    ),
  ],
};
```

## Key Exports

- `provideHttpClient`, `provideNativeHttpClient`, `NativeHttpBackend`
- `withFetch`, `withInterceptors`, `withInterceptorsFromDi`, `withHttpCaching`, `withParallelBatching`, `withXsrfConfiguration`, `withNoXsrfProtection`, `withNativeHttpConfig`
- `createNativeHttpResource`, `NATIVE_HTTP_CONFIG`, `resolveNativeHttpConfig`

## Documentation

- [Getting Started with External Angular 22+ & Nx Apps](../../docs/GETTING_STARTED.md)
- [Architectural Roadmap & OS Maturity Matrix](../../ROADMAP.md)

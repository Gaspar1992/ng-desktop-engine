# `@ng-desktop/server`

Embedded Analog/Nitro-style local server router and event handlers running in-process inside `ng-desktop-engine` (`server_worker`), with direct access to host SQLite, XDG filesystem, multithreaded compute, and OS HTTP/2 bridges.

## Install

```bash
pnpm add @ng-desktop/server
```

Peer dependencies: `@angular/core >= 22.0.0`, `@ng-desktop/platform`.

## Quick Usage

Create file-based routes under `src/server/routes/` (for example `src/server/routes/api/notes/[id].put.ts`):

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

  const body = readBody<{ note?: string }>(event);
  event.context.db.execute('INSERT OR REPLACE INTO notes (id, note) VALUES (?, ?);', [
    id,
    String(body.note ?? ''),
  ]);
  return { saved: true, id };
});
```

## Key Exports

- **Handlers & Request Helpers:** `defineNativeEventHandler`, `createError`, `readBody`, `getQuery`, `getRouterParam`, `setResponseStatus`, `setResponseHeader`, `NativeServerHttpError`
- **Router & Host Bridge:** `createNativeServerRouter`, `registerNativeServerApp`, `createHostServerContext`

## Documentation

- [Getting Started with External Angular 22+ & Nx Apps](../../docs/GETTING_STARTED.md)
- [Ecosystem Compatibility Guide](../../docs/ECOSYSTEM_COMPATIBILITY.md)
- [Architectural Roadmap & OS Maturity Matrix](../../ROADMAP.md)

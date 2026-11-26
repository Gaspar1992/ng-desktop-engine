import {
  createError,
  defaultStatusText,
  NativeEventHandler,
  NativeServerComputeMatch,
  NativeServerContext,
  NativeServerEvent,
  NativeServerHttpError,
  NativeServerHttpRequestOptions,
  NativeServerHttpResponse,
  NativeServerMethod,
  NativeServerRequest,
  NativeServerResponse,
  NativeServerSqlParam,
  NativeServerXdgKind,
} from './event';

/** Route entry pairing an HTTP method, URL pattern (static, `:param`, or `*slug`), and handler. */
export interface NativeServerRouteDefinition {
  method: NativeServerMethod;
  pattern: string;
  handler: NativeEventHandler;
}

/** In-process router dispatching local `HttpClient` requests inside the Rust server worker. */
export interface NativeServerRouter {
  handle(request: NativeServerRequest): Promise<NativeServerResponse>;
  getRoutes(): readonly NativeServerRouteDefinition[];
}

/** Low-level host bridge contract exposed to `server.bundle.js` via `globalThis.__NG_NATIVE_SERVER_HOST__`. */
export interface NativeServerHostBridge {
  sqliteOpen?(path: string): string;
  sqliteExecute(sql: string, params?: readonly NativeServerSqlParam[]): boolean;
  sqliteQuery(
    sql: string,
    params?: readonly NativeServerSqlParam[],
  ): Array<Record<string, unknown>>;
  fsResolvePath(kind: NativeServerXdgKind, relPath: string): string;
  fsReadText(path: string): string | null;
  fsWriteText(path: string, content: string): boolean;
  fsWatchFile(path: string): boolean;
  fsPollChangedFiles(): string[];
  computeFuzzyFilter(
    items: readonly string[],
    query: string,
    limit?: number,
  ): NativeServerComputeMatch[];
  computeHash(input: string): string;
  httpRequest(
    method: string,
    url: string,
    body?: string,
    options?: NativeServerHttpRequestOptions,
  ): Promise<NativeServerHttpResponse> | NativeServerHttpResponse;
}

declare global {
  var __NG_NATIVE_SERVER_HOST__: NativeServerHostBridge | undefined;
  var __NG_NATIVE_SERVER_ROUTER__: NativeServerRouter | undefined;
}

function safeDecodeUriComponent(raw: string): string {
  try {
    return decodeURIComponent(raw);
  } catch {
    throw createError({
      statusCode: 400,
      statusMessage: 'Bad Request',
      message: `Malformed URI component '${raw}'`,
    });
  }
}

function parseUrlParts(rawUrl: string): { path: string; query: Record<string, string> } {
  const withoutOrigin = rawUrl
    .trim()
    .replace(/^(?:https?:\/\/(?:localhost|127\.0\.0\.1)(?::\d+)?|local:\/?)/i, '');
  const hashIdx = withoutOrigin.indexOf('#');
  const cleanUrl = hashIdx === -1 ? withoutOrigin : withoutOrigin.slice(0, hashIdx);
  const qIdx = cleanUrl.indexOf('?');
  const rawPath = qIdx === -1 ? cleanUrl : cleanUrl.slice(0, qIdx);
  const rawQuery = qIdx === -1 ? '' : cleanUrl.slice(qIdx + 1);

  let normalizedPath = rawPath.startsWith('/') ? rawPath : `/${rawPath}`;
  if (normalizedPath.length > 1) {
    normalizedPath = normalizedPath.replace(/\/+$/, '');
  }

  const query: Record<string, string> = {};
  if (rawQuery.length > 0) {
    for (const pair of rawQuery.split('&')) {
      if (!pair) continue;
      const eq = pair.indexOf('=');
      const rawKey = eq === -1 ? pair : pair.slice(0, eq);
      const rawVal = eq === -1 ? '' : pair.slice(eq + 1);
      const key = safeDecodeUriComponent(rawKey.replaceAll('+', ' '));
      const val = safeDecodeUriComponent(rawVal.replaceAll('+', ' '));
      query[key] = val;
    }
  }
  return { path: normalizedPath, query };
}

function splitSegments(path: string): string[] {
  return path.split('/').filter((seg) => seg.length > 0);
}

function matchPattern(
  pattern: string,
  path: string,
): { params: Record<string, string>; weights: number[] } | null {
  const cleanPattern = pattern.startsWith('/') ? pattern : `/${pattern}`;
  const patSegs = splitSegments(cleanPattern);
  const pathSegs = splitSegments(path);
  const params: Record<string, string> = {};
  const weights: number[] = [];

  for (let i = 0; i < patSegs.length; i++) {
    const seg = patSegs[i];
    if (seg.startsWith('*')) {
      const paramName = seg.slice(1).trim() || 'slug';
      params[paramName] = pathSegs.slice(i).map(safeDecodeUriComponent).join('/');
      weights.push(1);
      return { params, weights };
    }
    if (i >= pathSegs.length) {
      return null;
    }
    if (seg.startsWith(':')) {
      const paramName = seg.slice(1).trim();
      params[paramName] = safeDecodeUriComponent(pathSegs[i]);
      weights.push(2);
      continue;
    }
    if (seg !== pathSegs[i]) {
      return null;
    }
    weights.push(3);
  }

  if (patSegs.length !== pathSegs.length) {
    return null;
  }
  return { params, weights };
}

function compareMatchPrecedence(
  a: { weights: number[]; isExactMethod: boolean },
  b: { weights: number[]; isExactMethod: boolean },
): number {
  const len = Math.max(a.weights.length, b.weights.length);
  for (let i = 0; i < len; i++) {
    const wa = a.weights[i] ?? 0;
    const wb = b.weights[i] ?? 0;
    if (wa !== wb) {
      return wb - wa;
    }
  }
  if (a.isExactMethod !== b.isExactMethod) {
    return a.isExactMethod ? -1 : 1;
  }
  return 0;
}

function createFallbackHostBridge(): NativeServerHostBridge {
  const files = new Map<string, string>();
  const watched = new Set<string>();
  const changed = new Set<string>();
  const fallbackCompute = (items: readonly string[], query: string, limit = 50) => {
    const needle = query.trim().toLowerCase();
    const out: NativeServerComputeMatch[] = [];
    for (let index = 0; index < items.length; index++) {
      const value = items[index];
      const lower = value.toLowerCase();
      let score: number | null = null;
      if (!needle) score = 1;
      else if (lower === needle) score = 1000;
      else if (lower.startsWith(needle)) score = 600 + Math.max(0, 80 - value.length);
      else if (lower.includes(needle)) score = 350 + Math.max(0, 60 - lower.indexOf(needle) * 2);
      if (score !== null) out.push({ index, score, value });
    }
    out.sort((a, b) => (b.score !== a.score ? b.score - a.score : a.index - b.index));
    return out.slice(0, limit);
  };

  return {
    sqliteOpen: (path) => path || ':memory:',
    sqliteExecute: () => true,
    sqliteQuery: () => [],
    fsResolvePath: (kind, relPath) =>
      `/tmp/ng-desktop-server/${kind}/${relPath.replace(/^[/\\]+/, '')}`,
    fsReadText: (path) => files.get(path) ?? null,
    fsWriteText: (path, content) => {
      files.set(path, content);
      if (watched.has(path)) changed.add(path);
      return true;
    },
    fsWatchFile: (path) => {
      watched.add(path);
      return true;
    },
    fsPollChangedFiles: () => {
      const out = [...changed];
      changed.clear();
      return out;
    },
    computeFuzzyFilter: fallbackCompute,
    computeHash: (input) => {
      let hash = 2166136261;
      for (let i = 0; i < input.length; i++) {
        hash = Math.imul(hash ^ input.charCodeAt(i), 16777619) >>> 0;
      }
      return hash.toString(16).padStart(8, '0');
    },
    httpRequest: () => ({
      status: 501,
      statusText: 'Not Implemented',
      body: '{"error":"No native HTTP bridge configured"}',
      headers: {},
    }),
  };
}

export function createHostServerContext(): Omit<NativeServerContext, 'params' | 'query'> {
  let fallbackBridge: NativeServerHostBridge | undefined;
  const getHost = (): NativeServerHostBridge => {
    if (globalThis.__NG_NATIVE_SERVER_HOST__) {
      return globalThis.__NG_NATIVE_SERVER_HOST__;
    }
    fallbackBridge ??= createFallbackHostBridge();
    return fallbackBridge;
  };

  const serializeHttpBody = (body: unknown): string | undefined => {
    if (body === undefined || body === null) return undefined;
    return typeof body === 'string' ? body : JSON.stringify(body);
  };

  return {
    db: {
      open: (path) => {
        const host = getHost();
        return host.sqliteOpen ? host.sqliteOpen(path) : path;
      },
      execute: (sql, params) => getHost().sqliteExecute(sql, params),
      query: <T extends Record<string, unknown> = Record<string, unknown>>(
        sql: string,
        params?: readonly NativeServerSqlParam[],
      ) => getHost().sqliteQuery(sql, params) as T[],
    },
    fs: {
      resolvePath: (kind, relPath) => getHost().fsResolvePath(kind, relPath),
      readText: (path) => getHost().fsReadText(path),
      writeText: (path, content) => getHost().fsWriteText(path, content),
      watchFile: (path) => getHost().fsWatchFile(path),
      pollChangedFiles: () => getHost().fsPollChangedFiles(),
    },
    compute: {
      fuzzyFilter: (items, query, limit) => getHost().computeFuzzyFilter(items, query, limit),
      hash: (input) => getHost().computeHash(input),
    },
    http: {
      request: async (method, url, body, options) => {
        const res = await getHost().httpRequest(method, url, serializeHttpBody(body), options);
        return {
          status: res.status,
          statusText: res.statusText,
          body: res.body,
          headers: res.headers ?? {},
          ...(res.cached !== undefined ? { cached: res.cached } : {}),
        };
      },
      get: (url, options) =>
        Promise.resolve(getHost().httpRequest('GET', url, undefined, options)).then((res) => ({
          status: res.status,
          statusText: res.statusText,
          body: res.body,
          headers: res.headers ?? {},
          ...(res.cached !== undefined ? { cached: res.cached } : {}),
        })),
      post: (url, body, options) =>
        Promise.resolve(getHost().httpRequest('POST', url, serializeHttpBody(body), options)).then(
          (res) => ({
            status: res.status,
            statusText: res.statusText,
            body: res.body,
            headers: res.headers ?? {},
            ...(res.cached !== undefined ? { cached: res.cached } : {}),
          }),
        ),
    },
  };
}

function normalizeHeaders(headers?: Record<string, string>): Record<string, string> {
  const normalized: Record<string, string> = {};
  if (!headers) return normalized;
  for (const [k, v] of Object.entries(headers)) {
    normalized[k.toLowerCase()] = String(v);
  }
  return normalized;
}

function serializeResult(result: unknown, event: NativeServerEvent): NativeServerResponse {
  const status = event.response.status;
  const statusText = event.response.statusText ?? defaultStatusText(status);
  const headers = { ...event.response.headers };

  if (typeof result === 'string') {
    headers['content-type'] ??= 'text/plain; charset=utf-8';
    return { status, statusText, headers, body: result };
  }
  if (result === undefined) {
    headers['content-type'] ??= 'application/json; charset=utf-8';
    return { status, statusText, headers, body: '' };
  }
  headers['content-type'] ??= 'application/json; charset=utf-8';
  return { status, statusText, headers, body: JSON.stringify(result) };
}

/** Creates an in-process router sorting routes by specificity (`static > :param > *wildcard`). */
export function createNativeServerRouter(
  routes: readonly NativeServerRouteDefinition[],
  contextFactory: () => Omit<NativeServerContext, 'params' | 'query'> = createHostServerContext,
): NativeServerRouter {
  const routeList = [...routes];

  return {
    getRoutes: () => routeList,

    async handle(request: NativeServerRequest): Promise<NativeServerResponse> {
      const method = String(request.method || 'GET')
        .trim()
        .toUpperCase();
      let responseHeaders: Record<string, string> = {};

      try {
        const { path, query } = parseUrlParts(request.url);

        const pathMatches: Array<{
          route: NativeServerRouteDefinition;
          params: Record<string, string>;
          weights: number[];
          isExactMethod: boolean;
        }> = [];

        for (const route of routeList) {
          const matched = matchPattern(route.pattern, path);
          if (!matched) continue;
          pathMatches.push({
            route,
            params: matched.params,
            weights: matched.weights,
            isExactMethod: route.method.toUpperCase() === method,
          });
        }

        if (pathMatches.length === 0) {
          return {
            status: 404,
            statusText: 'Not Found',
            headers: { 'content-type': 'application/json; charset=utf-8' },
            body: JSON.stringify({ error: `Route not found: ${path}`, statusCode: 404 }),
          };
        }

        const methodMatches = pathMatches.filter((m) => m.isExactMethod || m.route.method === '*');
        if (methodMatches.length === 0) {
          return {
            status: 405,
            statusText: 'Method Not Allowed',
            headers: { 'content-type': 'application/json; charset=utf-8' },
            body: JSON.stringify({
              error: `Method ${method} not allowed for ${path}`,
              statusCode: 405,
            }),
          };
        }

        methodMatches.sort(compareMatchPrecedence);
        const selected = methodMatches[0];
        const baseCtx = contextFactory();
        const event: NativeServerEvent = {
          method,
          url: request.url,
          path,
          headers: normalizeHeaders(request.headers),
          body: request.body ?? null,
          context: {
            ...baseCtx,
            params: selected.params,
            query,
          },
          response: {
            status: 200,
            headers: {},
          },
        };
        responseHeaders = event.response.headers;

        const result = await selected.route.handler(event);
        return serializeResult(result, event);
      } catch (err) {
        if (err instanceof NativeServerHttpError) {
          return {
            status: err.statusCode,
            statusText: err.statusMessage,
            headers: {
              ...responseHeaders,
              'content-type': 'application/json; charset=utf-8',
            },
            body: JSON.stringify({
              error: err.message,
              statusCode: err.statusCode,
              ...(err.data !== undefined ? { data: err.data } : {}),
            }),
          };
        }
        const message = err instanceof Error ? err.message : String(err);
        return {
          status: 500,
          statusText: 'Internal Server Error',
          headers: { 'content-type': 'application/json; charset=utf-8' },
          body: JSON.stringify({ error: message, statusCode: 500 }),
        };
      }
    },
  };
}

/** Registers the compiled server router globally on `__NG_NATIVE_SERVER_ROUTER__` for Rust IPC dispatch. */
export function registerNativeServerApp(
  routes: readonly NativeServerRouteDefinition[],
): NativeServerRouter {
  const router = createNativeServerRouter(routes, createHostServerContext);
  globalThis.__NG_NATIVE_SERVER_ROUTER__ = router;
  return router;
}

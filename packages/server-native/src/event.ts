/** HTTP methods matched by [`createNativeServerRouter`], including `' * '` wildcard routes. */
export type NativeServerMethod =
  | 'GET'
  | 'POST'
  | 'PUT'
  | 'PATCH'
  | 'DELETE'
  | 'HEAD'
  | 'OPTIONS'
  | '*';

/** Logical XDG directory root exposed to local server route handlers. */
export type NativeServerXdgKind = 'config' | 'data' | 'cache';

/** Match entry returned by [`NativeServerComputeContext.fuzzyFilter`]. */
export interface NativeServerComputeMatch {
  index: number;
  score: number;
  value: string;
}

/** Optional headers and call classification for outbound server-side HTTP requests. */
export interface NativeServerHttpRequestOptions {
  headers?: Record<string, string>;
  callKind?: string;
}

/** Response returned from `event.context.http` inside a native server handler. */
export interface NativeServerHttpResponse {
  status: number;
  statusText: string;
  body: string;
  headers: Record<string, string>;
  cached?: boolean;
}

/** Positional bind parameter accepted by `event.context.db.execute` and `query`. */
export type NativeServerSqlParam = string | number | boolean | null;

/** Host-backed SQLite context available to native server route handlers. */
export interface NativeServerDbContext {
  open(path: string): string;
  execute(sql: string, params?: readonly NativeServerSqlParam[]): boolean;
  query<T extends Record<string, unknown> = Record<string, unknown>>(
    sql: string,
    params?: readonly NativeServerSqlParam[],
  ): T[];
}

/** Sandboxed XDG filesystem context available to native server route handlers. */
export interface NativeServerFsContext {
  resolvePath(kind: NativeServerXdgKind, relPath: string): string;
  readText(path: string): string | null;
  writeText(path: string, content: string): boolean;
  watchFile(path: string): boolean;
  pollChangedFiles(): string[];
}

/** Multithreaded Rust compute helper exposed on `event.context.compute`. */
export interface NativeServerComputeContext {
  fuzzyFilter(items: readonly string[], query: string, limit?: number): NativeServerComputeMatch[];
  hash(input: string): string;
}

/** Outbound OS-native HTTP client exposed on `event.context.http`. */
export interface NativeServerHttpContext {
  request(
    method: string,
    url: string,
    body?: unknown,
    options?: NativeServerHttpRequestOptions,
  ): Promise<NativeServerHttpResponse>;
  get(url: string, options?: NativeServerHttpRequestOptions): Promise<NativeServerHttpResponse>;
  post(
    url: string,
    body?: unknown,
    options?: NativeServerHttpRequestOptions,
  ): Promise<NativeServerHttpResponse>;
}

/** Execution context injected into every handled `NativeServerEvent`. */
export interface NativeServerContext {
  params: Record<string, string>;
  query: Record<string, string>;
  db: NativeServerDbContext;
  fs: NativeServerFsContext;
  compute: NativeServerComputeContext;
  http: NativeServerHttpContext;
}

/** Incoming request payload routed to [`NativeServerRouter.handle`]. */
export interface NativeServerRequest {
  method: string;
  url: string;
  headers?: Record<string, string>;
  body?: string | null;
}

/** Serialized HTTP-like response returned by [`NativeServerRouter.handle`]. */
export interface NativeServerResponse {
  status: number;
  statusText: string;
  headers: Record<string, string>;
  body: string;
}

/** Event object passed to native file-based API route handlers (`Analog`/`h3` ergonomics). */
export interface NativeServerEvent {
  readonly method: string;
  readonly url: string;
  readonly path: string;
  readonly headers: Record<string, string>;
  readonly body: string | null;
  readonly context: NativeServerContext;
  readonly response: {
    status: number;
    statusText?: string;
    headers: Record<string, string>;
  };
}

/** Handler function signature exported by `src/server/routes/**` files. */
export type NativeEventHandler<T = unknown> = (event: NativeServerEvent) => T | Promise<T>;

/** Structured HTTP error converted into JSON responses by [`createNativeServerRouter`]. */
export class NativeServerHttpError extends Error {
  readonly statusCode: number;
  readonly statusMessage: string;
  readonly data?: unknown;

  constructor(options: {
    statusCode: number;
    statusMessage?: string;
    message?: string;
    data?: unknown;
  }) {
    const statusMessage = options.statusMessage ?? defaultStatusText(options.statusCode);
    super(options.message ?? statusMessage);
    this.name = 'NativeServerHttpError';
    this.statusCode = options.statusCode;
    this.statusMessage = statusMessage;
    this.data = options.data;
  }
}

/** Returns the standard HTTP reason phrase for common status codes. */
export function defaultStatusText(status: number): string {
  switch (status) {
    case 200:
      return 'OK';
    case 201:
      return 'Created';
    case 204:
      return 'No Content';
    case 400:
      return 'Bad Request';
    case 401:
      return 'Unauthorized';
    case 403:
      return 'Forbidden';
    case 404:
      return 'Not Found';
    case 405:
      return 'Method Not Allowed';
    case 409:
      return 'Conflict';
    case 500:
      return 'Internal Server Error';
    default:
      return status >= 400 ? 'Error' : 'OK';
  }
}

/** Identity helper providing type inference for local server route handlers. */
export function defineNativeEventHandler<T = unknown>(
  handler: NativeEventHandler<T>,
): NativeEventHandler<T> {
  return handler;
}

/** Reads a named dynamic segment (e.g. `[id]`) from the matched route parameters. */
export function getRouterParam(event: NativeServerEvent, name: string): string | undefined {
  return event.context.params[name];
}

/** Returns a shallow copy of the parsed URL query parameters for the current request. */
export function getQuery(event: NativeServerEvent): Record<string, string> {
  return { ...event.context.query };
}

/** Parses JSON payloads or returns raw request body strings for non-JSON requests. */
export function readBody<T = unknown>(event: NativeServerEvent): T {
  const raw = event.body;
  if (raw === null || raw === undefined || raw.length === 0) {
    return undefined as T;
  }
  const trimmed = raw.trim();
  if (
    trimmed.startsWith('{') ||
    trimmed.startsWith('[') ||
    trimmed.startsWith('"') ||
    trimmed === 'true' ||
    trimmed === 'false' ||
    trimmed === 'null' ||
    /^-?\d+(?:\.\d+)?$/.test(trimmed)
  ) {
    try {
      return JSON.parse(raw) as T;
    } catch {
      return raw as T;
    }
  }
  return raw as T;
}

/** Sets the HTTP status code (and optional reason phrase) on `event.response`. */
export function setResponseStatus(
  event: NativeServerEvent,
  code: number,
  statusText?: string,
): void {
  event.response.status = code;
  if (statusText !== undefined) {
    event.response.statusText = statusText;
  }
}

/** Sets a lowercase response header on `event.response`. */
export function setResponseHeader(event: NativeServerEvent, name: string, value: string): void {
  event.response.headers[name.toLowerCase()] = value;
}

/** Constructs a [`NativeServerHttpError`] for clean early-return error responses inside handlers. */
export function createError(input: {
  statusCode?: number;
  statusMessage?: string;
  message?: string;
  data?: unknown;
}): NativeServerHttpError {
  return new NativeServerHttpError({
    statusCode: input.statusCode ?? 500,
    statusMessage: input.statusMessage,
    message: input.message,
    data: input.data,
  });
}

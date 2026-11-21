export type NativeServerMethod =
  | 'GET'
  | 'POST'
  | 'PUT'
  | 'PATCH'
  | 'DELETE'
  | 'HEAD'
  | 'OPTIONS'
  | '*';

export type NativeServerXdgKind = 'config' | 'data' | 'cache';

export interface NativeServerComputeMatch {
  index: number;
  score: number;
  value: string;
}

export interface NativeServerHttpRequestOptions {
  headers?: Record<string, string>;
  callKind?: string;
}

export interface NativeServerHttpResponse {
  status: number;
  statusText: string;
  body: string;
  headers: Record<string, string>;
  cached?: boolean;
}

export type NativeServerSqlParam = string | number | boolean | null;

export interface NativeServerDbContext {
  open(path: string): string;
  execute(sql: string, params?: readonly NativeServerSqlParam[]): boolean;
  query<T extends Record<string, unknown> = Record<string, unknown>>(
    sql: string,
    params?: readonly NativeServerSqlParam[],
  ): T[];
}

export interface NativeServerFsContext {
  resolvePath(kind: NativeServerXdgKind, relPath: string): string;
  readText(path: string): string | null;
  writeText(path: string, content: string): boolean;
  watchFile(path: string): boolean;
  pollChangedFiles(): string[];
}

export interface NativeServerComputeContext {
  fuzzyFilter(items: readonly string[], query: string, limit?: number): NativeServerComputeMatch[];
  hash(input: string): string;
}

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

export interface NativeServerContext {
  params: Record<string, string>;
  query: Record<string, string>;
  db: NativeServerDbContext;
  fs: NativeServerFsContext;
  compute: NativeServerComputeContext;
  http: NativeServerHttpContext;
}

export interface NativeServerRequest {
  method: string;
  url: string;
  headers?: Record<string, string>;
  body?: string | null;
}

export interface NativeServerResponse {
  status: number;
  statusText: string;
  headers: Record<string, string>;
  body: string;
}

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

export type NativeEventHandler<T = unknown> = (event: NativeServerEvent) => T | Promise<T>;

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

export function defineNativeEventHandler<T = unknown>(
  handler: NativeEventHandler<T>,
): NativeEventHandler<T> {
  return handler;
}

export function getRouterParam(event: NativeServerEvent, name: string): string | undefined {
  return event.context.params[name];
}

export function getQuery(event: NativeServerEvent): Record<string, string> {
  return { ...event.context.query };
}

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

export function setResponseHeader(event: NativeServerEvent, name: string, value: string): void {
  event.response.headers[name.toLowerCase()] = value;
}

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

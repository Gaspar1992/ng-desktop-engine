import {
  computeFuzzyFilterFallback,
  computeHashFallback,
  InMemoryFsStore,
  type NativeComputeMatch,
  type NativeXdgDirKind,
  resolveDefaultAppId,
} from './bridge-fs-compute';
import { InMemorySqliteStore, resolveNativeImageAsset } from './bridge-sqlite';

export type { NativeComputeMatch, NativeXdgDirKind } from './bridge-fs-compute';

/** Logical transport category reported to Rust `NativeHttpEngine` and telemetry. */
export type NativeHttpCallKind = 'json' | 'graphql' | 'asset' | 'batch' | 'text';

/** OS network reachability classification exposed by `NativeNetworkService`. */
export type NativeConnectivity = 'full' | 'limited' | 'offline';

/** Snapshot of OS connectivity and metered network status. */
export interface NativeNetworkStatus {
  online: boolean;
  metered: boolean;
  connectivity: NativeConnectivity;
}

/** Per-request headers and options passed across `bridge.httpRequest`. */
export interface NativeHttpRequestOptions {
  headers?: Record<string, string>;
  responseType?: string;
  callKind?: NativeHttpCallKind;
  withCredentials?: boolean;
}

/** Global configuration shared with Rust `NativeHttpEngine` (HTTP/2, XSRF, caching). */
export interface NativeHttpConfig {
  platform?: string;
  useFetchTransport?: boolean;
  interceptorsEnabled?: boolean;
  xsrfEnabled?: boolean;
  xsrfCookieName?: string;
  xsrfHeaderName?: string;
  xsrfTokenValue?: string;
  jsonpEnabled?: boolean;
  requestsViaParent?: boolean;
  maxParallelConnections?: number;
  cacheEnabled?: boolean;
  timeoutMs?: number;
}

export interface NativeHttpBridgeResponse {
  status: number;
  statusText: string;
  body: string;
  headers?: Record<string, string>;
  transport?: string;
  callKind?: NativeHttpCallKind;
  cached?: boolean;
}

export interface NativeWindowGeometry {
  width: number;
  height: number;
  x?: number | null;
  y?: number | null;
  maximized: boolean;
}

export interface NativeMenuItem {
  id: string;
  label: string;
  accelerator?: string | null;
  enabled?: boolean;
}

export interface NativeTrayState {
  visible: boolean;
  icon: string;
  tooltip: string;
  menuItems?: readonly NativeMenuItem[];
}

export interface NativeShortcutBinding {
  accelerator: string;
  actionId: string;
}

export interface NativeConfirmDialogOptions {
  title?: string;
  message?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  defaultConfirmed?: boolean;
}

/**
 * Host FFI bridge contract injected at `globalThis.__NG_NATIVE__` by the Rust `JsEngineBackend`
 * (`JscEngine` or `QuickJsEngine`).
 */
export interface NgNativeBridge {
  registerStyle(
    cls: string,
    bg?: string,
    fg?: string,
    pad?: number,
    space?: number,
    extras?: string,
  ): void;
  declareSignal(name: string, initial?: number): void;
  createElement(id: string, tag: string): string;
  createComment(id: string, text?: string): string;
  setText(id: string, text: string): void;
  setAttribute(id: string, key: string, value: string): void;
  setWindowIcon(id: string, appId: string, icoPath: string): void;
  appendChild(parent: string, child: string): void;
  insertBefore(parent: string, child: string, refChild?: string): void;
  removeChild(parent: string, child: string): void;
  addClass(id: string, className: string): void;
  removeClass(id: string, className: string): void;
  bindText(id: string, signalName: string, template: string): void;
  bindAttr(id: string, attrKey: string, signalName: string, template?: string): void;
  bindVisible(id: string, signalName: string, op?: string, rhs?: number): void;
  onAdd(id: string, eventName: string, signalName: string, delta?: number): void;
  onSet(id: string, eventName: string, signalName: string, value?: number): void;
  listen(id: string, eventName: string, callback: (event?: unknown) => void): () => void;
  destroyNode(id: string): void;
  getSystemTheme(): 'dark' | 'light';
  setSystemTheme(theme: 'dark' | 'light'): void;
  getClipboard(): string;
  setClipboard(text: string): void;
  sendNotification(title: string, body: string, icon?: string): void;
  openFileDialog(title?: string, defaultPath?: string): string;
  saveFileDialog(title?: string, suggestedName?: string): string;
  confirmDialog?(options?: NativeConfirmDialogOptions): boolean;
  setNextConfirmDialogResponse?(confirmed: boolean): void;
  configureHttp(config: NativeHttpConfig): NativeHttpConfig;
  getHttpConfig(): NativeHttpConfig;
  registerHttpEndpoint(method: string, url: string, status: number, body: string): void;
  httpRequest(
    method: string,
    url: string,
    body?: string,
    options?: NativeHttpRequestOptions,
  ): NativeHttpBridgeResponse;
  httpBatchRequest?(
    requests: Array<{
      method: string;
      url: string;
      body?: string;
      options?: NativeHttpRequestOptions;
    }>,
  ): NativeHttpBridgeResponse[];
  httpRequestAsync?(
    method: string,
    url: string,
    body?: string,
    options?: NativeHttpRequestOptions,
  ): Promise<NativeHttpBridgeResponse>;
  httpBatchRequestAsync?(
    requests: Array<{
      method: string;
      url: string;
      body?: string;
      options?: NativeHttpRequestOptions;
    }>,
  ): Promise<NativeHttpBridgeResponse[]>;
  downloadImageAsync?(
    url: string,
    width?: number,
  ): Promise<{ ok: boolean; cached?: boolean; path: string; error?: string }>;
  sqliteExecute(sql: string): boolean;
  sqliteQuery(sql: string): Array<Record<string, string>>;
  getNetworkStatus(): NativeNetworkStatus;
  setNetworkStatus(online: boolean, metered?: boolean): void;
  getSystemLocale(): string;
  getSystemTimezone(): string;
  resolveImageAsset(src: string, width?: number): string;
  logSystemError(message: string): void;
  getSystemErrors(): string[];
  computeFuzzyFilter(items: readonly string[], query: string, limit?: number): NativeComputeMatch[];
  computeHash(input: string): string;
  fsResolvePath(kind: NativeXdgDirKind, relPath: string): string;
  fsReadText(path: string): string | null;
  fsWriteText(path: string, content: string): boolean;
  fsWatchFile(path: string): boolean;
  fsPollChangedFiles(): string[];
  getWindowGeometry?(): NativeWindowGeometry;
  setWindowGeometry?(geometry: Partial<NativeWindowGeometry>): NativeWindowGeometry;
  getAppMenu?(): NativeMenuItem[];
  setAppMenu?(items: readonly NativeMenuItem[]): NativeMenuItem[];
  getTrayState?(): NativeTrayState;
  setTrayState?(state: Partial<NativeTrayState>): NativeTrayState;
  registerShortcut?(accelerator: string, actionId: string): NativeShortcutBinding[];
  unregisterShortcut?(accelerator: string): NativeShortcutBinding[];
  getRegisteredShortcuts?(): NativeShortcutBinding[];
  __dispatchEvent(id: string, eventName: string, eventPayload?: unknown): void;
  __dispatchAndDrain?(id: string, eventName: string, eventPayload?: unknown): string;
  __flushTimers(): void;
  __tickTimers?(nowMs?: number): void;
  __tickAndDrain?(nowMs?: number, completions?: Array<[number, unknown]>): string;
  __resolveHostCallback?(id: number, payload: unknown): void;
  beginBatch?(): void;
  endBatch?(): void;
  __drainQueue(): string;
}

declare global {
  var __NG_NATIVE__: NgNativeBridge | undefined;
}

declare const __NG_DESKTOP_PROD_BUNDLE__: boolean | undefined;

/**
 * Ensures `globalThis.__NG_NATIVE__` is initialized.
 * In the Rust native runtime (`JscEngine` / `QuickJsEngine`), the host injects this at startup;
 * in Node/Vitest unit tests, this provides an identical in-memory command queue.
 */
export function ensureNgNativeBridge(): NgNativeBridge {
  if (globalThis.__NG_NATIVE__) {
    return globalThis.__NG_NATIVE__;
  }
  if (typeof __NG_DESKTOP_PROD_BUNDLE__ === 'undefined' || !__NG_DESKTOP_PROD_BUNDLE__) {
    let queue: string[] = [];
    let batchDepth = 0;
    const batchBuffer: string[] = [];
    const emit = (cmd: string): void => {
      if (batchDepth > 0) {
        batchBuffer.push(cmd);
      } else {
        queue.push(cmd);
      }
    };
    let systemTheme: 'dark' | 'light' = 'dark';
    let clipboardText = '';
    const sqliteStore = new InMemorySqliteStore();
    const fsStore = new InMemoryFsStore();
    let windowGeometry: NativeWindowGeometry = {
      width: 1020,
      height: 720,
      x: null,
      y: null,
      maximized: false,
    };
    let appMenuItems: NativeMenuItem[] = [];
    let trayState: NativeTrayState = {
      visible: false,
      icon: '',
      tooltip: '',
      menuItems: [],
    };
    let shortcuts: NativeShortcutBinding[] = [];
    let nextConfirmResponse: boolean | null = null;
    let networkStatus: NativeNetworkStatus = {
      online: true,
      metered: false,
      connectivity: 'full',
    };
    const systemLocale =
      (typeof Intl !== 'undefined' ? Intl.DateTimeFormat().resolvedOptions().locale : '') ||
      'en-US';
    const systemTimezone =
      (typeof Intl !== 'undefined' ? Intl.DateTimeFormat().resolvedOptions().timeZone : '') ||
      'UTC';
    const systemErrors: string[] = [];
    const httpEndpoints = new Map<string, NativeHttpBridgeResponse>();
    let httpConfig: NativeHttpConfig = {
      platform: 'js-fallback',
      useFetchTransport: true,
      interceptorsEnabled: true,
      xsrfEnabled: true,
      xsrfCookieName: 'XSRF-TOKEN',
      xsrfHeaderName: 'X-XSRF-TOKEN',
      xsrfTokenValue: '',
      jsonpEnabled: false,
      requestsViaParent: false,
      maxParallelConnections: 24,
      cacheEnabled: true,
      timeoutMs: 15_000,
    };
    const listenersByNode = new Map<string, Map<string, Map<number, (e?: unknown) => void>>>();
    let nextListenerId = 1;

    const escapeNewlines = (value: string): string => value.replace(/\r?\n/g, '\u001e');

    const fallbackBridge: NgNativeBridge = {
      registerStyle(cls, bg = 'none', fg = 'none', pad = 0, space = 0, extras = '') {
        const extraTrimmed = extras.trim();
        emit(
          extraTrimmed
            ? `style ${cls} ${bg} ${fg} ${pad} ${space} ${extraTrimmed}`
            : `style ${cls} ${bg} ${fg} ${pad} ${space}`,
        );
      },
      declareSignal(name, initial = 0) {
        emit(`signal ${name} ${initial}`);
      },
      createElement(id, tag) {
        emit(`create ${id} ${tag}`);
        return id;
      },
      createComment(id, text = 'anchor') {
        emit(`comment ${id} ${text}`);
        return id;
      },
      setText(id, text) {
        emit(`text ${id} ${escapeNewlines(text)}`);
      },
      setAttribute(id, key, value) {
        emit(`attr ${id} ${key} ${escapeNewlines(value)}`);
      },
      setWindowIcon(id, appId, icoPath) {
        emit(`icon ${id} ${appId} ${icoPath}`);
      },
      appendChild(parent, child) {
        emit(`append ${parent} ${child}`);
      },
      insertBefore(parent, child, refChild) {
        if (refChild) {
          emit(`insert_before ${parent} ${child} ${refChild}`);
        } else {
          emit(`insert_before ${parent} ${child}`);
        }
      },
      removeChild(parent, child) {
        emit(`remove_child ${parent} ${child}`);
      },
      addClass(id, className) {
        emit(`add_class ${id} ${className}`);
      },
      removeClass(id, className) {
        emit(`remove_class ${id} ${className}`);
      },
      bindText(id, signalName, template) {
        emit(`bind_text ${id} ${signalName} ${template}`);
      },
      bindAttr(id, attrKey, signalName, template = '{}') {
        emit(`bind_attr ${id} ${attrKey} ${signalName} ${template}`);
      },
      bindVisible(id, signalName, op = '!=', rhs = 0) {
        emit(`bind_visible ${id} ${signalName} ${op} ${rhs}`);
      },
      onAdd(id, eventName, signalName, delta = 1) {
        emit(`on_add ${id} ${eventName} ${signalName} ${delta}`);
      },
      onSet(id, eventName, signalName, value = 0) {
        emit(`on_set ${id} ${eventName} ${signalName} ${value}`);
      },
      listen(id, eventName, callback) {
        const lid = nextListenerId++;
        let byEvent = listenersByNode.get(id);
        if (!byEvent) {
          byEvent = new Map();
          listenersByNode.set(id, byEvent);
        }
        let callbacks = byEvent.get(eventName);
        if (!callbacks) {
          callbacks = new Map();
          byEvent.set(eventName, callbacks);
        }
        callbacks.set(lid, callback);
        return () => {
          callbacks.delete(lid);
          if (callbacks.size === 0) {
            byEvent.delete(eventName);
            if (byEvent.size === 0) {
              listenersByNode.delete(id);
            }
          }
        };
      },
      destroyNode(id) {
        listenersByNode.delete(id);
        emit(`destroy ${id}`);
      },
      getSystemTheme() {
        return systemTheme;
      },
      setSystemTheme(theme) {
        systemTheme = theme === 'light' ? 'light' : 'dark';
        emit(`theme ${systemTheme}`);
      },
      getClipboard() {
        return clipboardText;
      },
      setClipboard(text) {
        clipboardText = text;
      },
      sendNotification(title, body, icon = resolveDefaultAppId()) {
        emit(`comment notify_${nextListenerId++} [${icon}] ${title}: ${body}`);
      },
      openFileDialog(_title = 'Open File', defaultPath = '/home/user/document.txt') {
        return defaultPath;
      },
      saveFileDialog(_title = 'Save File', suggestedName = '/home/user/untitled.txt') {
        return suggestedName;
      },
      confirmDialog(options = {}) {
        if (nextConfirmResponse !== null) {
          const next = nextConfirmResponse;
          nextConfirmResponse = null;
          return next;
        }
        return options.defaultConfirmed !== false;
      },
      setNextConfirmDialogResponse(confirmed) {
        nextConfirmResponse = Boolean(confirmed);
      },
      configureHttp(config) {
        httpConfig = { ...httpConfig, ...config };
        return { ...httpConfig };
      },
      getHttpConfig() {
        return { ...httpConfig };
      },
      registerHttpEndpoint(method, url, status, body) {
        const key = `${method.trim().toUpperCase()} ${url.trim()}`;
        httpEndpoints.set(key, {
          status,
          statusText: status < 400 ? 'OK' : 'Error',
          body,
          headers: {},
          transport: 'mock-route',
          callKind: 'json',
          cached: false,
        });
      },
      httpRequest(method, url, _body, options) {
        const key = `${method.trim().toUpperCase()} ${url.trim()}`;
        return (
          httpEndpoints.get(key) ?? {
            status: 404,
            statusText: 'Not Found',
            body: '{"error":"Endpoint not registered"}',
            headers: {},
            transport: 'js-fallback',
            callKind: options?.callKind ?? 'json',
            cached: false,
          }
        );
      },
      httpBatchRequest(requests) {
        return requests.map((req) =>
          fallbackBridge.httpRequest(req.method, req.url, req.body, req.options),
        );
      },
      httpRequestAsync(method, url, body, options) {
        return Promise.resolve(fallbackBridge.httpRequest(method, url, body, options));
      },
      httpBatchRequestAsync(requests) {
        return Promise.resolve(fallbackBridge.httpBatchRequest!(requests));
      },
      downloadImageAsync(url, width) {
        return Promise.resolve({
          ok: true,
          cached: false,
          path: fallbackBridge.resolveImageAsset(url, width),
        });
      },
      sqliteExecute(sql) {
        return sqliteStore.execute(sql);
      },
      sqliteQuery(sql) {
        return sqliteStore.query(sql);
      },
      getNetworkStatus() {
        return { ...networkStatus };
      },
      setNetworkStatus(online, metered = false) {
        networkStatus = {
          online,
          metered,
          connectivity: !online ? 'offline' : metered ? 'limited' : 'full',
        };
      },
      getSystemLocale() {
        return systemLocale;
      },
      getSystemTimezone() {
        return systemTimezone;
      },
      resolveImageAsset(src, width) {
        return resolveNativeImageAsset(src, width);
      },
      logSystemError(message) {
        systemErrors.push(String(message));
      },
      getSystemErrors() {
        return [...systemErrors];
      },
      computeFuzzyFilter(items, query, limit = 50) {
        return computeFuzzyFilterFallback(items, query, limit);
      },
      computeHash(input) {
        return computeHashFallback(input);
      },
      fsResolvePath(kind, relPath) {
        return fsStore.resolvePath(kind, relPath);
      },
      fsReadText(path) {
        return fsStore.readText(path);
      },
      fsWriteText(path, content) {
        return fsStore.writeText(path, content);
      },
      fsWatchFile(path) {
        return fsStore.watchFile(path);
      },
      fsPollChangedFiles() {
        return fsStore.pollChangedFiles();
      },
      getWindowGeometry() {
        return { ...windowGeometry };
      },
      setWindowGeometry(geom) {
        windowGeometry = {
          width:
            typeof geom.width === 'number' && geom.width >= 160
              ? Math.trunc(geom.width)
              : windowGeometry.width,
          height:
            typeof geom.height === 'number' && geom.height >= 120
              ? Math.trunc(geom.height)
              : windowGeometry.height,
          x:
            typeof geom.x === 'number'
              ? Math.trunc(geom.x)
              : geom.x === null
                ? null
                : (windowGeometry.x ?? null),
          y:
            typeof geom.y === 'number'
              ? Math.trunc(geom.y)
              : geom.y === null
                ? null
                : (windowGeometry.y ?? null),
          maximized:
            typeof geom.maximized === 'boolean' ? geom.maximized : windowGeometry.maximized,
        };
        return { ...windowGeometry };
      },
      getAppMenu() {
        return appMenuItems.map((item) => ({ ...item }));
      },
      setAppMenu(items) {
        appMenuItems = items.map((item) => ({
          id: String(item.id),
          label: String(item.label || item.id),
          accelerator: item.accelerator ?? null,
          enabled: item.enabled !== false,
        }));
        return appMenuItems.map((item) => ({ ...item }));
      },
      getTrayState() {
        return { ...trayState, menuItems: [...(trayState.menuItems ?? [])] };
      },
      setTrayState(state) {
        trayState = {
          visible: typeof state.visible === 'boolean' ? state.visible : trayState.visible,
          icon: typeof state.icon === 'string' ? state.icon : trayState.icon,
          tooltip: typeof state.tooltip === 'string' ? state.tooltip : trayState.tooltip,
          menuItems: state.menuItems ? [...state.menuItems] : [...(trayState.menuItems ?? [])],
        };
        return { ...trayState, menuItems: [...(trayState.menuItems ?? [])] };
      },
      registerShortcut(accelerator, actionId) {
        const cleanAcc = accelerator.trim();
        const cleanAction = actionId.trim();
        if (!cleanAcc || !cleanAction) {
          return shortcuts.map((s) => ({ ...s }));
        }
        const existing = shortcuts.find((s) => s.accelerator === cleanAcc);
        if (existing) {
          existing.actionId = cleanAction;
        } else {
          shortcuts.push({ accelerator: cleanAcc, actionId: cleanAction });
        }
        return shortcuts.map((s) => ({ ...s }));
      },
      unregisterShortcut(accelerator) {
        const cleanAcc = accelerator.trim();
        shortcuts = shortcuts.filter((s) => s.accelerator !== cleanAcc);
        return shortcuts.map((s) => ({ ...s }));
      },
      getRegisteredShortcuts() {
        return shortcuts.map((s) => ({ ...s }));
      },
      __dispatchEvent(id, eventName, eventPayload = {}) {
        const callbacks = listenersByNode.get(id)?.get(eventName);
        if (!callbacks || callbacks.size === 0) {
          return;
        }
        for (const callback of Array.from(callbacks.values())) {
          callback(eventPayload);
        }
      },
      __dispatchAndDrain(id, eventName, eventPayload = {}) {
        fallbackBridge.__dispatchEvent(id, eventName, eventPayload);
        fallbackBridge.__flushTimers();
        return fallbackBridge.__drainQueue();
      },
      __flushTimers() {
        // No-op in synchronous fallback
      },
      __tickTimers() {
        // No-op in synchronous fallback
      },
      __tickAndDrain(nowMs, completions = []) {
        if (Array.isArray(completions)) {
          for (const [cbId, payload] of completions) {
            fallbackBridge.__resolveHostCallback?.(cbId, payload);
          }
        }
        fallbackBridge.__tickTimers?.(nowMs);
        return fallbackBridge.__drainQueue();
      },
      __resolveHostCallback() {
        // No-op in synchronous fallback
      },
      beginBatch() {
        batchDepth += 1;
      },
      endBatch() {
        if (batchDepth > 0) {
          batchDepth -= 1;
        }
        if (batchDepth === 0 && batchBuffer.length > 0) {
          queue.push(...batchBuffer);
          batchBuffer.length = 0;
        }
      },
      __drainQueue() {
        if (queue.length === 0) {
          return '';
        }
        const out = queue.join('\n');
        queue = [];
        return out;
      },
    };

    globalThis.__NG_NATIVE__ = fallbackBridge;
    return fallbackBridge;
  }

  throw new Error('Native __NG_NATIVE__ bridge is required in production bundles.');
}

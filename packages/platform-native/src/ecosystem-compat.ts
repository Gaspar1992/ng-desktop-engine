import {
  EnvironmentProviders,
  Inject,
  Injectable,
  InjectionToken,
  makeEnvironmentProviders,
  Optional,
} from '@angular/core';
import { ensureNgNativeBridge, NgNativeBridge } from './bridge';
import { getNativeActiveElement, NativeElementNode, NativeEventListener } from './native-element';

export type AriaPoliteness = 'off' | 'polite' | 'assertive';

export interface NativeAriaAnnouncement {
  readonly message: string;
  readonly politeness: AriaPoliteness;
}

export interface NativeDevToolsRecord {
  readonly action: unknown;
  readonly state: unknown;
}

export interface NativeDevToolsConnection {
  init(state: unknown): void;
  send(action: unknown, state: unknown): void;
  subscribe(listener: (message: unknown) => void): () => void;
  unsubscribe(): void;
}

export interface NativeDocumentFacade {
  readonly activeElement: NativeElementNode | null;
  readonly body: NativeElementNode;
  createElement(tag: string): NativeElementNode;
  createComment(text?: string): NativeElementNode;
  createTextNode(text: string): NativeElementNode;
  getElementById(id: string): NativeElementNode | null;
  querySelector(selector: string): NativeElementNode | null;
  addEventListener(type: string, listener: NativeEventListener): void;
  removeEventListener(type: string, listener: NativeEventListener): void;
}

export interface NativeEcosystemCompatOptions {
  /** Install `NativeStorage` onto `globalThis.localStorage` when absent. Defaults to `true`. */
  installGlobalStorage?: boolean;
  /** Install `NativeDevToolsHook` onto `globalThis.__REDUX_DEVTOOLS_EXTENSION__`. Defaults to `true`. */
  installDevToolsExtension?: boolean;
}

function escapeSqlLiteral(value: string): string {
  return value.replaceAll("'", "''");
}

export const NATIVE_BRIDGE = new InjectionToken<NgNativeBridge>('NATIVE_BRIDGE', {
  providedIn: 'root',
  factory: () => ensureNgNativeBridge(),
});

/**
 * Web `Storage`-compatible persistence adapter backed by the native SQLite engine (`ng_native_storage`)
 * and an in-memory map, enabling `@ngrx/store` meta-reducers and state hydration without a browser DOM.
 */
@Injectable({ providedIn: 'root' })
export class NativeStorage implements Storage {
  private readonly store = new Map<string, string>();
  private readonly bridge: NgNativeBridge;
  private nextId = 1;

  constructor(@Optional() @Inject(NATIVE_BRIDGE) bridge?: NgNativeBridge | null) {
    this.bridge = bridge ?? ensureNgNativeBridge();
    this.ensureTable();
  }

  private ensureTable(): void {
    try {
      this.bridge.sqliteExecute(
        'CREATE TABLE IF NOT EXISTS ng_native_storage (key TEXT PRIMARY KEY, val TEXT NOT NULL)',
      );
      const rows = this.bridge.sqliteQuery('SELECT key, val FROM ng_native_storage');
      for (const row of rows) {
        const key = row['key'];
        const val = row['val'];
        if (typeof key === 'string' && typeof val === 'string') {
          this.store.set(key, val);
        }
      }
    } catch {
      // Fall back silently to in-memory storage if sqlite capability is restricted at runtime
    }
  }

  get length(): number {
    return this.store.size;
  }

  key(index: number): string | null {
    if (index < 0 || index >= this.store.size) {
      return null;
    }
    return Array.from(this.store.keys())[index] ?? null;
  }

  getItem(key: string): string | null {
    return this.store.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    const strValue = String(value);
    const idNum = this.nextId++;
    this.store.set(key, strValue);
    try {
      this.bridge.sqliteExecute(
        `INSERT OR REPLACE INTO ng_native_storage (key, val) VALUES ('${escapeSqlLiteral(key)}', '${escapeSqlLiteral(strValue)}')`,
      );
      this.bridge.setAttribute('__ng_native_storage__', `k_${idNum}`, key);
    } catch {
      // In-memory store remains consistent when disk persistence is unavailable
    }
  }

  removeItem(key: string): void {
    this.store.delete(key);
    try {
      this.bridge.sqliteExecute(
        `DELETE FROM ng_native_storage WHERE key = '${escapeSqlLiteral(key)}'`,
      );
    } catch {
      // Ignore bridge persistence error in restricted sandboxes
    }
  }

  clear(): void {
    this.store.clear();
    try {
      this.bridge.sqliteExecute('DELETE FROM ng_native_storage');
    } catch {
      // Ignore bridge persistence error in restricted sandboxes
    }
  }
}

/**
 * Headless `@angular/cdk/a11y` `LiveAnnouncer`-compatible service for `@angular/aria` and native
 * GTK4/WinUI/AppKit accessibility announcements.
 */
@Injectable({ providedIn: 'root' })
export class NativeAriaAnnouncer {
  private readonly announcements: NativeAriaAnnouncement[] = [];
  private readonly bridge: NgNativeBridge;

  constructor(@Optional() @Inject(NATIVE_BRIDGE) bridge?: NgNativeBridge | null) {
    this.bridge = bridge ?? ensureNgNativeBridge();
  }

  announce(message: string, politeness: AriaPoliteness = 'polite'): Promise<void> {
    const clean = String(message ?? '').trim();
    if (!clean) {
      return Promise.resolve();
    }
    this.announcements.push({ message: clean, politeness });
    this.bridge.createComment(`aria_live_${this.announcements.length}`, `${politeness}:${clean}`);
    return Promise.resolve();
  }

  clear(): void {
    this.announcements.length = 0;
  }

  history(): readonly NativeAriaAnnouncement[] {
    return this.announcements;
  }
}

/**
 * Redux DevTools (`__REDUX_DEVTOOLS_EXTENSION__`) bridge allowing `@ngrx/store-devtools`
 * to attach cleanly inside JavaScriptCore / QuickJS without browser extensions.
 */
@Injectable({ providedIn: 'root' })
export class NativeDevToolsHook {
  private readonly records: NativeDevToolsRecord[] = [];
  private readonly subscribers = new Set<(message: unknown) => void>();
  private latestState: unknown = undefined;

  connect(): NativeDevToolsConnection {
    return {
      init: (state: unknown) => {
        this.latestState = state;
        this.records.push({ action: { type: '@@INIT' }, state });
      },
      send: (action: unknown, state: unknown) => {
        this.latestState = state;
        const record: NativeDevToolsRecord = { action, state };
        this.records.push(record);
        for (const listener of this.subscribers) {
          listener({ type: 'ACTION_RECORDED', payload: record });
        }
      },
      subscribe: (listener: (message: unknown) => void) => {
        this.subscribers.add(listener);
        return () => {
          this.subscribers.delete(listener);
        };
      },
      unsubscribe: () => {
        this.subscribers.clear();
      },
    };
  }

  history(): readonly NativeDevToolsRecord[] {
    return this.records;
  }

  currentState(): unknown {
    return this.latestState;
  }
}

function findNodeById(root: NativeElementNode, id: string): NativeElementNode | null {
  if (root.id === id || root.getAttribute('id') === id) {
    return root;
  }
  for (const child of root.children) {
    const found = findNodeById(child, id);
    if (found) {
      return found;
    }
  }
  return null;
}

function findNodeBySelector(root: NativeElementNode, selector: string): NativeElementNode | null {
  const clean = selector.trim();
  if (!clean) {
    return null;
  }
  if (clean.startsWith('#')) {
    return findNodeById(root, clean.slice(1));
  }
  if (clean.startsWith('.')) {
    const className = clean.slice(1);
    if (root.classList.contains(className)) {
      return root;
    }
  } else if (root.tag.toLowerCase() === clean.toLowerCase()) {
    return root;
  }
  for (const child of root.children) {
    const match = findNodeBySelector(child, clean);
    if (match) {
      return match;
    }
  }
  return null;
}

/**
 * Creates a lightweight headless `Document` facade backed by `NativeElementNode` for
 * `@angular/cdk`, `@angular/aria`, and DOM-agnostic UI primitives.
 */
export function createNativeDocument(root?: NativeElementNode): NativeDocumentFacade {
  const body = root ?? new NativeElementNode('__body__', 'ng-window');
  let nextSeq = 1;

  return {
    get activeElement(): NativeElementNode | null {
      return getNativeActiveElement();
    },
    body,
    createElement(tag: string): NativeElementNode {
      return new NativeElementNode(`doc_el_${nextSeq++}`, tag);
    },
    createComment(text = ''): NativeElementNode {
      const node = new NativeElementNode(`doc_comment_${nextSeq++}`, '#comment', true);
      node.textContent = text;
      return node;
    },
    createTextNode(text: string): NativeElementNode {
      const node = new NativeElementNode(`doc_text_${nextSeq++}`, '#text');
      node.textContent = text;
      return node;
    },
    getElementById(id: string): NativeElementNode | null {
      return findNodeById(body, id);
    },
    querySelector(selector: string): NativeElementNode | null {
      return findNodeBySelector(body, selector);
    },
    addEventListener(type: string, listener: NativeEventListener): void {
      body.addEventListener(type, listener);
    },
    removeEventListener(type: string, listener: NativeEventListener): void {
      body.removeEventListener(type, listener);
    },
  };
}

export const NATIVE_STORAGE = new InjectionToken<Storage>('NATIVE_STORAGE');
export const NATIVE_DOCUMENT = new InjectionToken<NativeDocumentFacade>('NATIVE_DOCUMENT');
export const NATIVE_DEVTOOLS_HOOK = new InjectionToken<NativeDevToolsHook>('NATIVE_DEVTOOLS_HOOK');

/**
 * Registers compatibility providers (`NativeStorage`, `NativeAriaAnnouncer`, `NativeDevToolsHook`,
 * and `NativeDocumentFacade`) so state & headless UI libraries (`NgRx`, `@angular/aria`,
 * `@angular/cdk`) work smoothly inside the native desktop runtime.
 */
export function provideNativeEcosystemCompat(
  options: NativeEcosystemCompatOptions = {},
): EnvironmentProviders {
  const storage = new NativeStorage();
  const devTools = new NativeDevToolsHook();
  const nativeDocument = createNativeDocument();
  const globalBag = globalThis as Record<string, unknown>;

  if (options.installGlobalStorage !== false && globalBag['localStorage'] === undefined) {
    globalBag['localStorage'] = storage;
  }
  if (
    options.installDevToolsExtension !== false &&
    globalBag['__REDUX_DEVTOOLS_EXTENSION__'] === undefined
  ) {
    globalBag['__REDUX_DEVTOOLS_EXTENSION__'] = devTools;
  }

  return makeEnvironmentProviders([
    { provide: NativeStorage, useValue: storage },
    { provide: NATIVE_STORAGE, useValue: storage },
    NativeAriaAnnouncer,
    { provide: NativeDevToolsHook, useValue: devTools },
    { provide: NATIVE_DEVTOOLS_HOOK, useValue: devTools },
    { provide: NATIVE_DOCUMENT, useValue: nativeDocument },
  ]);
}

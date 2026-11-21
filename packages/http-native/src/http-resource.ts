import { computed, Signal, signal, WritableSignal } from '@angular/core';
import {
  ensureNgNativeBridge,
  NativeHttpBridgeResponse,
  NativeHttpRequestOptions,
} from '@ng-desktop/platform';

export type NativeResourceStatus = 'idle' | 'loading' | 'resolved' | 'error';

export interface NativeHttpResourceOptions<T> {
  request?: NativeHttpRequestOptions;
  defaultValue?: T;
  map?: (raw: unknown) => T;
}

export interface NativeHttpResourceRef<T> {
  readonly url: Signal<string | undefined>;
  readonly value: WritableSignal<T | undefined>;
  readonly status: WritableSignal<NativeResourceStatus>;
  readonly error: WritableSignal<unknown>;
  readonly isLoading: Signal<boolean>;
  readonly hasValue: Signal<boolean>;
  reload(): boolean;
  reloadAsync(): Promise<boolean>;
}

/**
 * Signal-based reactive HTTP resource backed by the OS-native Rust HTTP bridge (`ensureNgNativeBridge()`).
 * Mirrors Angular's `httpResource()` ergonomics (`value`, `status`, `error`, `isLoading`, `hasValue`, `reload`, `reloadAsync`).
 */
export function createNativeHttpResource<T = unknown>(
  urlOrFn: string | (() => string | undefined),
  options?: NativeHttpResourceOptions<T>,
): NativeHttpResourceRef<T> {
  const resolveUrl = (): string | undefined => {
    const raw = typeof urlOrFn === 'function' ? urlOrFn() : urlOrFn;
    const clean = raw?.trim();
    return clean && clean.length > 0 ? clean : undefined;
  };

  const url = computed(() => resolveUrl());
  const value = signal<T | undefined>(options?.defaultValue);
  const status = signal<NativeResourceStatus>('idle');
  const error = signal<unknown>(undefined);
  const isLoading = computed(() => status() === 'loading');
  const hasValue = computed(() => value() !== undefined);
  let requestSeq = 0;

  const applyResponse = (targetUrl: string, res: NativeHttpBridgeResponse): boolean => {
    if (res.status >= 200 && res.status < 300) {
      const responseType = options?.request?.responseType ?? 'json';
      let parsed: unknown = res.body;
      if (responseType === 'json' && typeof res.body === 'string' && res.body.length > 0) {
        parsed = JSON.parse(res.body);
      }
      const finalValue = options?.map ? options.map(parsed) : (parsed as T);
      value.set(finalValue);
      status.set('resolved');
      return true;
    }

    error.set({
      status: res.status,
      statusText: res.statusText,
      body: res.body,
      url: targetUrl,
    });
    status.set('error');
    return false;
  };

  const resetToIdle = (): false => {
    status.set('idle');
    error.set(undefined);
    return false;
  };

  const reload = (): boolean => {
    const seq = ++requestSeq;
    const targetUrl = resolveUrl();
    if (!targetUrl) {
      return resetToIdle();
    }

    status.set('loading');
    error.set(undefined);
    try {
      const bridge = ensureNgNativeBridge();
      const res = bridge.httpRequest('GET', targetUrl, undefined, options?.request);
      if (seq !== requestSeq) {
        return false;
      }
      return applyResponse(targetUrl, res);
    } catch (err) {
      if (seq !== requestSeq) {
        return false;
      }
      error.set(err);
      status.set('error');
      return false;
    }
  };

  const reloadAsync = async (): Promise<boolean> => {
    const seq = ++requestSeq;
    const targetUrl = resolveUrl();
    if (!targetUrl) {
      return resetToIdle();
    }

    status.set('loading');
    error.set(undefined);
    try {
      const bridge = ensureNgNativeBridge();
      const res =
        typeof bridge.httpRequestAsync === 'function'
          ? await bridge.httpRequestAsync('GET', targetUrl, undefined, options?.request)
          : bridge.httpRequest('GET', targetUrl, undefined, options?.request);
      if (seq !== requestSeq) {
        return false;
      }
      return applyResponse(targetUrl, res);
    } catch (err) {
      if (seq !== requestSeq) {
        return false;
      }
      error.set(err);
      status.set('error');
      return false;
    }
  };

  reload();

  return {
    url,
    value,
    status,
    error,
    isLoading,
    hasValue,
    reload,
    reloadAsync,
  };
}

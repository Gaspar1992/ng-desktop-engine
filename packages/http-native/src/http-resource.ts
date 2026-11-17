import { computed, Signal, signal, WritableSignal } from '@angular/core';
import { ensureNgNativeBridge, NativeHttpRequestOptions } from '@ng-native/platform';

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
}

/**
 * Signal-based reactive HTTP resource backed by the OS-native Rust HTTP bridge (`ensureNgNativeBridge()`).
 * Mirrors Angular's `httpResource()` ergonomics (`value`, `status`, `error`, `isLoading`, `hasValue`, `reload`).
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

  const reload = (): boolean => {
    const targetUrl = resolveUrl();
    if (!targetUrl) {
      status.set('idle');
      error.set(undefined);
      return false;
    }

    status.set('loading');
    error.set(undefined);
    try {
      const bridge = ensureNgNativeBridge();
      const res = bridge.httpRequest('GET', targetUrl, undefined, options?.request);
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
    } catch (err) {
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
  };
}

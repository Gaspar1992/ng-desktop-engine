import {
  HttpBackend,
  HttpErrorResponse,
  HttpEvent,
  HttpFeature,
  HttpFeatureKind,
  HttpHeaders,
  HttpRequest,
  HttpResponse,
  provideHttpClient as angularProvideHttpClient,
  withFetch,
  withInterceptors,
  withInterceptorsFromDi,
  withJsonpSupport,
  withNoXsrfProtection,
  withRequestsMadeViaParent,
  withXhr,
  withXsrfConfiguration,
} from '@angular/common/http';
import {
  EnvironmentProviders,
  Injectable,
  InjectionToken,
  makeEnvironmentProviders,
  Provider,
} from '@angular/core';
import {
  ensureNgNativeBridge,
  type NativeHttpBridgeResponse,
  NativeHttpCallKind,
  NativeHttpConfig,
} from '@ng-desktop/platform';
import { Observable } from 'rxjs';

export {
  HttpFeatureKind,
  withFetch,
  withInterceptors,
  withInterceptorsFromDi,
  withJsonpSupport,
  withNoXsrfProtection,
  withRequestsMadeViaParent,
  withXhr,
  withXsrfConfiguration,
};

/**
 * Injection token holding the resolved `NativeHttpConfig` translated from `provideHttpClient(...)` features.
 */
export const NATIVE_HTTP_CONFIG = new InjectionToken<NativeHttpConfig>('NATIVE_HTTP_CONFIG');

const NATIVE_CONFIG_OVERRIDE_TOKEN = new InjectionToken<Partial<NativeHttpConfig>>(
  'NATIVE_HTTP_CONFIG_OVERRIDE',
);

interface InternalHttpFeature {
  ɵkind: HttpFeatureKind;
  ɵproviders?: unknown[];
}

interface ValueProviderLike {
  provide?: unknown;
  useValue?: unknown;
}

function isNativeConfigProvider(
  item: unknown,
): item is { provide: unknown; useValue: Partial<NativeHttpConfig> } {
  return (
    typeof item === 'object' &&
    item !== null &&
    (item as ValueProviderLike).provide === NATIVE_CONFIG_OVERRIDE_TOKEN &&
    typeof (item as ValueProviderLike).useValue === 'object' &&
    (item as ValueProviderLike).useValue !== null
  );
}

function applyXsrfProviders(providers: unknown[] | undefined, config: NativeHttpConfig): void {
  if (!providers) {
    return;
  }
  const stringValues: string[] = [];
  for (const item of providers) {
    if (typeof item !== 'object' || item === null) {
      continue;
    }
    const candidate = item as ValueProviderLike;
    if (typeof candidate.useValue !== 'string') {
      continue;
    }
    const desc = String(candidate.provide ?? '');
    if (desc.includes('XSRF_COOKIE_NAME')) {
      config.xsrfCookieName = candidate.useValue;
    } else if (desc.includes('XSRF_HEADER_NAME')) {
      config.xsrfHeaderName = candidate.useValue;
    } else {
      stringValues.push(candidate.useValue);
    }
  }
  if (stringValues.length >= 1 && config.xsrfCookieName === 'XSRF-TOKEN') {
    config.xsrfCookieName = stringValues[0];
  }
  if (stringValues.length >= 2 && config.xsrfHeaderName === 'X-XSRF-TOKEN') {
    config.xsrfHeaderName = stringValues[1];
  }
}

/**
 * Translates Angular `HttpFeature` configurations (`withFetch`, `withXsrfConfiguration`,
 * `withNoXsrfProtection`, `withInterceptors`, `withJsonpSupport`, and `@ng-desktop/http` features)
 * into a Rust-compatible `NativeHttpConfig`.
 */
export function resolveNativeHttpConfig(
  features: readonly HttpFeature<HttpFeatureKind>[],
): NativeHttpConfig {
  const config: NativeHttpConfig = {
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

  for (const feature of features) {
    const internal = feature as unknown as InternalHttpFeature;
    if (internal.ɵproviders) {
      for (const item of internal.ɵproviders) {
        if (isNativeConfigProvider(item)) {
          Object.assign(config, item.useValue);
        }
      }
    }

    switch (internal.ɵkind) {
      case HttpFeatureKind.Fetch:
        config.useFetchTransport = true;
        break;
      case HttpFeatureKind.Xhr:
        config.useFetchTransport = false;
        break;
      case HttpFeatureKind.Interceptors:
      case HttpFeatureKind.LegacyInterceptors:
        config.interceptorsEnabled = true;
        break;
      case HttpFeatureKind.NoXsrfProtection:
        config.xsrfEnabled = false;
        break;
      case HttpFeatureKind.CustomXsrfConfiguration:
        config.xsrfEnabled = true;
        applyXsrfProviders(internal.ɵproviders, config);
        break;
      case HttpFeatureKind.JsonpSupport:
        config.jsonpEnabled = true;
        break;
      case HttpFeatureKind.RequestsMadeViaParent:
        config.requestsViaParent = true;
        break;
    }
  }

  return config;
}

/**
 * Configures native engine HTTP options alongside standard Angular `provideHttpClient(...)` features.
 */
export function withNativeHttpConfig(
  config: Partial<NativeHttpConfig>,
): HttpFeature<HttpFeatureKind> {
  return {
    ɵkind: HttpFeatureKind.Interceptors,
    ɵproviders: [{ provide: NATIVE_CONFIG_OVERRIDE_TOKEN, useValue: config } satisfies Provider],
  } as unknown as HttpFeature<HttpFeatureKind>;
}

/**
 * Enables or disables response caching in the Rust OS-native HTTP engine.
 */
export function withHttpCaching(cacheEnabled: boolean): HttpFeature<HttpFeatureKind> {
  return withNativeHttpConfig({ cacheEnabled });
}

/**
 * Tunes parallel multiplexed HTTP/2 connections for batch and asset transfers in Rust.
 */
export function withParallelBatching(maxParallelConnections: number): HttpFeature<HttpFeatureKind> {
  return withNativeHttpConfig({ maxParallelConnections: Math.max(1, maxParallelConnections) });
}

export function detectNativeCallKind(req: HttpRequest<unknown>): NativeHttpCallKind {
  const overrideKind = req.headers.get('X-Ng-Native-Call-Kind')?.trim().toLowerCase();
  if (
    overrideKind === 'graphql' ||
    overrideKind === 'asset' ||
    overrideKind === 'batch' ||
    overrideKind === 'text' ||
    overrideKind === 'json'
  ) {
    return overrideKind;
  }
  if (req.responseType === 'arraybuffer' || req.responseType === 'blob') {
    return 'asset';
  }
  if (req.responseType === 'text') {
    return 'text';
  }
  const cleanUrl = req.urlWithParams.split('?')[0].toLowerCase();
  if (req.method.toUpperCase() === 'POST' && cleanUrl.includes('graphql')) {
    return 'graphql';
  }
  if (/\.(png|jpe?g|webp|svg|ico|bin)$/i.test(cleanUrl)) {
    return 'asset';
  }
  if (/\.(txt|html|css|xml|csv)$/i.test(cleanUrl)) {
    return 'text';
  }
  return 'json';
}

function extractRequestHeaders(req: HttpRequest<unknown>): Record<string, string> {
  const headers: Record<string, string> = {};
  for (const key of req.headers.keys()) {
    const values = req.headers.getAll(key);
    if (values && values.length > 0) {
      headers[key] = values.join(', ');
    }
  }
  return headers;
}

/**
 * Custom Angular `HttpBackend` that bypasses browser `XMLHttpRequest` / `fetch`
 * and executes requests through `globalThis.__NG_NATIVE__.httpRequest` in the Rust OS-native HTTP engine
 * (`libsoup-3.0` on Linux, `NSURLSession` on macOS, `WinHTTP` on Windows).
 * Compatible with `HttpClient`, functional `HttpInterceptorFn`, and `httpResource()`.
 */
@Injectable({ providedIn: 'root' })
export class NativeHttpBackend implements HttpBackend {
  handle(req: HttpRequest<unknown>): Observable<HttpEvent<unknown>> {
    return new Observable<HttpEvent<unknown>>((observer) => {
      let cancelled = false;
      const emitResponse = (rawRes: NativeHttpBridgeResponse): void => {
        if (cancelled) {
          return;
        }
        const headers = new HttpHeaders(rawRes.headers ?? {});
        let parsedBody: unknown = rawRes.body;
        if (req.responseType === 'json' && rawRes.body.length > 0) {
          try {
            parsedBody = JSON.parse(rawRes.body);
          } catch (parseErr) {
            if (rawRes.status >= 200 && rawRes.status < 300) {
              observer.error(
                new HttpErrorResponse({
                  error: { error: parseErr, text: rawRes.body },
                  headers,
                  status: rawRes.status,
                  statusText: rawRes.statusText || 'OK',
                  url: req.urlWithParams,
                }),
              );
              return;
            }
            parsedBody = rawRes.body;
          }
        } else if (req.responseType === 'blob' && typeof Blob === 'function') {
          parsedBody = new Blob([rawRes.body]);
        } else if (req.responseType === 'arraybuffer' && typeof TextEncoder === 'function') {
          parsedBody = new TextEncoder().encode(rawRes.body).buffer;
        }

        if (rawRes.status >= 200 && rawRes.status < 300) {
          observer.next(
            new HttpResponse({
              body: parsedBody,
              headers,
              status: rawRes.status,
              statusText: rawRes.statusText,
              url: req.urlWithParams,
            }),
          );
          observer.complete();
        } else {
          observer.error(
            new HttpErrorResponse({
              error: parsedBody,
              headers,
              status: rawRes.status,
              statusText: rawRes.statusText,
              url: req.urlWithParams,
            }),
          );
        }
      };

      try {
        const bridge = ensureNgNativeBridge();
        const serializedBody = req.serializeBody();
        const bodyStr =
          serializedBody === null || serializedBody === undefined
            ? undefined
            : typeof serializedBody === 'string'
              ? serializedBody
              : typeof URLSearchParams !== 'undefined' && serializedBody instanceof URLSearchParams
                ? serializedBody.toString()
                : JSON.stringify(serializedBody);

        const callKind = detectNativeCallKind(req);
        const options = {
          headers: extractRequestHeaders(req),
          responseType: req.responseType,
          callKind,
          withCredentials: req.withCredentials,
        };
        const hasHostService =
          typeof (globalThis as { __NG_NATIVE_HOST_SERVICE__?: unknown })
            .__NG_NATIVE_HOST_SERVICE__ === 'function';

        if (hasHostService && typeof bridge.httpRequestAsync === 'function') {
          bridge
            .httpRequestAsync(req.method, req.urlWithParams, bodyStr, options)
            .then(emitResponse)
            .catch((err) => {
              if (!cancelled) {
                observer.error(err);
              }
            });
          return () => {
            cancelled = true;
          };
        }

        const rawRes = bridge.httpRequest(req.method, req.urlWithParams, bodyStr, options);
        emitResponse(rawRes);
      } catch (err) {
        observer.error(err);
      }
      return undefined;
    });
  }
}

/**
 * Drop-in Angular `provideHttpClient(...)` provider for native desktop apps.
 * Translates Angular features (`withFetch`, `withInterceptors`, `withXsrfConfiguration`,
 * `withNoXsrfProtection`, `withJsonpSupport`, `withRequestsMadeViaParent`, and native transport
 * helpers) into the Rust OS-native HTTP engine and routes `HttpClient` through `NativeHttpBackend`.
 */
export function provideHttpClient(
  ...features: HttpFeature<HttpFeatureKind>[]
): EnvironmentProviders {
  const resolvedConfig = resolveNativeHttpConfig(features);
  const appliedConfig = ensureNgNativeBridge().configureHttp(resolvedConfig);

  const sanitizedFeatures = features.filter((feature) => {
    const kind = (feature as unknown as InternalHttpFeature).ɵkind;
    return kind !== HttpFeatureKind.Fetch && kind !== HttpFeatureKind.Xhr;
  });

  const providers: (Provider | EnvironmentProviders)[] = [
    angularProvideHttpClient(...sanitizedFeatures),
    { provide: NATIVE_HTTP_CONFIG, useValue: appliedConfig },
  ];

  if (!resolvedConfig.requestsViaParent) {
    providers.push({ provide: HttpBackend, useExisting: NativeHttpBackend });
  }

  return makeEnvironmentProviders(providers);
}

/**
 * Backward-compatible alias for `provideHttpClient(...)`.
 */
export const provideNativeHttpClient = provideHttpClient;

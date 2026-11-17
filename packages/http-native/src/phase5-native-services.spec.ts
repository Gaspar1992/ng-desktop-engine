import { HttpClient, HttpHeaders, HttpInterceptorFn } from '@angular/common/http';
import { signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { NativeDesktopService } from '@ng-native/desktop';
import {
  createNativeHttpResource,
  NATIVE_HTTP_CONFIG,
  NativeHttpBackend,
  provideHttpClient,
  provideNativeHttpClient,
  withFetch,
  withHttpCaching,
  withInterceptors,
  withNoXsrfProtection,
  withParallelBatching,
  withXsrfConfiguration,
} from '@ng-native/http';
import { ensureNgNativeBridge, NativeHttpRequestOptions } from '@ng-native/platform';
import { NativePlatformLocation } from '@ng-native/router';
import { firstValueFrom } from 'rxjs';

describe('Phase 5 — @ng-native/router, @ng-native/http & @ng-native/desktop', () => {
  beforeEach(() => {
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();
    bridge.setSystemTheme('dark');
    bridge.setClipboard('');
  });

  it('navigates in-memory URL history stack via NativePlatformLocation without window.location', () => {
    const loc = new NativePlatformLocation();
    expect(loc.pathname).toBe('/');

    let popEvents = 0;
    const unlisten = loc.onPopState(() => {
      popEvents += 1;
    });

    loc.pushState({ tab: 'settings' }, 'Settings', '/settings?mode=advanced#audio');
    expect(loc.pathname).toBe('/settings');
    expect(loc.search).toBe('?mode=advanced');
    expect(loc.hash).toBe('#audio');
    expect(loc.getState()).toEqual({ tab: 'settings' });

    loc.pushState({ tab: 'about' }, 'About', '/about');
    expect(loc.pathname).toBe('/about');

    loc.back();
    expect(loc.pathname).toBe('/settings');
    expect(popEvents).toBe(1);

    loc.forward();
    expect(loc.pathname).toBe('/about');
    expect(popEvents).toBe(2);

    unlisten();
  });

  it('executes Angular HttpClient requests with functional interceptors over NativeHttpBackend', async () => {
    const bridge = ensureNgNativeBridge();
    bridge.registerHttpEndpoint(
      'GET',
      'https://api.ng-native.dev/projects?platform=linux',
      200,
      JSON.stringify({ project: 'ng-linux-counter', fps: 60 }),
    );

    let interceptedUrl = '';
    const authInterceptor: HttpInterceptorFn = (req, next) => {
      interceptedUrl = req.urlWithParams;
      return next(req);
    };

    TestBed.configureTestingModule({
      providers: [NativeHttpBackend, provideNativeHttpClient(withInterceptors([authInterceptor]))],
    });
    const http = TestBed.inject(HttpClient);

    const data = await firstValueFrom(
      http.get<{ project: string; fps: number }>(
        'https://api.ng-native.dev/projects?platform=linux',
      ),
    );
    expect(interceptedUrl).toBe('https://api.ng-native.dev/projects?platform=linux');
    expect(data).toEqual({ project: 'ng-linux-counter', fps: 60 });
  });

  it('synchronizes NativeDesktopService theme Signal, clipboard, notifications, and file dialogs', () => {
    const bridge = ensureNgNativeBridge();
    const desktop = new NativeDesktopService();

    expect(desktop.theme()).toBe('dark');
    expect(desktop.toggleTheme()).toBe('light');
    expect(desktop.theme()).toBe('light');
    expect(bridge.getSystemTheme()).toBe('light');

    desktop.setClipboardText('Copied from Angular 21');
    expect(desktop.getClipboardText()).toBe('Copied from Angular 21');

    desktop.notify('Export Ready', 'Saved package successfully');
    expect(bridge.__drainQueue()).toContain('Export Ready: Saved package successfully');

    expect(desktop.openFileDialog({ defaultPath: '/home/dev/config.json' })).toBe(
      '/home/dev/config.json',
    );
    expect(desktop.saveFileDialog({ suggestedName: '/home/dev/bundle.ngb' })).toBe(
      '/home/dev/bundle.ngb',
    );
  });

  it('translates drop-in provideHttpClient features into NativeHttpConfig and forwards callKind & headers', async () => {
    const bridge = ensureNgNativeBridge();
    const capturedCalls: Array<{
      url: string;
      options?: NativeHttpRequestOptions;
    }> = [];
    const originalHttp = bridge.httpRequest.bind(bridge);

    bridge.registerHttpEndpoint(
      'GET',
      'https://pokeapi.co/api/v2/pokemon/25',
      200,
      JSON.stringify({ id: 25, name: 'pikachu' }),
    );
    bridge.registerHttpEndpoint(
      'POST',
      'https://beta.pokeapi.co/graphql/v1beta',
      200,
      JSON.stringify({ data: { pokemon: { name: 'charizard' } } }),
    );
    bridge.registerHttpEndpoint(
      'GET',
      'https://cdn.ng-native.dev/sprites/25.png',
      200,
      'PNG_BINARY_STUB',
    );
    bridge.registerHttpEndpoint(
      'GET',
      'https://cdn.ng-native.dev/notes.txt',
      200,
      'Bulbasaur notes',
    );

    bridge.httpRequest = (method, url, body, options) => {
      capturedCalls.push({ url, options });
      return originalHttp(method, url, body, options);
    };

    try {
      const traceInterceptor: HttpInterceptorFn = (req, next) =>
        next(req.clone({ setHeaders: { 'X-Client-Trace': 'pokedex-v2' } }));

      TestBed.configureTestingModule({
        providers: [
          provideHttpClient(
            withFetch(),
            withInterceptors([traceInterceptor]),
            withXsrfConfiguration({ cookieName: 'MY-XSRF', headerName: 'X-MY-XSRF' }),
            withHttpCaching(true),
            withParallelBatching(16),
          ),
        ],
      });

      const resolvedConfig = TestBed.inject(NATIVE_HTTP_CONFIG);
      expect(provideNativeHttpClient).toBe(provideHttpClient);
      expect(resolvedConfig.useFetchTransport).toBe(true);
      expect(resolvedConfig.xsrfEnabled).toBe(true);
      expect(resolvedConfig.xsrfCookieName).toBe('MY-XSRF');
      expect(resolvedConfig.xsrfHeaderName).toBe('X-MY-XSRF');
      expect(resolvedConfig.cacheEnabled).toBe(true);
      expect(resolvedConfig.maxParallelConnections).toBe(16);
      expect(bridge.getHttpConfig().xsrfHeaderName).toBe('X-MY-XSRF');

      const http = TestBed.inject(HttpClient);
      const poke = await firstValueFrom(
        http.get<{ id: number; name: string }>('https://pokeapi.co/api/v2/pokemon/25'),
      );
      expect(poke.name).toBe('pikachu');

      const gql = await firstValueFrom(
        http.post<{ data: { pokemon: { name: string } } }>(
          'https://beta.pokeapi.co/graphql/v1beta',
          { query: '{ pokemon(id: 6) { name } }' },
        ),
      );
      expect(gql.data.pokemon.name).toBe('charizard');

      const asset = await firstValueFrom(
        http.get('https://cdn.ng-native.dev/sprites/25.png', {
          responseType: 'blob',
        }),
      );
      expect(asset).toBeInstanceOf(Blob);
      expect(await asset.text()).toBe('PNG_BINARY_STUB');

      const txt = await firstValueFrom(
        http.get('https://cdn.ng-native.dev/notes.txt', {
          headers: new HttpHeaders({ 'X-Custom': 'yes' }),
          responseType: 'text',
        }),
      );
      expect(txt).toBe('Bulbasaur notes');

      expect(capturedCalls.map((c) => c.options?.callKind)).toEqual([
        'json',
        'graphql',
        'asset',
        'text',
      ]);
      expect(capturedCalls[0].options?.headers?.['X-Client-Trace']).toBe('pokedex-v2');
      expect(capturedCalls[3].options?.headers?.['X-Custom']).toBe('yes');

      TestBed.resetTestingModule();
      TestBed.configureTestingModule({
        providers: [provideHttpClient(withNoXsrfProtection())],
      });
      expect(TestBed.inject(NATIVE_HTTP_CONFIG).xsrfEnabled).toBe(false);
    } finally {
      bridge.httpRequest = originalHttp;
    }
  });

  it('manages reactive signal HTTP resource lifecycle via createNativeHttpResource', () => {
    const bridge = ensureNgNativeBridge();
    bridge.registerHttpEndpoint(
      'GET',
      'https://pokeapi.co/api/v2/pokemon/1',
      200,
      JSON.stringify({ id: 1, name: 'bulbasaur' }),
    );
    bridge.registerHttpEndpoint(
      'GET',
      'https://pokeapi.co/api/v2/pokemon/25',
      200,
      JSON.stringify({ id: 25, name: 'pikachu' }),
    );

    const pokeId = signal<number | undefined>(undefined);
    const resource = createNativeHttpResource<string>(
      () => (pokeId() ? `https://pokeapi.co/api/v2/pokemon/${pokeId()}` : undefined),
      {
        map: (raw) => (raw as { name: string }).name.toUpperCase(),
      },
    );

    expect(resource.status()).toBe('idle');
    expect(resource.hasValue()).toBe(false);
    expect(resource.isLoading()).toBe(false);

    pokeId.set(1);
    expect(resource.reload()).toBe(true);
    expect(resource.status()).toBe('resolved');
    expect(resource.hasValue()).toBe(true);
    expect(resource.value()).toBe('BULBASAUR');

    pokeId.set(25);
    expect(resource.reload()).toBe(true);
    expect(resource.value()).toBe('PIKACHU');

    pokeId.set(9999);
    expect(resource.reload()).toBe(false);
    expect(resource.status()).toBe('error');
    expect(resource.error()).toMatchObject({ status: 404 });
  });
});

import { HttpClient, HttpHeaders, HttpInterceptorFn } from '@angular/common/http';
import { signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import {
  ActivatedRouteSnapshot,
  DetachedRouteHandle,
  RouteReuseStrategy,
  RouterStateSnapshot,
  TitleStrategy,
  UrlSegment,
} from '@angular/router';
import {
  NativeDesktopService,
  NativeNetworkService,
  NativeSqliteService,
  provideNativeDesktop,
} from '@ng-desktop/desktop';
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
} from '@ng-desktop/http';
import {
  ensureNgNativeBridge,
  NativeHttpRequestOptions,
  NativeTitleService,
} from '@ng-desktop/platform';
import {
  NativePlatformLocation,
  NativeStackRouteReuseStrategy,
  NativeTitleStrategy,
  provideNativeRouter,
  withNativeRouteReuse,
  withNativeTitleStrategy,
} from '@ng-desktop/router';
import { firstValueFrom } from 'rxjs';

describe('Phase 5 — @ng-desktop/router, @ng-desktop/http & @ng-desktop/desktop', () => {
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
      JSON.stringify({ project: 'sample-desktop-app', fps: 60 }),
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
    expect(data).toEqual({ project: 'sample-desktop-app', fps: 60 });
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
      'https://api.example.test/v2/items/25',
      200,
      JSON.stringify({ id: 25, name: 'widget' }),
    );
    bridge.registerHttpEndpoint(
      'POST',
      'https://api.example.test/graphql',
      200,
      JSON.stringify({ data: { item: { name: 'gadget' } } }),
    );
    bridge.registerHttpEndpoint(
      'GET',
      'https://cdn.ng-native.dev/sprites/25.png',
      200,
      'PNG_BINARY_STUB',
    );
    bridge.registerHttpEndpoint('GET', 'https://cdn.ng-native.dev/notes.txt', 200, 'Item notes');

    bridge.httpRequest = (method, url, body, options) => {
      capturedCalls.push({ url, options });
      return originalHttp(method, url, body, options);
    };

    try {
      const traceInterceptor: HttpInterceptorFn = (req, next) =>
        next(req.clone({ setHeaders: { 'X-Client-Trace': 'desktop-v2' } }));

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
      const item = await firstValueFrom(
        http.get<{ id: number; name: string }>('https://api.example.test/v2/items/25'),
      );
      expect(item.name).toBe('widget');

      const gql = await firstValueFrom(
        http.post<{ data: { item: { name: string } } }>('https://api.example.test/graphql', {
          query: '{ item(id: 6) { name } }',
        }),
      );
      expect(gql.data.item.name).toBe('gadget');

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
      expect(txt).toBe('Item notes');

      expect(capturedCalls.map((c) => c.options?.callKind)).toEqual([
        'json',
        'graphql',
        'asset',
        'text',
      ]);
      expect(capturedCalls[0].options?.headers?.['X-Client-Trace']).toBe('desktop-v2');
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
      'https://api.example.test/v2/items/1',
      200,
      JSON.stringify({ id: 1, name: 'alpha' }),
    );
    bridge.registerHttpEndpoint(
      'GET',
      'https://api.example.test/v2/items/25',
      200,
      JSON.stringify({ id: 25, name: 'widget' }),
    );

    const itemId = signal<number | undefined>(undefined);
    const resource = createNativeHttpResource<string>(
      () => (itemId() ? `https://api.example.test/v2/items/${itemId()}` : undefined),
      {
        map: (raw) => (raw as { name: string }).name.toUpperCase(),
      },
    );

    expect(resource.status()).toBe('idle');
    expect(resource.hasValue()).toBe(false);
    expect(resource.isLoading()).toBe(false);

    itemId.set(1);
    expect(resource.reload()).toBe(true);
    expect(resource.status()).toBe('resolved');
    expect(resource.hasValue()).toBe(true);
    expect(resource.value()).toBe('ALPHA');

    itemId.set(25);
    expect(resource.reload()).toBe(true);
    expect(resource.value()).toBe('WIDGET');

    itemId.set(9999);
    expect(resource.reload()).toBe(false);
    expect(resource.status()).toBe('error');
    expect(resource.error()).toMatchObject({ status: 404 });

    TestBed.runInInjectionContext(() => {
      const reactiveId = signal<number | undefined>(1);
      const reactiveResource = createNativeHttpResource<string>(
        () => (reactiveId() ? `https://api.example.test/v2/items/${reactiveId()}` : undefined),
        { map: (raw) => (raw as { name: string }).name.toUpperCase() },
      );
      expect(reactiveResource.value()).toBe('ALPHA');
      reactiveId.set(25);
      TestBed.tick();
      expect(reactiveResource.value()).toBe('WIDGET');
    });
  });

  it('provides NativeSqliteService liveQuery, NativeNetworkService, TitleStrategy, and RouteReuseStrategy', () => {
    TestBed.configureTestingModule({
      providers: [
        provideNativeDesktop(),
        provideNativeRouter([]),
        withNativeTitleStrategy('Workspace'),
        withNativeRouteReuse(),
      ],
    });

    const sqlite = TestBed.inject(NativeSqliteService);
    sqlite.execute('CREATE TABLE IF NOT EXISTS notes (id TEXT PRIMARY KEY, body TEXT)');
    const liveNotes = sqlite.liveQuery<{ id: string; body: string }>(
      'SELECT * FROM notes ORDER BY id ASC',
      (r) => ({ id: r['id'] ?? '', body: r['body'] ?? '' }),
    );
    expect(liveNotes.count()).toBe(0);

    sqlite.execute("INSERT INTO notes (id, body) VALUES ('25', 'Note B')");
    sqlite.execute("INSERT INTO notes (id, body) VALUES ('1', 'Note A')");
    expect(liveNotes.count()).toBe(2);
    expect(liveNotes.rows().map((r) => r.id)).toEqual(['1', '25']);

    sqlite.execute("DELETE FROM notes WHERE id = '1'");
    expect(liveNotes.rows()).toEqual([{ id: '25', body: 'Note B' }]);

    const network = TestBed.inject(NativeNetworkService);
    expect(network.connectivity()).toBe('full');
    network.setOnlineStatus(true, true);
    expect(network.connectivity()).toBe('limited');
    network.setOnlineStatus(false);
    expect(network.connectivity()).toBe('offline');
    network.setOnlineStatus(true, false);

    const titleStrategy = TestBed.inject(TitleStrategy) as NativeTitleStrategy;
    const nativeTitle = TestBed.inject(NativeTitleService);
    vi.spyOn(titleStrategy, 'buildTitle').mockReturnValue('Overview');
    titleStrategy.updateTitle({} as RouterStateSnapshot);
    expect(nativeTitle.getTitle()).toBe('Workspace — Overview');

    const reuse = TestBed.inject(RouteReuseStrategy) as NativeStackRouteReuseStrategy;
    const mockSnapshot = {
      routeConfig: { path: 'detail/25' },
      data: {},
      url: [new UrlSegment('detail', {}), new UrlSegment('25', {})],
      get pathFromRoot() {
        return [this];
      },
    } as unknown as ActivatedRouteSnapshot;

    expect(reuse.shouldDetach(mockSnapshot)).toBe(true);
    let destroyedAttached = false;
    const fakeHandle = {
      componentRef: {
        destroy: () => {
          destroyedAttached = true;
        },
      },
    } as unknown as DetachedRouteHandle;
    reuse.store(mockSnapshot, fakeHandle);
    expect(reuse.shouldAttach(mockSnapshot)).toBe(true);
    expect(reuse.cachedRoutes()).toEqual(['/detail/25']);
    expect(reuse.retrieve(mockSnapshot)).toBe(fakeHandle);
    expect(reuse.shouldAttach(mockSnapshot)).toBe(false);
    expect(reuse.cachedRoutes()).toEqual([]);
    reuse.clearCache();
    expect(reuse.cachedRoutes()).toEqual([]);
    expect(destroyedAttached).toBe(false);
  });

  it('supports createNativeHttpResource.reloadAsync with stale request cancellation', async () => {
    const bridge = ensureNgNativeBridge();
    let resolveFirst!: (val: {
      ok: boolean;
      status: number;
      statusText: string;
      headers: Record<string, string>;
      body: string;
      fromCache?: boolean;
    }) => void;
    let resolveSecond!: typeof resolveFirst;
    let callIdx = 0;

    bridge.httpRequestAsync = () =>
      new Promise((resolve) => {
        callIdx += 1;
        if (callIdx === 1) {
          resolveFirst = resolve;
        } else {
          resolveSecond = resolve;
        }
      });

    bridge.registerHttpEndpoint('GET', 'https://api.example.test/initial', 200, '{"step":"sync"}');
    let currentUrl = 'https://api.example.test/initial';
    const res = createNativeHttpResource<{ step: string }>(() => currentUrl);
    expect(res.value()).toEqual({ step: 'sync' });

    currentUrl = 'https://api.example.test/slow';
    const p1 = res.reloadAsync();
    currentUrl = 'https://api.example.test/fast';
    const p2 = res.reloadAsync();

    resolveSecond({
      ok: true,
      status: 200,
      statusText: 'OK',
      headers: {},
      body: '{"step":"fast"}',
    });
    expect(await p2).toBe(true);
    expect(res.value()).toEqual({ step: 'fast' });

    resolveFirst({
      ok: true,
      status: 200,
      statusText: 'OK',
      headers: {},
      body: '{"step":"stale"}',
    });
    expect(await p1).toBe(false);
    expect(res.value()).toEqual({ step: 'fast' });
    delete bridge.httpRequestAsync;
  });

  it('destroys detached route componentRefs on eviction, replacement, and clearCache', () => {
    const strategy = new NativeStackRouteReuseStrategy();
    const makeSnap = (segment: string) =>
      ({
        routeConfig: { path: segment },
        data: {},
        url: [new UrlSegment(segment, {})],
        get pathFromRoot() {
          return [this];
        },
      }) as unknown as ActivatedRouteSnapshot;

    const destroyed: string[] = [];
    for (let i = 0; i < 16; i += 1) {
      const label = `route-${i}`;
      strategy.store(makeSnap(label), {
        componentRef: { destroy: () => destroyed.push(label) },
      } as unknown as DetachedRouteHandle);
    }
    expect(destroyed).toEqual([]);

    // Adding a 17th entry should evict and destroy the oldest ('route-0')
    strategy.store(makeSnap('route-16'), {
      componentRef: { destroy: () => destroyed.push('route-16') },
    } as unknown as DetachedRouteHandle);
    expect(destroyed).toEqual(['route-0']);

    // Replacing an existing key should destroy its previous handle
    strategy.store(makeSnap('route-1'), {
      componentRef: { destroy: () => destroyed.push('route-1-new') },
    } as unknown as DetachedRouteHandle);
    expect(destroyed).toEqual(['route-0', 'route-1']);

    strategy.clearCache();
    expect(destroyed).toContain('route-16');
    expect(destroyed).toContain('route-1-new');
    expect(strategy.cachedRoutes()).toEqual([]);
  });

  it('detaches NativeSqliteService.liveQuery refresher when destroy() is called', () => {
    const sqlite = TestBed.inject(NativeSqliteService);
    sqlite.execute('CREATE TABLE IF NOT EXISTS live_cleanup (id TEXT PRIMARY KEY)');
    sqlite.execute('DELETE FROM live_cleanup');

    const ref = sqlite.liveQuery<{ id: string }>('SELECT * FROM live_cleanup');
    expect(ref.count()).toBe(0);
    sqlite.execute("INSERT INTO live_cleanup (id) VALUES ('a')");
    expect(ref.count()).toBe(1);

    ref.destroy();

    sqlite.execute("INSERT INTO live_cleanup (id) VALUES ('b')");
    // Frozen at count 1 after destroy()
    expect(ref.count()).toBe(1);
  });
});

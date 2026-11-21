import {
  createError,
  createNativeServerRouter,
  defineNativeEventHandler,
  getQuery,
  getRouterParam,
  type NativeServerHostBridge,
  readBody,
  registerNativeServerApp,
  setResponseHeader,
  setResponseStatus,
} from '@ng-desktop/server';

describe('@ng-desktop/server — AnalogJS-style Native Server Router', () => {
  it('routes static, parameterized, catch-all paths with precedence, queries, body, and custom status/headers', async () => {
    const router = createNativeServerRouter([
      {
        method: 'GET',
        pattern: '/api/pokemon/*slug',
        handler: defineNativeEventHandler((event) => ({
          kind: 'catchall',
          slug: getRouterParam(event, 'slug'),
        })),
      },
      {
        method: 'GET',
        pattern: '/api/pokemon/:id',
        handler: defineNativeEventHandler((event) => ({
          kind: 'param',
          id: getRouterParam(event, 'id'),
          query: getQuery(event),
        })),
      },
      {
        method: 'GET',
        pattern: '/api/pokemon/featured',
        handler: defineNativeEventHandler(() => ({ kind: 'static', name: 'Pikachu' })),
      },
      {
        method: 'POST',
        pattern: '/api/favorites',
        handler: defineNativeEventHandler((event) => {
          const payload = readBody<{ id: number; note: string }>(event);
          setResponseStatus(event, 201, 'Created');
          setResponseHeader(event, 'x-custom-saved', String(payload.id));
          return { saved: payload };
        }),
      },
      {
        method: 'GET',
        pattern: '/api/health',
        handler: defineNativeEventHandler(() => 'healthy'),
      },
    ]);

    const staticRes = await router.handle({ method: 'GET', url: '/api/pokemon/featured/' });
    expect(staticRes.status).toBe(200);
    expect(JSON.parse(staticRes.body)).toEqual({ kind: 'static', name: 'Pikachu' });

    const paramRes = await router.handle({
      method: 'GET',
      url: 'http://localhost/api/pokemon/25?lang=es&shiny=true',
    });
    expect(paramRes.status).toBe(200);
    expect(JSON.parse(paramRes.body)).toEqual({
      kind: 'param',
      id: '25',
      query: { lang: 'es', shiny: 'true' },
    });

    const catchAllRes = await router.handle({
      method: 'GET',
      url: '/api/pokemon/regions/kanto/starter',
    });
    expect(catchAllRes.status).toBe(200);
    expect(JSON.parse(catchAllRes.body)).toEqual({
      kind: 'catchall',
      slug: 'regions/kanto/starter',
    });

    const postRes = await router.handle({
      method: 'POST',
      url: '/api/favorites',
      body: JSON.stringify({ id: 6, note: 'Charizard' }),
    });
    expect(postRes.status).toBe(201);
    expect(postRes.headers['x-custom-saved']).toBe('6');
    expect(JSON.parse(postRes.body)).toEqual({ saved: { id: 6, note: 'Charizard' } });

    const textRes = await router.handle({ method: 'GET', url: '/api/health' });
    expect(textRes.status).toBe(200);
    expect(textRes.headers['content-type']).toContain('text/plain');
    expect(textRes.body).toBe('healthy');
  });

  it('returns 404, 405, HttpError codes, and 500 on unexpected errors', async () => {
    const router = createNativeServerRouter([
      {
        method: 'GET',
        pattern: '/api/items/:id',
        handler: defineNativeEventHandler((event) => {
          const id = getRouterParam(event, 'id');
          if (id === 'missing') {
            throw createError({ statusCode: 404, message: 'Item missing', data: { id } });
          }
          if (id === 'boom') {
            throw new Error('Database unreachable');
          }
          return { id };
        }),
      },
    ]);

    const notFound = await router.handle({ method: 'GET', url: '/api/unknown' });
    expect(notFound.status).toBe(404);

    const wrongMethod = await router.handle({ method: 'DELETE', url: '/api/items/12' });
    expect(wrongMethod.status).toBe(405);

    const httpErr = await router.handle({ method: 'GET', url: '/api/items/missing' });
    expect(httpErr.status).toBe(404);
    expect(JSON.parse(httpErr.body)).toEqual({
      error: 'Item missing',
      statusCode: 404,
      data: { id: 'missing' },
    });

    const crashErr = await router.handle({ method: 'GET', url: '/api/items/boom' });
    expect(crashErr.status).toBe(500);
    expect(JSON.parse(crashErr.body)).toEqual({
      error: 'Database unreachable',
      statusCode: 500,
    });
  });

  it('registers router globally and delegates db/fs/compute/http calls to __NG_NATIVE_SERVER_HOST__', async () => {
    const executedSql: Array<{ sql: string; params?: readonly unknown[] }> = [];
    const queriedSql: Array<{ sql: string; params?: readonly unknown[] }> = [];
    const mockHost: NativeServerHostBridge = {
      sqliteExecute: (sql, params) => {
        executedSql.push({ sql, params });
        return true;
      },
      sqliteQuery: (sql, params) => {
        queriedSql.push({ sql, params });
        return [{ id: 1, name: 'bulbasaur' }];
      },
      fsResolvePath: (kind, rel) => `/xdg/${kind}/${rel}`,
      fsReadText: () => '{"ok":true}',
      fsWriteText: () => true,
      fsWatchFile: () => true,
      fsPollChangedFiles: () => [],
      computeFuzzyFilter: (items, query) =>
        items
          .map((value, index) => ({ index, score: 100, value }))
          .filter((entry) => entry.value.includes(query)),
      computeHash: (input) => `h_${input.length}`,
      httpRequest: async (_method, url) => ({
        status: 200,
        statusText: 'OK',
        body: JSON.stringify({ upstream: url }),
        headers: { 'content-type': 'application/json' },
      }),
    };

    const g = globalThis as {
      __NG_NATIVE_SERVER_HOST__?: NativeServerHostBridge;
      __NG_NATIVE_SERVER_ROUTER__?: unknown;
    };
    const prevHost = g.__NG_NATIVE_SERVER_HOST__;
    const prevRouter = g.__NG_NATIVE_SERVER_ROUTER__;
    g.__NG_NATIVE_SERVER_HOST__ = mockHost;

    try {
      const router = registerNativeServerApp([
        {
          method: 'GET',
          pattern: '/api/bootstrap',
          handler: defineNativeEventHandler(async (event) => {
            event.context.db.execute('INSERT INTO catalog VALUES (?, ?);', [1, 'bulbasaur']);
            const rows = event.context.db.query('SELECT * FROM catalog WHERE id = ?;', [1]);
            const filtered = event.context.compute.fuzzyFilter(['pikachu', 'bulbasaur'], 'bulb');
            const cachePath = event.context.fs.resolvePath('cache', 'catalog.json');
            const upstream = await event.context.http.get('https://pokeapi.co/api/v2/pokemon/1');
            return {
              rows,
              filtered,
              cachePath,
              hash: event.context.compute.hash('abc'),
              upstream: JSON.parse(upstream.body),
            };
          }),
        },
      ]);

      expect(g.__NG_NATIVE_SERVER_ROUTER__).toBe(router);
      const res = await router.handle({ method: 'GET', url: '/api/bootstrap' });
      expect(res.status).toBe(200);
      expect(executedSql).toEqual([
        { sql: 'INSERT INTO catalog VALUES (?, ?);', params: [1, 'bulbasaur'] },
      ]);
      expect(queriedSql).toEqual([{ sql: 'SELECT * FROM catalog WHERE id = ?;', params: [1] }]);
      expect(JSON.parse(res.body)).toEqual({
        rows: [{ id: 1, name: 'bulbasaur' }],
        filtered: [{ index: 1, score: 100, value: 'bulbasaur' }],
        cachePath: '/xdg/cache/catalog.json',
        hash: 'h_3',
        upstream: { upstream: 'https://pokeapi.co/api/v2/pokemon/1' },
      });
    } finally {
      g.__NG_NATIVE_SERVER_HOST__ = prevHost;
      g.__NG_NATIVE_SERVER_ROUTER__ = prevRouter as typeof g.__NG_NATIVE_SERVER_ROUTER__;
    }
  });
});

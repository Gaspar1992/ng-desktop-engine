import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import vm from 'node:vm';
import { compileServerBundle } from '@ng-desktop/builders';
import type { NativeServerHostBridge } from '@ng-desktop/server';

interface InMemoryTables {
  catalogCache: Map<string, string>;
  pokemonCache: Map<number, string>;
  favorites: Map<number, { id: number; name: string; note: string }>;
  fieldNotes: Map<number, string>;
}

function createMockHost(tables: InMemoryTables, upstreamHits: string[]): NativeServerHostBridge {
  return {
    sqliteExecute: (sql, params = []) => {
      if (sql.includes('INSERT OR REPLACE INTO catalog_cache')) {
        tables.catalogCache.set(String(params[0]), String(params[1]));
      } else if (sql.includes('INSERT OR REPLACE INTO pokemon_cache')) {
        tables.pokemonCache.set(Number(params[0]), String(params[1]));
      } else if (sql.includes('INSERT OR REPLACE INTO favorites')) {
        const id = Number(params[0]);
        tables.favorites.set(id, { id, name: String(params[1]), note: String(params[2] ?? '') });
      } else if (sql.includes('DELETE FROM favorites')) {
        tables.favorites.delete(Number(params[0]));
      } else if (sql.includes('INSERT OR REPLACE INTO field_notes')) {
        tables.fieldNotes.set(Number(params[0]), String(params[1] ?? ''));
      }
      return true;
    },
    sqliteQuery: (sql, params = []) => {
      if (sql.includes('FROM catalog_cache')) {
        const val = tables.catalogCache.get(String(params[0]));
        return val ? [{ payload: val }] : [];
      }
      if (sql.includes('FROM pokemon_cache')) {
        const val = tables.pokemonCache.get(Number(params[0]));
        return val ? [{ payload: val }] : [];
      }
      if (sql.includes('FROM favorites WHERE id = ?')) {
        const fav = tables.favorites.get(Number(params[0]));
        return fav ? [fav] : [];
      }
      if (sql.includes('SELECT id FROM favorites')) {
        return Array.from(tables.favorites.values())
          .sort((a, b) => a.id - b.id)
          .map((row) => ({ id: row.id }));
      }
      if (sql.includes('FROM favorites')) {
        return Array.from(tables.favorites.values()).sort((a, b) => a.id - b.id);
      }
      if (sql.includes('FROM field_notes')) {
        const note = tables.fieldNotes.get(Number(params[0]));
        return note !== undefined ? [{ note }] : [];
      }
      return [];
    },
    fsResolvePath: (kind, rel) => `/tmp/${kind}/${rel}`,
    fsReadText: () => null,
    fsWriteText: () => true,
    fsWatchFile: () => true,
    fsPollChangedFiles: () => [],
    computeFuzzyFilter: (items, query, limit = 50) => {
      const needle = query.trim().toLowerCase();
      return items
        .map((value, index) => ({
          index,
          score: value.toLowerCase().includes(needle) ? 500 : 0,
          value,
        }))
        .filter((entry) => entry.score > 0)
        .slice(0, limit);
    },
    computeHash: (input) => `hash_${input.length}`,
    httpRequest: async (_method, url) => {
      upstreamHits.push(url);
      if (url.includes('/pokemon?limit=')) {
        return {
          status: 200,
          statusText: 'OK',
          headers: {},
          body: JSON.stringify({
            results: [
              { name: 'bulbasaur', url: 'https://pokeapi.co/api/v2/pokemon/1/' },
              { name: 'charmander', url: 'https://pokeapi.co/api/v2/pokemon/4/' },
              { name: 'pikachu', url: 'https://pokeapi.co/api/v2/pokemon/25/' },
            ],
          }),
        };
      }
      if (url.endsWith('/pokemon/25')) {
        return {
          status: 200,
          statusText: 'OK',
          headers: {},
          body: JSON.stringify({
            id: 25,
            name: 'pikachu',
            height: 4,
            weight: 60,
            base_experience: 112,
            types: [{ slot: 1, type: { name: 'electric' } }],
            abilities: [{ is_hidden: false, ability: { name: 'static' } }],
            stats: [{ base_stat: 35, stat: { name: 'hp' } }],
            sprites: {
              other: {
                'official-artwork': { front_default: 'https://cdn.example.test/25.png' },
              },
            },
          }),
        };
      }
      if (url.endsWith('/pokemon-species/25')) {
        return {
          status: 200,
          statusText: 'OK',
          headers: {},
          body: JSON.stringify({
            genera: [{ genus: 'Pokémon Ratón', language: { name: 'es' } }],
            flavor_text_entries: [
              { flavor_text: 'Almacena electricidad\nen sus mejillas.', language: { name: 'es' } },
            ],
          }),
        };
      }
      return { status: 404, statusText: 'Not Found', headers: {}, body: '{}' };
    },
  };
}

describe('Pokédex @ng-desktop/server routes', () => {
  it('compiles all routes and serves catalog, detail cache, favorites, and field notes', async () => {
    const outDir = mkdtempSync(join(tmpdir(), 'ng-pokedex-server-spec-'));
    try {
      const serverBundlePath = compileServerBundle({
        workspaceRoot: process.cwd(),
        sourceRootAbs: resolve(process.cwd(), 'examples/counter-app/src'),
        outDir,
        projectName: 'ng-linux-counter',
      });
      expect(serverBundlePath).toBe(join(outDir, 'server.bundle.js'));

      const code = readFileSync(serverBundlePath!, 'utf8');
      const tables: InMemoryTables = {
        catalogCache: new Map(),
        pokemonCache: new Map(),
        favorites: new Map(),
        fieldNotes: new Map(),
      };
      const upstreamHits: string[] = [];

      const sandbox: Record<string, unknown> = {
        console,
        URL,
        URLSearchParams,
        __NG_NATIVE_SERVER_HOST__: createMockHost(tables, upstreamHits),
      };
      sandbox['globalThis'] = sandbox;
      vm.runInNewContext(code, sandbox);

      const router = sandbox['__NG_NATIVE_SERVER_ROUTER__'] as {
        handle: (req: { method: string; url: string; body?: string }) => Promise<{
          status: number;
          body: string;
        }>;
      };

      const catalogRes = await router.handle({
        method: 'GET',
        url: '/api/pokemon?offset=0&limit=12',
      });
      expect(catalogRes.status).toBe(200);
      const catalogJson = JSON.parse(catalogRes.body) as {
        total: number;
        items: Array<{ id: number; displayName: string }>;
      };
      expect(catalogJson.total).toBe(3);
      expect(catalogJson.items.map((i) => i.displayName)).toEqual([
        'Bulbasaur',
        'Charmander',
        'Pikachu',
      ]);

      const searchRes = await router.handle({ method: 'GET', url: '/api/pokemon?q=pika' });
      expect(searchRes.status).toBe(200);
      expect(
        (JSON.parse(searchRes.body) as { items: Array<{ id: number }> }).items.map((i) => i.id),
      ).toEqual([25]);

      const badIdRes = await router.handle({ method: 'GET', url: '/api/pokemon/9999' });
      expect(badIdRes.status).toBe(400);

      const detailRes = await router.handle({ method: 'GET', url: '/api/pokemon/25' });
      expect(detailRes.status).toBe(200);
      const detailJson = JSON.parse(detailRes.body) as {
        cached: boolean;
        pokemon: {
          displayName: string;
          genus: string;
          description: string;
          types: Array<{ label: string; badgeClass: string }>;
        };
      };
      expect(detailJson.cached).toBe(false);
      expect(detailJson.pokemon.displayName).toBe('Pikachu');
      expect(detailJson.pokemon.genus).toBe('Pokémon Ratón');
      expect(detailJson.pokemon.description).toBe('Almacena electricidad en sus mejillas.');
      expect(detailJson.pokemon.types).toEqual([
        { label: 'ELÉCTRICO', badgeClass: 'badge-electric' },
      ]);

      const hitsBeforeCache = upstreamHits.length;
      const detailCachedRes = await router.handle({ method: 'GET', url: '/api/pokemon/25' });
      expect((JSON.parse(detailCachedRes.body) as { cached: boolean }).cached).toBe(true);
      expect(upstreamHits.length).toBe(hitsBeforeCache);

      const postFav = await router.handle({
        method: 'POST',
        url: '/api/favorites',
        body: JSON.stringify({ id: 25, name: 'Pikachu', note: 'Favorito' }),
      });
      expect(postFav.status).toBe(201);

      const putNote = await router.handle({
        method: 'PUT',
        url: '/api/notes/25',
        body: JSON.stringify({ note: 'Nota actualizada' }),
      });
      expect(putNote.status).toBe(200);

      const afterDetail = await router.handle({ method: 'GET', url: '/api/pokemon/25' });
      expect(JSON.parse(afterDetail.body) as { isFavorite: boolean; note: string }).toMatchObject({
        isFavorite: true,
        note: 'Nota actualizada',
      });

      const favList = await router.handle({
        method: 'GET',
        url: '/api/pokemon?favoritesOnly=true',
      });
      expect(
        (JSON.parse(favList.body) as { items: Array<{ id: number }> }).items.map((i) => i.id),
      ).toEqual([25]);

      const delFav = await router.handle({ method: 'DELETE', url: '/api/favorites/25' });
      expect(delFav.status).toBe(200);
      const favGet = await router.handle({ method: 'GET', url: '/api/favorites' });
      expect((JSON.parse(favGet.body) as { items: unknown[] }).items).toEqual([]);
    } finally {
      rmSync(outDir, { recursive: true, force: true });
    }
  });
});

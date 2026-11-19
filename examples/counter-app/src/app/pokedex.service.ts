import { HttpClient } from '@angular/common/http';
import { inject, Injectable, signal } from '@angular/core';
import {
  NativeComputeService,
  NativeFileSystemService,
  type NativeLiveQueryRef,
  NativeNetworkService,
  NativeSqliteService,
} from '@ng-native/desktop';
import { createNativeHttpResource } from '@ng-native/http';
import { ensureNgNativeBridge, NativeTitleService } from '@ng-native/platform';
import { Observable } from 'rxjs';

export interface PokemonSummary {
  id: number;
  name: string;
  height?: number;
  weight?: number;
  base_experience?: number;
  types?: Array<{ slot?: number; type?: { name: string } }>;
  stats?: Array<{ base_stat: number; stat?: { name: string } }>;
}

export interface CatalogEntry {
  id: number;
  name: string;
}

export interface FavoritePokemonRow {
  id: number;
  name: string;
  note: string;
}

@Injectable({ providedIn: 'root' })
export class PokedexService {
  private readonly http = inject(HttpClient, { optional: true });
  private readonly sqlite = inject(NativeSqliteService);
  private readonly network = inject(NativeNetworkService);
  private readonly compute = inject(NativeComputeService);
  private readonly fs = inject(NativeFileSystemService);
  private readonly titleService = inject(NativeTitleService);

  readonly selectedId = signal(1);
  readonly searchQuery = signal('');
  readonly loadedCount = signal(12);
  readonly isOnline = this.network.isOnline;

  readonly selectedPokemonResource = createNativeHttpResource<PokemonSummary>(
    () => `https://pokeapi.co/api/v2/pokemon/${this.selectedId()}`,
  );

  readonly favorites: NativeLiveQueryRef<FavoritePokemonRow>;

  constructor() {
    this.sqlite.execute(
      'CREATE TABLE IF NOT EXISTS favorites (id TEXT PRIMARY KEY, name TEXT, note TEXT)',
    );
    this.favorites = this.sqlite.liveQuery<FavoritePokemonRow>(
      'SELECT * FROM favorites ORDER BY id ASC',
      (r) => ({
        id: Number(r['id']),
        name: r['name'] ?? '',
        note: r['note'] ?? '',
      }),
    );
  }

  fetchPokemon(idOrName: string | number): Observable<PokemonSummary> | null {
    if (!this.http) {
      return null;
    }
    return this.http.get<PokemonSummary>(`https://pokeapi.co/api/v2/pokemon/${idOrName}`);
  }

  selectPokemon(id: number, name?: string): void {
    this.selectedId.set(id);
    this.selectedPokemonResource.reload();
    if (name) {
      const padded = String(id).padStart(4, '0');
      this.titleService.setTitle(`Pokédex Nacional — #${padded} ${name}`);
    }
  }

  saveFavorite(id: number, name: string, note = ''): void {
    const safeName = name.replaceAll("'", "''");
    const safeNote = note.replaceAll("'", "''");
    this.sqlite.execute(
      `INSERT OR REPLACE INTO favorites (id, name, note) VALUES ('${id}', '${safeName}', '${safeNote}')`,
    );
  }

  removeFavorite(id: number): void {
    this.sqlite.execute(`DELETE FROM favorites WHERE id = '${id}'`);
  }

  /**
   * Executes a parallel batch request via the OS-native HTTP bridge (`httpBatchRequest`)
   * for the provided Pokémon IDs and returns parsed summaries.
   */
  fetchBatch(ids: readonly number[]): PokemonSummary[] {
    if (ids.length === 0) {
      return [];
    }
    const bridge = ensureNgNativeBridge();
    const requests = ids.map((id) => ({
      method: 'GET',
      url: `https://pokeapi.co/api/v2/pokemon/${id}`,
      options: { callKind: 'json' as const },
    }));
    const responses = bridge.httpBatchRequest
      ? bridge.httpBatchRequest(requests)
      : requests.map((req) => bridge.httpRequest(req.method, req.url, undefined, req.options));

    const results: PokemonSummary[] = [];
    for (const res of responses) {
      if (res.status >= 200 && res.status < 300 && res.body) {
        try {
          results.push(JSON.parse(res.body) as PokemonSummary);
        } catch {
          // Ignore malformed entry inside batch
        }
      }
    }
    return results;
  }

  filterCatalog(entries: readonly CatalogEntry[], query: string, limit = 18): CatalogEntry[] {
    const clean = query.trim().toLowerCase();
    if (!clean) {
      return entries.slice(0, limit);
    }
    return this.compute
      .fuzzyFilter(entries, (entry) => `${entry.id} ${entry.name}`, clean, limit)
      .map((scored) => scored.item);
  }

  exportFavoritesToDisk(relPath = 'favorites.json'): string {
    const targetPath = this.fs.resolveXdgPath('data', relPath);
    this.fs.writeJson(targetPath, this.favorites.rows());
    return targetPath;
  }

  importFavoritesFromFile(filePath: string): number {
    const rows = this.fs.readJson<FavoritePokemonRow[]>(filePath, []);
    let imported = 0;
    for (const row of rows) {
      if (typeof row.id === 'number' && typeof row.name === 'string') {
        this.saveFavorite(row.id, row.name, row.note ?? '');
        imported += 1;
      }
    }
    return imported;
  }
}

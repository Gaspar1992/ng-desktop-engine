import { HttpClient } from '@angular/common/http';
import { inject, Injectable, signal } from '@angular/core';
import { createNativeHttpResource } from '@ng-native/http';
import { ensureNgNativeBridge } from '@ng-native/platform';
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

@Injectable({ providedIn: 'root' })
export class PokedexService {
  private readonly http = inject(HttpClient, { optional: true });

  readonly selectedId = signal(1);
  readonly searchQuery = signal('');
  readonly loadedCount = signal(12);

  readonly selectedPokemonResource = createNativeHttpResource<PokemonSummary>(
    () => `https://pokeapi.co/api/v2/pokemon/${this.selectedId()}`,
  );

  fetchPokemon(idOrName: string | number): Observable<PokemonSummary> | null {
    if (!this.http) {
      return null;
    }
    return this.http.get<PokemonSummary>(`https://pokeapi.co/api/v2/pokemon/${idOrName}`);
  }

  selectPokemon(id: number): void {
    this.selectedId.set(id);
    this.selectedPokemonResource.reload();
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
    return entries
      .filter(
        (entry) => entry.name.toLowerCase().includes(clean) || String(entry.id).startsWith(clean),
      )
      .slice(0, limit);
  }
}

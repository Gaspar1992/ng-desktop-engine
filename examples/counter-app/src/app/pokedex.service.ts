import { HttpClient } from '@angular/common/http';
import { inject, Injectable, signal } from '@angular/core';
import {
  NativeComputeService,
  NativeFileSystemService,
  NativeNetworkService,
} from '@ng-desktop/desktop';
import { NativeTitleService } from '@ng-desktop/platform';
import { firstValueFrom } from 'rxjs';
import type {
  CatalogListItem,
  FavoritePokemonRow,
  PokemonDetailRecord,
} from '../server/pokedex-store';

export type PokedexCatalogCard = CatalogListItem & { isFavorite?: boolean };

export interface PokedexListResponse {
  total: number;
  offset: number;
  limit: number;
  hasMore: boolean;
  items: PokedexCatalogCard[];
  favoriteIds: number[];
}

export interface PokedexDetailResponse {
  pokemon: PokemonDetailRecord;
  isFavorite: boolean;
  note: string;
  cached: boolean;
}

@Injectable({ providedIn: 'root' })
export class PokedexService {
  private readonly http = inject(HttpClient);
  private readonly network = inject(NativeNetworkService);
  private readonly compute = inject(NativeComputeService);
  private readonly fs = inject(NativeFileSystemService);
  private readonly titleService = inject(NativeTitleService);

  readonly viewMode = signal<'list' | 'detail'>('list');
  readonly catalogItems = signal<PokedexCatalogCard[]>([]);
  readonly totalCount = signal(0);
  readonly hasMore = signal(false);
  readonly loading = signal(false);
  readonly errorMessage = signal('');
  readonly searchQuery = signal('');
  readonly favoritesOnly = signal(false);
  readonly favoriteIds = signal<Set<number>>(new Set());
  readonly favoritesList = signal<FavoritePokemonRow[]>([]);

  readonly selectedId = signal(1);
  readonly selectedDetail = signal<PokemonDetailRecord | null>(null);
  readonly selectedNote = signal('');
  readonly noteStatus = signal('');
  readonly isOnline = this.network.isOnline;

  private catalogSeq = 0;
  private detailSeq = 0;

  isFavorite(id: number): boolean {
    return this.favoriteIds().has(id);
  }

  async loadCatalogPage(reset = true): Promise<void> {
    const seq = ++this.catalogSeq;
    const offset = reset ? 0 : this.catalogItems().length;
    const q = encodeURIComponent(this.searchQuery().trim());
    const favOnly = this.favoritesOnly() ? 'true' : 'false';
    const url = `/api/pokemon?q=${q}&offset=${offset}&limit=12&favoritesOnly=${favOnly}`;

    this.loading.set(true);
    this.errorMessage.set('');
    try {
      const res = await firstValueFrom(this.http.get<PokedexListResponse>(url));
      if (seq !== this.catalogSeq) {
        return;
      }
      this.catalogItems.set(reset ? res.items : [...this.catalogItems(), ...res.items]);
      this.totalCount.set(res.total);
      this.hasMore.set(res.hasMore);
      if (Array.isArray(res.favoriteIds)) {
        this.favoriteIds.set(new Set(res.favoriteIds));
      }
    } catch (err) {
      if (seq === this.catalogSeq) {
        const msg = err instanceof Error ? err.message : 'Error al cargar la Pokédex';
        this.errorMessage.set(msg);
      }
    } finally {
      if (seq === this.catalogSeq) {
        this.loading.set(false);
      }
    }
  }

  loadMore(): Promise<void> {
    return this.loadCatalogPage(false);
  }

  setSearchQuery(query: string): Promise<void> {
    this.searchQuery.set(query);
    return this.loadCatalogPage(true);
  }

  toggleFavoritesOnly(): Promise<void> {
    return this.setFavoritesOnly(!this.favoritesOnly());
  }

  setFavoritesOnly(onlyFavorites: boolean): Promise<void> {
    if (this.favoritesOnly() === onlyFavorites && this.catalogSeq > 0) {
      return Promise.resolve();
    }
    this.favoritesOnly.set(onlyFavorites);
    return this.loadCatalogPage(true);
  }

  async refreshFavorites(): Promise<void> {
    try {
      const res = await firstValueFrom(
        this.http.get<{ items: FavoritePokemonRow[] }>('/api/favorites'),
      );
      this.favoritesList.set(res.items);
      this.favoriteIds.set(new Set(res.items.map((item) => item.id)));
    } catch {
      // Keep last known favorites state
    }
  }

  async openPokemon(id: number): Promise<void> {
    const boundedId = Math.max(1, Math.min(1025, Math.trunc(id) || 1));
    const seq = ++this.detailSeq;
    this.selectedId.set(boundedId);
    this.viewMode.set('detail');
    this.loading.set(true);
    this.errorMessage.set('');
    this.noteStatus.set('');

    try {
      const res = await firstValueFrom(
        this.http.get<PokedexDetailResponse>(`/api/pokemon/${boundedId}`),
      );
      if (seq !== this.detailSeq) {
        return;
      }
      this.selectedDetail.set(res.pokemon);
      this.selectedNote.set(res.note);
      this.favoriteIds.update((prev) => {
        const next = new Set(prev);
        if (res.isFavorite) {
          next.add(boundedId);
        } else {
          next.delete(boundedId);
        }
        return next;
      });
      this.titleService.setTitle(
        `Pokédex Nacional — ${res.pokemon.number} ${res.pokemon.displayName}`,
      );
    } catch (err) {
      if (seq === this.detailSeq) {
        const msg = err instanceof Error ? err.message : `No se pudo abrir Pokémon #${boundedId}`;
        this.errorMessage.set(msg);
      }
    } finally {
      if (seq === this.detailSeq) {
        this.loading.set(false);
      }
    }
  }

  backToList(): Promise<void> {
    this.viewMode.set('list');
    this.noteStatus.set('');
    this.titleService.setTitle('Pokédex Nacional');
    return this.loadCatalogPage(true);
  }

  async toggleFavorite(id: number, displayName: string, note = ''): Promise<void> {
    if (this.isFavorite(id)) {
      await firstValueFrom(this.http.delete(`/api/favorites/${id}`));
      this.favoriteIds.update((prev) => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
    } else {
      await firstValueFrom(
        this.http.post('/api/favorites', {
          id,
          name: displayName,
          note: note || this.selectedNote(),
        }),
      );
      this.favoriteIds.update((prev) => {
        const next = new Set(prev);
        next.add(id);
        return next;
      });
    }
    await this.refreshFavorites();
    if (this.viewMode() === 'list' && this.favoritesOnly()) {
      await this.loadCatalogPage(true);
    }
  }

  async saveFieldNote(id: number, note: string): Promise<void> {
    await firstValueFrom(this.http.put(`/api/notes/${id}`, { note }));
    this.selectedNote.set(note);
    this.noteStatus.set('Nota guardada');
  }

  filterCatalog<T extends { id: number; name: string }>(
    entries: readonly T[],
    query: string,
    limit = 18,
  ): T[] {
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
    this.fs.writeJson(targetPath, this.favoritesList());
    return targetPath;
  }
}

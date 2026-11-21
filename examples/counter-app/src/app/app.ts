import { Component, CUSTOM_ELEMENTS_SCHEMA, inject, OnInit, signal } from '@angular/core';
import { PokedexService } from './pokedex.service';
import { PokemonCard } from './pokemon-card';
import { TypeBadge } from './type-badge';

function extractPayloadString(event: unknown): string {
  if (typeof event === 'string') {
    return event;
  }
  if (event && typeof event === 'object') {
    if ('value' in event) {
      return String((event as { value: unknown }).value ?? '');
    }
    if ('target' in event) {
      const target = (event as { target?: { value?: unknown } }).target;
      if (target && 'value' in target) {
        return String(target.value ?? '');
      }
    }
  }
  return '';
}

@Component({
  selector: 'app-root',
  imports: [PokemonCard, TypeBadge],
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  templateUrl: './app.html',
  styleUrl: './app.css',
})
export class App implements OnInit {
  protected readonly pokedex = inject(PokedexService);
  protected readonly windowTitle = signal('Pokédex Nacional');
  protected readonly subtitle = signal('Angular 22 + @ng-desktop/server');
  protected readonly noteDraft = signal('');

  ngOnInit(): void {
    void this.pokedex.refreshFavorites();
    void this.pokedex.loadCatalogPage(true);
  }

  onSearchInput(event: unknown): void {
    void this.pokedex.setSearchQuery(extractPayloadString(event));
  }

  onSpinChanged(event: unknown): void {
    const raw = extractPayloadString(event);
    const id = Number.parseInt(raw, 10);
    if (Number.isInteger(id) && id >= 1 && id <= 1025) {
      void this.openPokemon(id);
    }
  }

  toggleFavoritesFilter(): void {
    void this.pokedex.toggleFavoritesOnly();
  }

  loadMore(): void {
    void this.pokedex.loadMore();
  }

  async openPokemon(id: number): Promise<void> {
    await this.pokedex.openPokemon(id);
    this.noteDraft.set(this.pokedex.selectedNote());
  }

  backToList(): void {
    void this.pokedex.backToList();
  }

  previousPokemon(): void {
    const prev = this.pokedex.selectedId() - 1;
    void this.openPokemon(prev >= 1 ? prev : 1);
  }

  nextPokemon(): void {
    const next = this.pokedex.selectedId() + 1;
    void this.openPokemon(next <= 1025 ? next : 1025);
  }

  toggleCurrentFavorite(): void {
    const detail = this.pokedex.selectedDetail();
    if (!detail) {
      return;
    }
    void this.pokedex.toggleFavorite(detail.id, detail.displayName, this.noteDraft());
  }

  onNoteInput(event: unknown): void {
    this.noteDraft.set(extractPayloadString(event));
  }

  saveNote(): void {
    const detail = this.pokedex.selectedDetail();
    if (!detail) {
      return;
    }
    void this.pokedex.saveFieldNote(detail.id, this.noteDraft());
  }
}

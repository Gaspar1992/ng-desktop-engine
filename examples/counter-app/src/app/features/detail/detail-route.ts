import { Component, effect, inject, input } from '@angular/core';
import { Router } from '@ng-desktop/router';
import { PokedexService } from '../../core/services/pokedex.service';
import { PokemonDetailView } from './pokemon-detail-view';

@Component({
  selector: 'app-detail-route',
  imports: [PokemonDetailView],
  template: `
    @if (pokedex.selectedDetail(); as detail) {
      <app-pokemon-detail-view
        [detail]="detail"
        [isFavorite]="pokedex.isFavorite(detail.id)"
        [noteStatus]="pokedex.noteStatus()"
        [noteControl]="pokedex.noteControl"
        (back)="backToList()"
        (previous)="previousPokemon()"
        (next)="nextPokemon()"
        (toggleFavorite)="toggleCurrentFavorite()"
        (saveNote)="saveNote()"
      />
    }
  `,
})
export class DetailRouteComponent {
  protected readonly pokedex = inject(PokedexService);
  private readonly router = inject(Router);

  /** Route `:id` bound automatically via `withComponentInputBinding()`. */
  readonly id = input<string>();

  constructor() {
    effect(() => {
      const parsed = Number.parseInt(this.id() ?? '', 10);
      if (!Number.isInteger(parsed) || parsed < 1 || parsed > 1025) {
        return;
      }
      if (
        this.pokedex.viewMode() !== 'detail' ||
        this.pokedex.selectedId() !== parsed ||
        this.pokedex.selectedDetail()?.id !== parsed
      ) {
        void this.pokedex.openPokemon(parsed);
      }
    });
  }

  backToList(): void {
    void this.router.navigate(['/pokemon']);
  }

  previousPokemon(): void {
    const prev = Math.max(1, this.pokedex.selectedId() - 1);
    void this.router.navigate(['/pokemon', prev]);
  }

  nextPokemon(): void {
    const next = Math.min(1025, this.pokedex.selectedId() + 1);
    void this.router.navigate(['/pokemon', next]);
  }

  toggleCurrentFavorite(): void {
    const detail = this.pokedex.selectedDetail();
    if (!detail) {
      return;
    }
    void this.pokedex.toggleFavorite(
      detail.id,
      detail.displayName,
      this.pokedex.noteControl.getRawValue(),
    );
  }

  saveNote(): void {
    const detail = this.pokedex.selectedDetail();
    if (!detail) {
      return;
    }
    void this.pokedex.saveFieldNote(detail.id, this.pokedex.noteControl.getRawValue());
  }
}

import { Component, computed, input, output } from '@angular/core';
import { FormControl, ReactiveFormsModule } from '@angular/forms';
import { NATIVE_FORM_DIRECTIVES } from '@ng-desktop/forms';
import { NATIVE_UI_DIRECTIVES } from '@ng-desktop/platform';
import type { PokemonDetailRecord } from '../../../server/pokedex-store';
import { PokemonCard } from '../../shared/ui/pokemon-card/pokemon-card';
import { StatMeter } from '../../shared/ui/stat-meter/stat-meter';
import { TypeBadge } from '../../shared/ui/type-badge/type-badge';

@Component({
  selector: 'app-pokemon-detail-view',
  imports: [
    ReactiveFormsModule,
    NATIVE_FORM_DIRECTIVES,
    NATIVE_UI_DIRECTIVES,
    PokemonCard,
    TypeBadge,
    StatMeter,
  ],
  templateUrl: './pokemon-detail-view.html',
  styleUrl: './pokemon-detail-view.css',
})
export class PokemonDetailView {
  readonly detail = input.required<PokemonDetailRecord>();
  readonly isFavorite = input.required<boolean>();
  readonly noteStatus = input<string>('');
  readonly noteControl = input.required<FormControl<string>>();

  readonly totalStats = computed(() => {
    const current = this.detail();
    return current.totalStats ?? current.stats.reduce((acc, s) => acc + s.value, 0);
  });

  readonly back = output<void>();
  readonly previous = output<void>();
  readonly next = output<void>();
  readonly toggleFavorite = output<void>();
  readonly saveNote = output<void>();
}

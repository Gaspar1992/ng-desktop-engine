import { Component, input, output } from '@angular/core';
import { NATIVE_UI_DIRECTIVES, NativeVirtualGridComponent } from '@ng-desktop/platform';
import type { PokedexCatalogCard } from '../../core/services/pokedex.service';

@Component({
  selector: 'app-catalog-view',
  imports: [NATIVE_UI_DIRECTIVES, NativeVirtualGridComponent],
  templateUrl: './catalog-view.html',
  styleUrl: './catalog-view.css',
})
export class CatalogView {
  readonly items = input.required<readonly PokedexCatalogCard[]>();
  readonly totalCount = input.required<number>();
  readonly favoriteIds = input.required<ReadonlySet<number>>();
  readonly hasMore = input<boolean>(false);

  readonly selectPokemon = output<number>();
  readonly loadMore = output<void>();

  readonly trackByPokemonId = (_index: number, poke: PokedexCatalogCard): number => poke.id;
}

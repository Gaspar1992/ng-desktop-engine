import { Component, CUSTOM_ELEMENTS_SCHEMA, input, output } from '@angular/core';
import type { PokedexCatalogCard } from '../../core/services/pokedex.service';

@Component({
  selector: 'app-catalog-view',
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
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
}

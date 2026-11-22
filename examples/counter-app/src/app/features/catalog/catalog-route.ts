import { Component, inject, OnInit } from '@angular/core';
import { Router } from '@ng-desktop/router';
import { PokedexService } from '../../core/services/pokedex.service';
import { CatalogView } from './catalog-view';

@Component({
  selector: 'app-catalog-route',
  imports: [CatalogView],
  template: `
    <app-catalog-view
      [items]="pokedex.catalogItems()"
      [totalCount]="pokedex.totalCount()"
      [favoriteIds]="pokedex.favoriteIds()"
      [hasMore]="pokedex.hasMore()"
      (selectPokemon)="openPokemon($event)"
      (loadMore)="loadMore()"
    />
  `,
})
export class CatalogRouteComponent implements OnInit {
  protected readonly pokedex = inject(PokedexService);
  private readonly router = inject(Router);

  ngOnInit(): void {
    if (this.pokedex.viewMode() !== 'list') {
      void this.pokedex.backToList();
    }
  }

  openPokemon(id: number): void {
    const boundedId = Math.max(1, Math.min(1025, Math.trunc(id) || 1));
    void this.router.navigate(['/pokemon', boundedId]);
  }

  loadMore(): void {
    void this.pokedex.loadMore();
  }
}

import { Routes } from '@ng-desktop/router';
import { CatalogRouteComponent } from './features/catalog/catalog-route';
import { DetailRouteComponent } from './features/detail/detail-route';

export const appRoutes: Routes = [
  { path: '', pathMatch: 'full', redirectTo: 'pokemon' },
  {
    path: 'pokemon',
    component: CatalogRouteComponent,
    title: 'Catálogo Nacional',
  },
  {
    path: 'pokemon/:id',
    component: DetailRouteComponent,
    title: 'Ficha Pokémon',
  },
];

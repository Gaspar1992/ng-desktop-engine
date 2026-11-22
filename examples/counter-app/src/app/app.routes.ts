import { Routes } from '@ng-desktop/router';

export const appRoutes: Routes = [
  { path: '', pathMatch: 'full', redirectTo: 'pokemon' },
  {
    path: 'pokemon',
    loadComponent: () =>
      import('./features/catalog/catalog-route').then((m) => m.CatalogRouteComponent),
    title: 'Catálogo Nacional',
  },
  {
    path: 'pokemon/:id',
    loadComponent: () =>
      import('./features/detail/detail-route').then((m) => m.DetailRouteComponent),
    title: 'Ficha Pokémon',
  },
];

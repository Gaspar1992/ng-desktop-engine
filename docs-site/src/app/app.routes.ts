import { Routes } from '@angular/router';

export const routes: Routes = [
  {
    path: '',
    loadComponent: () => import('./pages/home.page').then((m) => m.HomePage),
    title: 'ng-desktop-engine — Angular 22 Native Desktop Apps',
  },
  {
    path: 'getting-started',
    loadComponent: () => import('./pages/getting-started.page').then((m) => m.GettingStartedPage),
    title: 'Getting Started — ng-desktop-engine',
  },
  {
    path: 'architecture',
    loadComponent: () => import('./pages/architecture.page').then((m) => m.ArchitecturePage),
    title: 'Engine Architecture — ng-desktop-engine',
  },
  {
    path: 'widgets',
    loadComponent: () => import('./pages/widgets.page').then((m) => m.WidgetsPage),
    title: '29 Native Primitives — ng-desktop-engine',
  },
  {
    path: 'playground',
    loadComponent: () => import('./pages/playground.page').then((m) => m.PlaygroundPage),
    title: 'Live .ngb Playground — ng-desktop-engine',
  },
  {
    path: 'packages',
    loadComponent: () => import('./pages/packages.page').then((m) => m.PackagesPage),
    title: 'Packages & Native Binaries — ng-desktop-engine',
  },
  {
    path: 'showcase/pokedex',
    loadComponent: () => import('./pages/pokedex-showcase.page').then((m) => m.PokedexShowcasePage),
    title: 'Pokédex Showcase — ng-desktop-engine',
  },
  {
    path: '**',
    redirectTo: '',
  },
];

import { Component } from '@angular/core';
import { Routes } from '@ng-desktop/router';

@Component({
  selector: 'app-pokedex-route-outlet',
  template: '',
})
export class PokedexRouteOutletComponent {}

export const appRoutes: Routes = [
  { path: '', pathMatch: 'full', redirectTo: 'pokemon' },
  { path: 'pokemon', component: PokedexRouteOutletComponent, title: 'Catálogo Nacional' },
  { path: 'pokemon/:id', component: PokedexRouteOutletComponent, title: 'Ficha Pokémon' },
];

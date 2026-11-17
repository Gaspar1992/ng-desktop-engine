import { HttpClient } from '@angular/common/http';
import { Component, CUSTOM_ELEMENTS_SCHEMA, inject, signal } from '@angular/core';
import { Observable } from 'rxjs';
import { PokemonCard } from './pokemon-card';
import { TypeBadge } from './type-badge';

@Component({
  selector: 'app-root',
  imports: [PokemonCard, TypeBadge],
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  templateUrl: './app.html',
  styleUrl: './app.css',
})
export class App {
  private readonly http = inject(HttpClient, { optional: true });
  protected readonly windowTitle = signal('Pokédex Nacional');
  protected readonly subtitle = signal('Angular 22 + GTK4 / Libadwaita');
  protected readonly greeting = signal('Bulbasaur');
  protected readonly author = signal('');
  protected readonly pokemonId = signal(0);

  /**
   * Executes a live `PokéAPI v2` request via Angular `HttpClient` -> `@ng-native/http` -> Rust OS HTTP stack.
   */
  fetchPokemonHttp(idOrName: string | number): Observable<unknown> | null {
    if (!this.http) {
      return null;
    }
    return this.http.get(`https://pokeapi.co/api/v2/pokemon/${idOrName}`);
  }

  increment(): void {
    this.pokemonId.update((value) => value + 1);
  }

  decrement(): void {
    this.pokemonId.update((value) => value - 1);
  }

  reset(): void {
    this.pokemonId.set(0);
  }

  nextPokemon(): void {
    this.pokemonId.update((value) => value + 1);
  }

  previousPokemon(): void {
    this.pokemonId.update((value) => value - 1);
  }

  backToList(): void {
    this.pokemonId.set(0);
  }

  loadMorePokemon(): void {
    // Handled natively by PokedexController (@defer on viewport / load-more pagination)
  }

  selectBulbasaur(): void {
    this.pokemonId.set(1);
  }

  selectCharmander(): void {
    this.pokemonId.set(4);
  }

  selectSquirtle(): void {
    this.pokemonId.set(7);
  }

  selectPikachu(): void {
    this.pokemonId.set(25);
  }

  selectCharizard(): void {
    this.pokemonId.set(6);
  }

  selectGengar(): void {
    this.pokemonId.set(94);
  }
}

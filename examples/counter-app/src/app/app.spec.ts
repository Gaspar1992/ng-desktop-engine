import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@ng-native/http';
import { ensureNgNativeBridge, NativeTitleService } from '@ng-native/platform';
import { firstValueFrom } from 'rxjs';
import { App } from './app';
import { PokedexService } from './pokedex.service';

describe('App', () => {
  beforeEach(async () => {
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();
    bridge.registerHttpEndpoint(
      'GET',
      'https://pokeapi.co/api/v2/pokemon/1',
      200,
      JSON.stringify({ id: 1, name: 'bulbasaur' }),
    );
    bridge.registerHttpEndpoint(
      'GET',
      'https://pokeapi.co/api/v2/pokemon/25',
      200,
      JSON.stringify({ id: 25, name: 'pikachu' }),
    );

    await TestBed.configureTestingModule({
      imports: [App],
      providers: [provideHttpClient()],
    }).compileComponents();
  });

  it('should create the app', () => {
    const fixture = TestBed.createComponent(App);
    const app = fixture.componentInstance;
    expect(app).toBeTruthy();
  });

  it('should initialize default signals and navigate between pokedex list and detail views', () => {
    const fixture = TestBed.createComponent(App);
    const app = fixture.componentInstance;

    expect(app['windowTitle']()).toBe('Pokédex Nacional');
    expect(app['subtitle']()).toBe('Angular 22 + GTK4 / Libadwaita');
    expect(app['author']()).toBe('');
    expect(app['pokemonId']()).toBe(0);

    app.selectBulbasaur();
    expect(app['pokemonId']()).toBe(1);

    app.selectPikachu();
    expect(app['pokemonId']()).toBe(25);

    app.backToList();
    expect(app['pokemonId']()).toBe(0);

    app.nextPokemon();
    expect(app['pokemonId']()).toBe(1);

    app.previousPokemon();
    expect(app['pokemonId']()).toBe(0);

    app.increment();
    app.increment();
    expect(app['pokemonId']()).toBe(2);

    app.decrement();
    expect(app['pokemonId']()).toBe(1);

    app.reset();
    expect(app['pokemonId']()).toBe(0);
  });

  it('wires PokedexService with native batching, httpResource, and catalog filtering', async () => {
    const service = TestBed.inject(PokedexService);
    const fixture = TestBed.createComponent(App);
    const app = fixture.componentInstance;

    expect(service.selectedPokemonResource.status()).toBe('resolved');
    expect(service.selectedPokemonResource.value()?.name).toBe('bulbasaur');

    service.selectPokemon(25);
    expect(service.selectedPokemonResource.value()?.name).toBe('pikachu');

    const batch = service.fetchBatch([1, 25]);
    expect(batch.map((p) => p.name)).toEqual(['bulbasaur', 'pikachu']);

    const catalog = [
      { id: 1, name: 'bulbasaur' },
      { id: 4, name: 'charmander' },
      { id: 25, name: 'pikachu' },
    ];
    expect(service.filterCatalog(catalog, 'pika').map((e) => e.id)).toEqual([25]);
    expect(service.filterCatalog(catalog, '1').map((e) => e.name)).toEqual(['bulbasaur']);

    const httpObs = app.fetchPokemonHttp(25);
    expect(httpObs).not.toBeNull();
    const fetched = (await firstValueFrom(httpObs!)) as { id: number; name: string };
    expect(fetched.name).toBe('pikachu');

    service.saveFavorite(25, 'Pikachu', 'Inicial eléctrico');
    service.saveFavorite(1, 'Bulbasaur', 'Semilla');
    expect(service.favorites.count()).toBe(2);
    expect(service.favorites.rows().map((f) => f.id)).toEqual([1, 25]);
    service.removeFavorite(1);
    expect(service.favorites.rows()).toEqual([
      { id: 25, name: 'Pikachu', note: 'Inicial eléctrico' },
    ]);

    const titleService = TestBed.inject(NativeTitleService);
    service.selectPokemon(25, 'Pikachu');
    expect(titleService.getTitle()).toBe('Pokédex Nacional — #0025 Pikachu');

    const exportedPath = service.exportFavoritesToDisk('favorites-backup.json');
    expect(exportedPath).toContain('favorites-backup.json');
    service.removeFavorite(25);
    expect(service.favorites.count()).toBe(0);

    const importedCount = service.importFavoritesFromFile(exportedPath);
    expect(importedCount).toBe(1);
    expect(service.favorites.rows()).toEqual([
      { id: 25, name: 'Pikachu', note: 'Inicial eléctrico' },
    ]);
  });
});

import { TestBed } from '@angular/core/testing';
import {
  ensureNgNativeBridge,
  NativeTitleService,
  provideNativePlatform,
} from '@ng-desktop/platform';
import { App } from './app';
import { appConfig } from './app.config';
import { PokedexService } from './pokedex.service';

describe('App (100% Angular + @ng-desktop/server API)', () => {
  const catalogResponse = {
    total: 2,
    offset: 0,
    limit: 12,
    hasMore: false,
    favoriteIds: [],
    items: [
      {
        id: 1,
        name: 'bulbasaur',
        displayName: 'Bulbasaur',
        number: 'N.º 0001',
        spriteUrl: 'https://cdn.example.test/1.png',
      },
      {
        id: 25,
        name: 'pikachu',
        displayName: 'Pikachu',
        number: 'N.º 0025',
        spriteUrl: 'https://cdn.example.test/25.png',
      },
    ],
  };

  const pikachuDetail = {
    pokemon: {
      id: 25,
      name: 'pikachu',
      displayName: 'Pikachu',
      number: 'N.º 0025',
      spriteUrl: 'https://cdn.example.test/25.png',
      genus: 'Pokémon Ratón',
      description: 'Cuando se enfada descarga energía.',
      heightMeters: '0.4 m',
      weightKg: '6.0 kg',
      ability: 'Static',
      baseExperience: '112 EXP',
      types: [{ label: 'ELÉCTRICO', badgeClass: 'badge-electric' }],
      stats: [{ key: 'hp', label: 'PS', value: 35 }],
    },
    isFavorite: false,
    note: '',
    cached: false,
  };

  beforeEach(async () => {
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();
    bridge.registerHttpEndpoint('GET', '/api/favorites', 200, JSON.stringify({ items: [] }));
    bridge.registerHttpEndpoint(
      'GET',
      '/api/pokemon?q=&offset=0&limit=12&favoritesOnly=false',
      200,
      JSON.stringify(catalogResponse),
    );
    bridge.registerHttpEndpoint(
      'GET',
      '/api/pokemon?q=pika&offset=0&limit=12&favoritesOnly=false',
      200,
      JSON.stringify({
        ...catalogResponse,
        total: 1,
        items: [catalogResponse.items[1]],
      }),
    );
    bridge.registerHttpEndpoint('GET', '/api/pokemon/25', 200, JSON.stringify(pikachuDetail));
    bridge.registerHttpEndpoint(
      'POST',
      '/api/favorites',
      201,
      JSON.stringify({ saved: true, item: { id: 25, name: 'Pikachu', note: '' } }),
    );
    bridge.registerHttpEndpoint(
      'DELETE',
      '/api/favorites/25',
      200,
      JSON.stringify({ removed: true }),
    );
    bridge.registerHttpEndpoint(
      'PUT',
      '/api/notes/25',
      200,
      JSON.stringify({ saved: true, id: 25, note: 'Nota inicial' }),
    );

    TestBed.resetTestingModule();
    await TestBed.configureTestingModule({
      imports: [App],
      providers: [provideNativePlatform(), ...(appConfig.providers ?? [])],
    }).compileComponents();
  });

  it('mounts Angular root, loads catalog from /api/pokemon, and filters by search query', async () => {
    const fixture = TestBed.createComponent(App);
    const service = TestBed.inject(PokedexService);
    fixture.detectChanges();
    await Promise.resolve();
    await Promise.resolve();

    expect(service.catalogItems().map((item) => item.displayName)).toEqual([
      'Bulbasaur',
      'Pikachu',
    ]);

    await service.setSearchQuery('pika');
    expect(service.catalogItems().map((item) => item.id)).toEqual([25]);

    const filtered = service.filterCatalog(
      [
        { id: 1, name: 'bulbasaur' },
        { id: 25, name: 'pikachu' },
      ],
      'pika',
    );
    expect(filtered.map((item) => item.id)).toEqual([25]);
  });

  it('opens detail view, toggles favorites, saves notes, updates NativeTitleService, and returns to list', async () => {
    const fixture = TestBed.createComponent(App);
    const app = fixture.componentInstance;
    const service = TestBed.inject(PokedexService);
    const titleService = TestBed.inject(NativeTitleService);
    fixture.detectChanges();

    await app.openPokemon(25);
    expect(service.viewMode()).toBe('detail');
    expect(service.selectedDetail()?.displayName).toBe('Pikachu');
    expect(titleService.getTitle()).toBe('Pokédex Nacional — N.º 0025 Pikachu');

    const bridge = ensureNgNativeBridge();
    bridge.registerHttpEndpoint(
      'GET',
      '/api/favorites',
      200,
      JSON.stringify({ items: [{ id: 25, name: 'Pikachu', note: 'Nota inicial' }] }),
    );

    await service.toggleFavorite(25, 'Pikachu');
    expect(service.isFavorite(25)).toBe(true);
    expect(service.favoritesList()).toHaveLength(1);

    const exportedPath = service.exportFavoritesToDisk('pokedex-unit-backup.json');
    expect(exportedPath).toContain('pokedex-unit-backup.json');

    await service.saveFieldNote(25, 'Nota inicial');
    expect(service.selectedNote()).toBe('Nota inicial');
    expect(service.noteStatus()).toBe('Nota guardada');

    bridge.registerHttpEndpoint('GET', '/api/favorites', 200, JSON.stringify({ items: [] }));
    await service.toggleFavorite(25, 'Pikachu');
    expect(service.isFavorite(25)).toBe(false);

    await service.backToList();
    expect(service.viewMode()).toBe('list');
    expect(titleService.getTitle()).toBe('Pokédex Nacional');
  });
});

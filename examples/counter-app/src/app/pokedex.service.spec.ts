import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { provideNativeDesktop } from '@ng-desktop/desktop';
import {
  ensureNgNativeBridge,
  NativeTitleService,
  provideNativePlatform,
} from '@ng-desktop/platform';
import { PokedexService } from './core/services/pokedex.service';

describe('PokedexService', () => {
  let service: PokedexService;
  let httpMock: HttpTestingController;
  let titleSpy: { setTitle: (t: string) => void; calls: string[] };

  beforeEach(() => {
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();
    titleSpy = {
      calls: [],
      setTitle(t: string) {
        titleSpy.calls.push(t);
      },
    };
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      providers: [
        provideNativePlatform(),
        provideNativeDesktop(),
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: NativeTitleService, useValue: titleSpy },
      ],
    });
    service = TestBed.inject(PokedexService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  it('loads pages, supports loadMore, and captures HTTP errors', async () => {
    const firstPage = service.loadCatalogPage(true);
    const req1 = httpMock.expectOne('/api/pokemon?q=&offset=0&limit=12&favoritesOnly=false');
    req1.flush({
      total: 2,
      offset: 0,
      limit: 12,
      hasMore: true,
      favoriteIds: [1],
      items: [
        {
          id: 1,
          name: 'bulbasaur',
          displayName: 'Bulbasaur',
          number: 'N.º 0001',
          spriteUrl: 'local://1.png',
        },
      ],
    });
    await firstPage;
    expect(service.catalogItems().length).toBe(1);
    expect(service.hasMore()).toBe(true);
    expect(service.isFavorite(1)).toBe(true);

    const nextPage = service.loadMore();
    const req2 = httpMock.expectOne('/api/pokemon?q=&offset=1&limit=12&favoritesOnly=false');
    req2.flush({
      total: 2,
      offset: 1,
      limit: 12,
      hasMore: false,
      favoriteIds: [1],
      items: [
        {
          id: 25,
          name: 'pikachu',
          displayName: 'Pikachu',
          number: 'N.º 0025',
          spriteUrl: 'local://25.png',
        },
      ],
    });
    await nextPage;
    expect(service.catalogItems().map((item) => item.id)).toEqual([1, 25]);
    expect(service.hasMore()).toBe(false);

    // No-op when hasMore is false
    await service.loadMore();

    const errLoad = service.loadCatalogPage(true);
    const reqErr = httpMock.expectOne('/api/pokemon?q=&offset=0&limit=12&favoritesOnly=false');
    reqErr.flush('boom', { status: 500, statusText: 'Server Error' });
    await errLoad;
    expect(service.errorMessage()).toBe('Error al cargar la Pokédex');
    expect(service.loading()).toBe(false);
  });

  it('clamps openPokemon bounds, updates title, handles errors, and toggles favorites/notes', async () => {
    const lowOpen = service.openPokemon(-9);
    const lowReq = httpMock.expectOne('/api/pokemon/1');
    lowReq.flush({
      pokemon: {
        id: 1,
        name: 'bulbasaur',
        displayName: 'Bulbasaur',
        number: 'N.º 0001',
        spriteUrl: 'local://1.png',
        genus: 'Semilla',
        description: 'Semilla dorsal',
        heightMeters: '0.7 m',
        weightKg: '6.9 kg',
        ability: 'Overgrow',
        baseExperience: '64 EXP',
        types: [],
        stats: [{ key: 'hp', label: 'PS', value: 45 }],
      },
      isFavorite: true,
      note: 'Initial note',
      cached: false,
    });
    await lowOpen;

    expect(service.selectedId()).toBe(1);
    expect(service.isFavorite(1)).toBe(true);
    expect(service.selectedNote()).toBe('Initial note');
    expect(titleSpy.calls.at(-1)).toBe('Pokédex Nacional — N.º 0001 Bulbasaur');

    const highErr = service.openPokemon(9999);
    const highReq = httpMock.expectOne('/api/pokemon/1025');
    highReq.flush('missing', { status: 404, statusText: 'Not Found' });
    await highErr;
    expect(service.selectedId()).toBe(1025);
    expect(service.errorMessage()).toBe('No se pudo abrir Pokémon #1025');

    const noteSave = service.saveFieldNote(1, 'Updated note');
    const noteReq = httpMock.expectOne('/api/notes/1');
    expect(noteReq.request.method).toBe('PUT');
    noteReq.flush({ ok: true });
    await noteSave;
    expect(service.noteStatus()).toBe('Nota guardada');

    const removeFav = service.toggleFavorite(1, 'Bulbasaur');
    const delReq = httpMock.expectOne('/api/favorites/1');
    expect(delReq.request.method).toBe('DELETE');
    delReq.flush({ deleted: true });
    await Promise.resolve();
    const favListReq = httpMock.expectOne('/api/favorites');
    favListReq.flush({ items: [] });
    await removeFav;
    expect(service.isFavorite(1)).toBe(false);
  });
});

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ensureNgNativeBridge, provideNativePlatform } from '@ng-desktop/platform';
import { PokedexCatalogCard } from './core/services/pokedex.service';
import { CatalogView } from './features/catalog/catalog-view';

describe('CatalogView', () => {
  let fixture: ComponentFixture<CatalogView>;
  let component: CatalogView;

  const sampleItems: PokedexCatalogCard[] = [
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
  ];

  beforeEach(async () => {
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();
    TestBed.resetTestingModule();
    await TestBed.configureTestingModule({
      imports: [CatalogView],
      providers: [provideNativePlatform()],
    }).compileComponents();

    fixture = TestBed.createComponent(CatalogView);
    component = fixture.componentInstance;
  });

  it('renders virtual grid cards with star indicator for favorites and trackBy', () => {
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();

    fixture.componentRef.setInput('items', sampleItems);
    fixture.componentRef.setInput('totalCount', 151);
    fixture.componentRef.setInput('favoriteIds', new Set([25]));
    fixture.componentRef.setInput('hasMore', true);
    fixture.detectChanges();

    const ops = bridge.__drainQueue();
    expect(ops).toContain('Mostrando 2 de 151 Pokémon (1 favoritos)');
    expect(ops).toContain('Bulbasaur');
    expect(ops).toContain('★ Pikachu');
    expect(ops).toContain('btn_load_more');
    expect(component.trackByPokemonId(1, sampleItems[1])).toBe(25);
  });

  it('renders empty state when catalog items array is empty and omits pagination button', () => {
    const bridge = ensureNgNativeBridge();
    fixture.componentRef.setInput('items', []);
    fixture.componentRef.setInput('totalCount', 0);
    fixture.componentRef.setInput('favoriteIds', new Set());
    fixture.componentRef.setInput('hasMore', false);
    fixture.detectChanges();

    const ops = bridge.__drainQueue();
    expect(ops).toContain('No se encontraron Pokémon para tu búsqueda.');
    expect(ops).not.toContain('Cargar más Pokémon (+12)');
  });

  it('emits selectPokemon and loadMore events', () => {
    const bridge = ensureNgNativeBridge();
    const selected: number[] = [];
    let loadMoreCount = 0;
    component.selectPokemon.subscribe((id) => selected.push(id));
    component.loadMore.subscribe(() => {
      loadMoreCount += 1;
    });

    fixture.componentRef.setInput('items', sampleItems);
    fixture.componentRef.setInput('totalCount', 24);
    fixture.componentRef.setInput('favoriteIds', new Set());
    fixture.componentRef.setInput('hasMore', true);
    fixture.detectChanges();

    const ops = bridge.__drainQueue();
    const openMatch = ops.match(/attr (ng_node_\d+) data-testid open-poke-25/);
    const loadMoreMatch = ops.match(/attr (ng_node_\d+) id btn_load_more/);
    expect(openMatch).toBeTruthy();
    expect(loadMoreMatch).toBeTruthy();

    bridge.__dispatchEvent(openMatch![1], 'clicked', {
      type: 'clicked',
      target: openMatch![1],
      value: null,
    });
    bridge.__dispatchEvent(loadMoreMatch![1], 'clicked', {
      type: 'clicked',
      target: loadMoreMatch![1],
      value: null,
    });
    fixture.detectChanges();

    expect(selected).toEqual([25]);
    expect(loadMoreCount).toBe(1);
  });
});

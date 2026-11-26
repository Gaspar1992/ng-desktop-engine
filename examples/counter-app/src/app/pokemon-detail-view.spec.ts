import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ensureNgNativeBridge, provideNativePlatform } from '@ng-desktop/platform';
import type { PokemonDetailRecord } from '../server/pokedex-store';
import { PokemonDetailView } from './features/detail/pokemon-detail-view';

describe('PokemonDetailView', () => {
  let fixture: ComponentFixture<PokemonDetailView>;
  let component: PokemonDetailView;

  const baseDetail: PokemonDetailRecord = {
    id: 25,
    name: 'pikachu',
    displayName: 'Pikachu',
    number: 'N.º 0025',
    spriteUrl: 'https://cdn.example.test/25.png',
    genus: 'Pokémon Ratón',
    description: 'Almacena electricidad en sus mejillas.',
    heightMeters: '0.4 m',
    weightKg: '6.0 kg',
    ability: 'Static',
    baseExperience: '112 EXP',
    types: [{ label: 'ELÉCTRICO', badgeClass: 'badge-electric' }],
    stats: [
      { key: 'hp', label: 'PS', value: 35 },
      { key: 'spe', label: 'VEL', value: 90 },
    ],
  };

  beforeEach(async () => {
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();
    TestBed.resetTestingModule();
    await TestBed.configureTestingModule({
      imports: [PokemonDetailView],
      providers: [provideNativePlatform()],
    }).compileComponents();

    fixture = TestBed.createComponent(PokemonDetailView);
    component = fixture.componentInstance;
  });

  it('computes totalStats fallback from stats array when totalStats is omitted', () => {
    fixture.componentRef.setInput('detail', baseDetail);
    fixture.componentRef.setInput('isFavorite', false);
    fixture.detectChanges();

    expect(component.totalStats()).toBe(125);

    fixture.componentRef.setInput('detail', { ...baseDetail, totalStats: 320 });
    fixture.detectChanges();

    expect(component.totalStats()).toBe(320);
  });

  it('renders favorite state, note status, and emits action outputs', () => {
    const bridge = ensureNgNativeBridge();
    let backCount = 0;
    let prevCount = 0;
    let nextCount = 0;
    const favNotes: string[] = [];
    const savedNotes: string[] = [];

    component.back.subscribe(() => {
      backCount += 1;
    });
    component.previous.subscribe(() => {
      prevCount += 1;
    });
    component.next.subscribe(() => {
      nextCount += 1;
    });
    component.toggleFavorite.subscribe((n) => favNotes.push(n));
    component.saveNote.subscribe((n) => savedNotes.push(n));

    fixture.componentRef.setInput('detail', baseDetail);
    fixture.componentRef.setInput('isFavorite', true);
    fixture.componentRef.setInput('note', 'Electrified note');
    fixture.componentRef.setInput('noteStatus', 'Nota guardada');
    fixture.detectChanges();

    const ops = bridge.__drainQueue();
    expect(ops).toContain('★ En Favoritos');
    expect(ops).toContain('Nota guardada');
    expect(ops).toContain('Pokémon Ratón');

    for (const btnId of ['btn_back', 'btn_prev', 'btn_next', 'btn_toggle_fav', 'btn_save_note']) {
      const match = ops.match(new RegExp(`attr (ng_node_\\d+) id ${btnId}`));
      expect(match, `Missing native node for ${btnId}`).toBeTruthy();
      bridge.__dispatchEvent(match![1], 'clicked', {
        type: 'clicked',
        target: match![1],
        value: null,
      });
    }

    expect(backCount).toBe(1);
    expect(prevCount).toBe(1);
    expect(nextCount).toBe(1);
    expect(favNotes).toEqual(['Electrified note']);
    expect(savedNotes).toEqual(['Electrified note']);
  });
});

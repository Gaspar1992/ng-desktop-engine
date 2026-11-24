import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideNativePlatform } from '@ng-desktop/platform';
import { PokemonCard } from './pokemon-card';

describe('PokemonCard', () => {
  let fixture: ComponentFixture<PokemonCard>;
  let component: PokemonCard;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    await TestBed.configureTestingModule({
      imports: [PokemonCard],
      providers: [provideNativePlatform()],
    }).compileComponents();

    fixture = TestBed.createComponent(PokemonCard);
    component = fixture.componentInstance;
  });

  it('should create the pokemon card component', () => {
    expect(component).toBeTruthy();
  });

  it('should accept name, number, and sprite inputs', () => {
    fixture.componentRef.setInput('name', 'PIKACHU');
    fixture.componentRef.setInput('number', 'N.º 0025');
    fixture.componentRef.setInput('sprite', '/tmp/ng_native_pokedex/25.png');
    fixture.detectChanges();

    expect(component.name()).toBe('PIKACHU');
    expect(component.number()).toBe('N.º 0025');
    expect(component.sprite()).toBe('/tmp/ng_native_pokedex/25.png');
  });
});

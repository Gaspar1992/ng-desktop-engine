import {
  Component,
  CUSTOM_ELEMENTS_SCHEMA,
  inject,
  OnDestroy,
  OnInit,
  signal,
} from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import { FormControl, ReactiveFormsModule } from '@angular/forms';
import { NativeDesktopService } from '@ng-desktop/desktop';
import { NATIVE_FORM_DIRECTIVES } from '@ng-desktop/forms';
import { NativePlatformLocation, Router, RouterOutlet } from '@ng-desktop/router';
import { PokedexService } from './pokedex.service';
import { PokemonCard } from './pokemon-card';
import { TypeBadge } from './type-badge';

@Component({
  selector: 'app-root',
  imports: [ReactiveFormsModule, NATIVE_FORM_DIRECTIVES, RouterOutlet, PokemonCard, TypeBadge],
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  templateUrl: './app.html',
  styleUrl: './app.css',
})
export class App implements OnInit, OnDestroy {
  protected readonly pokedex = inject(PokedexService);
  private readonly desktop = inject(NativeDesktopService);
  private readonly router = inject(Router);
  private readonly location = inject(NativePlatformLocation);

  protected readonly windowTitle = signal('Pokédex Nacional');
  protected readonly subtitle = signal(
    'Angular 22 + @ng-desktop/router + @ng-desktop/forms + @ng-desktop/server',
  );

  readonly searchControl = new FormControl('', { nonNullable: true });
  readonly spinIdControl = new FormControl(1, { nonNullable: true });
  readonly favoritesOnlyControl = new FormControl(false, { nonNullable: true });
  readonly noteControl = new FormControl('', { nonNullable: true });

  private unlistenPopState: VoidFunction | null = null;

  constructor() {
    this.searchControl.valueChanges.pipe(takeUntilDestroyed()).subscribe((query) => {
      void this.pokedex.setSearchQuery(query);
    });

    this.spinIdControl.valueChanges.pipe(takeUntilDestroyed()).subscribe((rawId) => {
      const id = Math.trunc(rawId);
      if (Number.isInteger(id) && id >= 1 && id <= 1025 && id !== this.pokedex.selectedId()) {
        void this.openPokemon(id);
      }
    });

    this.favoritesOnlyControl.valueChanges.pipe(takeUntilDestroyed()).subscribe((onlyFavorites) => {
      void this.pokedex.setFavoritesOnly(onlyFavorites);
    });
  }

  ngOnInit(): void {
    this.desktop.loadWindowGeometry();
    const menuItems = [
      {
        id: 'refresh-catalog',
        label: 'Recargar catálogo',
        accelerator: '<Control>r',
        enabled: true,
      },
      {
        id: 'toggle-favorites',
        label: 'Alternar filtro favoritos',
        accelerator: '<Control>b',
        enabled: true,
      },
      {
        id: 'export-favorites',
        label: 'Exportar favoritos JSON',
        accelerator: '<Control>e',
        enabled: true,
      },
      { id: 'back-to-list', label: 'Volver al listado', accelerator: 'Escape', enabled: true },
    ];
    this.desktop.setAppMenu(menuItems, {
      'refresh-catalog': () => void this.pokedex.loadCatalogPage(true),
      'toggle-favorites': () => this.toggleFavoritesFilter(),
      'export-favorites': () => {
        this.pokedex.exportFavoritesToDisk();
      },
      'back-to-list': () => void this.backToList(),
    });
    this.desktop.setTray({
      visible: true,
      icon: 'pokedex-app',
      tooltip: 'Pokédex Nacional — Angular 22 Native',
      menuItems,
    });
    this.desktop.registerShortcut('<Control>f', 'focus-search', () => {
      this.searchControl.setValue('');
    });
    this.desktop.registerShortcut('<Control>b', 'toggle-favorites', () => {
      this.toggleFavoritesFilter();
    });
    this.desktop.registerShortcut('Escape', 'back-to-list', () => {
      void this.backToList();
    });

    void this.pokedex.refreshFavorites();
    void this.pokedex.loadCatalogPage(true);
    this.unlistenPopState = this.location.onPopState(() => {
      const match = this.location.pathname.match(/^\/pokemon\/(\d+)$/);
      if (match) {
        const id = Number.parseInt(match[1], 10);
        if (Number.isInteger(id) && id >= 1 && id <= 1025) {
          void this.loadPokemonDetailState(id);
        }
      } else if (this.location.pathname === '/pokemon' || this.location.pathname === '/') {
        void this.pokedex.backToList();
      }
    });
  }

  ngOnDestroy(): void {
    this.unlistenPopState?.();
    this.unlistenPopState = null;
  }

  toggleFavoritesFilter(): void {
    const next = !this.pokedex.favoritesOnly();
    this.favoritesOnlyControl.setValue(next, { emitEvent: false });
    void this.pokedex.setFavoritesOnly(next);
  }

  loadMore(): void {
    void this.pokedex.loadMore();
  }

  private async loadPokemonDetailState(boundedId: number): Promise<void> {
    await this.pokedex.openPokemon(boundedId);
    this.spinIdControl.setValue(boundedId, { emitEvent: false });
    this.noteControl.setValue(this.pokedex.selectedNote(), { emitEvent: false });
  }

  async openPokemon(id: number): Promise<void> {
    const boundedId = Math.max(1, Math.min(1025, Math.trunc(id) || 1));
    await this.router.navigate(['/pokemon', boundedId]);
    await this.loadPokemonDetailState(boundedId);
  }

  async backToList(): Promise<void> {
    await this.router.navigate(['/pokemon']);
    await this.pokedex.backToList();
  }

  previousPokemon(): void {
    const prev = this.pokedex.selectedId() - 1;
    void this.openPokemon(prev >= 1 ? prev : 1);
  }

  nextPokemon(): void {
    const next = this.pokedex.selectedId() + 1;
    void this.openPokemon(next <= 1025 ? next : 1025);
  }

  toggleCurrentFavorite(): void {
    const detail = this.pokedex.selectedDetail();
    if (!detail) {
      return;
    }
    void this.pokedex.toggleFavorite(detail.id, detail.displayName, this.noteControl.getRawValue());
  }

  saveNote(): void {
    const detail = this.pokedex.selectedDetail();
    if (!detail) {
      return;
    }
    void this.pokedex.saveFieldNote(detail.id, this.noteControl.getRawValue());
  }
}

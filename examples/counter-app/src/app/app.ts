import {
  Component,
  effect,
  ElementRef,
  inject,
  OnDestroy,
  OnInit,
  signal,
  untracked,
  viewChild,
  ViewEncapsulation,
} from '@angular/core';
import { form, max, min } from '@angular/forms/signals';
import { NativeDesktopService } from '@ng-desktop/desktop';
import { NATIVE_SIGNAL_FORM_DIRECTIVES } from '@ng-desktop/forms';
import { NATIVE_UI_DIRECTIVES, NativeElementNode } from '@ng-desktop/platform';
import { Router, RouterOutlet } from '@ng-desktop/router';
import { PokedexService } from './core/services/pokedex.service';

@Component({
  selector: 'app-root',
  imports: [NATIVE_SIGNAL_FORM_DIRECTIVES, NATIVE_UI_DIRECTIVES, RouterOutlet],
  templateUrl: './app.html',
  styleUrl: './app.css',
  encapsulation: ViewEncapsulation.None,
})
export class App implements OnInit, OnDestroy {
  protected readonly pokedex = inject(PokedexService);
  private readonly desktop = inject(NativeDesktopService);
  private readonly router = inject(Router);
  private readonly searchEntry = viewChild<ElementRef<NativeElementNode>>('searchEntry');

  protected readonly theme = this.desktop.theme;
  protected readonly windowTitle = signal('Pokédex Nacional');
  protected readonly subtitle = signal(
    'Angular 22 + Signal Forms + @ng-desktop/router + @ng-desktop/server',
  );

  readonly filterModel = signal({
    query: '',
    pokeId: 1,
    favoritesOnly: false,
  });
  readonly filterForm = form(this.filterModel, (path) => {
    min(path.pokeId, 1);
    max(path.pokeId, 1025);
  });

  private lastQuery = '';
  private lastFavoritesOnly = false;
  private lastRoutePokeId = 1;

  constructor() {
    effect(() => {
      const selected = this.pokedex.selectedId();
      if (untracked(() => this.filterModel().pokeId) !== selected) {
        this.lastRoutePokeId = selected;
        this.filterModel.update((prev) => ({ ...prev, pokeId: selected }));
      }
    });

    effect(() => {
      const query = this.filterModel().query;
      if (query !== this.lastQuery) {
        this.lastQuery = query;
        void this.pokedex.setSearchQuery(query);
      }
    });

    effect(() => {
      const rawId = this.filterModel().pokeId;
      const id = Math.trunc(rawId);
      if (Number.isInteger(id) && id >= 1 && id <= 1025 && id !== this.lastRoutePokeId) {
        this.lastRoutePokeId = id;
        if (id !== untracked(() => this.pokedex.selectedId())) {
          void this.openPokemon(id);
        }
      }
    });

    effect(() => {
      const onlyFavorites = this.filterModel().favoritesOnly;
      if (onlyFavorites !== this.lastFavoritesOnly) {
        this.lastFavoritesOnly = onlyFavorites;
        void this.pokedex.setFavoritesOnly(onlyFavorites);
      }
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
      this.focusSearchEntry();
    });
    this.desktop.registerShortcut('<Control>b', 'toggle-favorites', () => {
      this.toggleFavoritesFilter();
    });
    this.desktop.registerShortcut('Escape', 'back-to-list', () => {
      void this.backToList();
    });
    this.desktop.registerShortcut('<Control>t', 'toggle-theme', () => {
      this.toggleTheme();
    });

    void this.pokedex.refreshFavorites();
    void this.pokedex.loadCatalogPage(true);
    if (!this.router.navigated) {
      this.router.initialNavigation();
    }
  }

  ngOnDestroy(): void {
    this.desktop.unregisterShortcut('<Control>f');
    this.desktop.unregisterShortcut('<Control>b');
    this.desktop.unregisterShortcut('Escape');
    this.desktop.unregisterShortcut('<Control>t');
  }

  onBeforeUnload(): void {
    this.desktop.saveWindowGeometry(this.desktop.windowGeometry());
  }

  toggleTheme(): void {
    this.desktop.toggleTheme();
  }

  focusSearchEntry(): void {
    this.searchEntry()?.nativeElement.focus();
  }

  clearSearch(): void {
    this.filterModel.update((prev) => ({ ...prev, query: '' }));
  }

  toggleFavoritesFilter(): void {
    const next = !this.pokedex.favoritesOnly();
    this.lastFavoritesOnly = next;
    this.filterModel.update((prev) => ({ ...prev, favoritesOnly: next }));
    void this.pokedex.setFavoritesOnly(next);
  }

  loadMore(): void {
    void this.pokedex.loadMore();
  }

  openPokemon(id: number): Promise<boolean> {
    const boundedId = Math.max(1, Math.min(1025, Math.trunc(id) || 1));
    return this.router.navigate(['/pokemon', boundedId]);
  }

  backToList(): Promise<boolean> {
    return this.router.navigate(['/pokemon']);
  }
}

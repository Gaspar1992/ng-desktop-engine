import { DOCUMENT } from '@angular/common';
import { TestBed } from '@angular/core/testing';
import { provideRouter, Router } from '@angular/router';
import { compilePlaygroundToNgb } from './core/ngb-playground-compiler';
import { DOCS_THEME_STORAGE_KEY, DocsThemeService } from './core/theme.service';
import { PACKAGES_CATALOG } from './data/packages-catalog.data';
import { PLAYGROUND_PRESETS } from './data/playground-presets.data';
import { PRIMITIVES_CATALOG } from './data/primitives-catalog.data';
import { SearchDialogComponent } from './layout/search-dialog.component';
import { ArchitecturePage } from './pages/architecture.page';
import { GettingStartedPage } from './pages/getting-started.page';
import { HomePage } from './pages/home.page';
import { PackagesPage } from './pages/packages.page';
import { PlaygroundPage } from './pages/playground.page';
import { WidgetsPage } from './pages/widgets.page';

describe('ng-desktop-docs portal', () => {
  beforeEach(() => {
    localStorage.removeItem(DOCS_THEME_STORAGE_KEY);
    let meta = document.querySelector('meta[name="color-scheme"]');
    if (!meta) {
      meta = document.createElement('meta');
      meta.setAttribute('name', 'color-scheme');
      document.head.appendChild(meta);
    }
    meta.setAttribute('content', 'light dark');
  });

  afterEach(() => {
    localStorage.removeItem(DOCS_THEME_STORAGE_KEY);
    vi.restoreAllMocks();
  });

  it('contains all 29 UI primitives with unique IDs and GTK4/WinUI 3/AppKit mappings', () => {
    expect(PRIMITIVES_CATALOG.length).toBe(29);
    const ids = new Set(PRIMITIVES_CATALOG.map((p) => p.id));
    expect(ids.size).toBe(29);

    for (const primitive of PRIMITIVES_CATALOG) {
      expect(primitive.canonicalTag.length).toBeGreaterThan(0);
      expect(primitive.gtk4.length).toBeGreaterThan(0);
      expect(primitive.winui3.length).toBeGreaterThan(0);
      expect(primitive.appkit.length).toBeGreaterThan(0);
    }

    expect(PACKAGES_CATALOG.filter((p) => p.kind === 'library').length).toBe(7);
    expect(PACKAGES_CATALOG.filter((p) => p.kind === 'binary').length).toBe(5);
  });

  it('compiles HTML void tags, HeaderBar children, arrow handlers, and dark CSS cascades cleanly', () => {
    const template = `
      <ng-window title="Demo">
        <ng-header-bar title="Workspace">
          <ng-button label="Save" (click)="count.update(n => n + 1)"></ng-button>
        </ng-header-bar>
        <div class="card accent">
          <hr>
          <img src="icon.png">
          <input placeholder="Query">
          <p>After voids</p>
        </div>
      </ng-window>
    `;
    const css = `
      :root { --panel: #ffffff; }
      @media (prefers-color-scheme: dark) {
        :root { --panel: #10141e; }
      }
      .card {
        background: var(--panel);
        padding: 16px;
        gap: 10px;
      }
      .accent {
        color: #ff4d79;
      }
    `;

    const darkResult = compilePlaygroundToNgb(template, css, 'dark');
    expect(darkResult.warnings).toEqual([]);
    const windowNode = darkResult.roots[0];
    expect(windowNode?.nativeTag).toBe('ng-window');
    expect(windowNode?.children.length).toBe(2);

    const headerBar = windowNode?.children[0];
    expect(headerBar?.nativeTag).toBe('ng-header-bar');
    expect(headerBar?.children.length).toBe(1);
    expect(headerBar?.children[0]?.events).toContain('click');

    // True HTML void tags (`<hr>`, `<img>`, `<input>`) must remain siblings under `ng-box`, never nesting `<p>`.
    const cardBox = windowNode?.children[1];
    expect(cardBox?.nativeTag).toBe('ng-box');
    expect(cardBox?.children.map((c) => c.nativeTag)).toEqual([
      'ng-separator',
      'ng-image',
      'ng-entry',
      'ng-label',
    ]);
    expect(cardBox?.style.background).toBe('#10141e');
    expect(cardBox?.style.color).toBe('#ff4d79');
  });

  it('manages two-state system-first theme toggle without retroactive cleanup on OS change', () => {
    let mediaChangeListener: ((event: MediaQueryListEvent) => void) | undefined;
    const mockMql = {
      matches: false,
      media: '(prefers-color-scheme: dark)',
      addEventListener: vi.fn((_ev: string, listener: (event: MediaQueryListEvent) => void) => {
        mediaChangeListener = listener;
      }),
      removeEventListener: vi.fn(),
    };
    Object.defineProperty(globalThis, 'matchMedia', {
      configurable: true,
      writable: true,
      value: vi.fn(() => mockMql),
    });

    TestBed.configureTestingModule({
      providers: [DocsThemeService],
    });
    const service = TestBed.inject(DocsThemeService);
    const doc = TestBed.inject(DOCUMENT);

    expect(service.effectiveTheme()).toBe('light');
    expect(localStorage.getItem(DOCS_THEME_STORAGE_KEY)).toBeNull();
    expect(doc.querySelector('meta[name="color-scheme"]')?.getAttribute('content')).toBe(
      'light dark',
    );

    // Diverge from light OS preference -> persists explicit 'dark'
    service.toggle();
    expect(service.effectiveTheme()).toBe('dark');
    expect(localStorage.getItem(DOCS_THEME_STORAGE_KEY)).toBe('dark');
    expect(doc.querySelector('meta[name="color-scheme"]')?.getAttribute('content')).toBe('dark');

    // OS switches to dark while pinned to 'dark': NEVER retroactively clear localStorage
    mockMql.matches = true;
    mediaChangeListener?.({ matches: true } as MediaQueryListEvent);
    expect(localStorage.getItem(DOCS_THEME_STORAGE_KEY)).toBe('dark');

    // User toggles to light (which now diverges from dark OS) -> persists 'light'
    service.toggle();
    expect(service.effectiveTheme()).toBe('light');
    expect(localStorage.getItem(DOCS_THEME_STORAGE_KEY)).toBe('light');

    // User toggles back to dark (matches current dark OS) -> clears localStorage
    service.toggle();
    expect(service.effectiveTheme()).toBe('dark');
    expect(localStorage.getItem(DOCS_THEME_STORAGE_KEY)).toBeNull();
    expect(doc.querySelector('meta[name="color-scheme"]')?.getAttribute('content')).toBe(
      'light dark',
    );
  });

  it('filters all 29 primitives reactively in WidgetsPage', () => {
    TestBed.configureTestingModule({
      imports: [WidgetsPage],
    });
    const fixture = TestBed.createComponent(WidgetsPage);
    fixture.detectChanges();

    const rows = fixture.nativeElement.querySelectorAll('tbody tr');
    expect(rows.length).toBe(29);

    fixture.componentInstance.activeCategory.set('Form Inputs');
    fixture.detectChanges();
    expect(fixture.componentInstance.filteredPrimitives().length).toBe(9);

    fixture.componentInstance.activeCategory.set('All');
    fixture.componentInstance.searchQuery.set('AdwDialog');
    fixture.detectChanges();
    expect(fixture.componentInstance.filteredPrimitives().length).toBe(1);
    expect(fixture.componentInstance.filteredPrimitives()[0]?.name).toBe('Dialog');
  });

  it('loads presets, pins and unpins theme preview in PlaygroundPage', () => {
    TestBed.configureTestingModule({
      imports: [PlaygroundPage],
    });
    const fixture = TestBed.createComponent(PlaygroundPage);
    fixture.detectChanges();

    for (const preset of PLAYGROUND_PRESETS) {
      fixture.componentInstance.loadPreset(preset);
      fixture.detectChanges();
      expect(fixture.componentInstance.compilation().warnings).toEqual([]);
      expect(fixture.componentInstance.compilation().nodeCount).toBeGreaterThan(0);
    }

    fixture.componentInstance.setPreviewTheme('dark');
    expect(fixture.componentInstance.previewTheme()).toBe('dark');
    fixture.componentInstance.setPreviewTheme(null);
    expect(fixture.componentInstance.pinnedPreviewTheme()).toBeNull();
  });

  it('renders ArchitecturePage, PackagesPage, GettingStartedPage and navigates via SearchDialogComponent', () => {
    TestBed.configureTestingModule({
      imports: [
        HomePage,
        SearchDialogComponent,
        ArchitecturePage,
        PackagesPage,
        GettingStartedPage,
      ],
      providers: [provideRouter([])],
    });

    const homeFixture = TestBed.createComponent(HomePage);
    homeFixture.detectChanges();
    expect(homeFixture.nativeElement.textContent).toContain('Angular 22 Native Desktop Apps');

    const archFixture = TestBed.createComponent(ArchitecturePage);
    archFixture.detectChanges();
    expect(archFixture.nativeElement.querySelectorAll('[role="tab"]').length).toBe(6);

    const pkgFixture = TestBed.createComponent(PackagesPage);
    pkgFixture.componentInstance.kindFilter.set('binary');
    pkgFixture.detectChanges();
    expect(pkgFixture.componentInstance.filteredPackages().length).toBe(5);

    const gsFixture = TestBed.createComponent(GettingStartedPage);
    gsFixture.detectChanges();
    expect(gsFixture.nativeElement.textContent).toContain('provideNativeHttpClient');

    const router = TestBed.inject(Router);
    const navSpy = vi.spyOn(router, 'navigate').mockResolvedValue(true);
    const dialogFixture = TestBed.createComponent(SearchDialogComponent);
    dialogFixture.detectChanges();
    dialogFixture.componentInstance.query.set('Play');
    dialogFixture.detectChanges();
    const firstResult = dialogFixture.componentInstance.results()[0];
    expect(firstResult?.route).toBe('/playground');
    if (firstResult) {
      dialogFixture.componentInstance.selectEntry(firstResult);
    }
    expect(navSpy).toHaveBeenCalledWith(['/playground'], undefined);
  });
});

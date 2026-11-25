import { DOCUMENT } from '@angular/common';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { compilePlaygroundToNgb } from './core/ngb-playground-compiler';
import { DOCS_THEME_STORAGE_KEY, DocsThemeService } from './core/theme.service';
import { PACKAGES_CATALOG } from './data/packages-catalog.data';
import { PLAYGROUND_PRESETS } from './data/playground-presets.data';
import { PRIMITIVES_CATALOG } from './data/primitives-catalog.data';
import { SearchDialogComponent } from './layout/search-dialog.component';
import { HomePage } from './pages/home.page';
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

  it('compiles templates and CSS variables/theme rules into .ngb instructions in browser', () => {
    const template = `
      <ng-window title="Demo">
        <div class="card">
          <p>Hello Desktop</p>
          <ng-button label="Save" (click)="save()"></ng-button>
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
    `;

    const darkResult = compilePlaygroundToNgb(template, css, 'dark');
    expect(darkResult.warnings).toEqual([]);
    expect(darkResult.nodeCount).toBe(4);
    expect(darkResult.styleRuleCount).toBeGreaterThanOrEqual(2);
    expect(
      darkResult.instructions.some(
        (inst) => inst.opcode === 'CreateNode' && inst.summary.includes('<ng-box>'),
      ),
    ).toBe(true);
    expect(
      darkResult.instructions.some(
        (inst) => inst.opcode === 'CreateNode' && inst.summary.includes('<ng-label>'),
      ),
    ).toBe(true);
    expect(
      darkResult.instructions.some(
        (inst) => inst.opcode === 'AttachEvent' && inst.summary.includes('on(click)'),
      ),
    ).toBe(true);

    const cardNode = darkResult.roots[0]?.children[0];
    expect(cardNode?.nativeTag).toBe('ng-box');
    expect(cardNode?.style.background).toBe('#10141e');
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

  it('loads presets and updates compiled output reactively in PlaygroundPage', () => {
    TestBed.configureTestingModule({
      imports: [PlaygroundPage],
    });
    const fixture = TestBed.createComponent(PlaygroundPage);
    fixture.detectChanges();

    const secondPreset = PLAYGROUND_PRESETS[2]!;
    fixture.componentInstance.loadPreset(secondPreset);
    fixture.detectChanges();

    expect(fixture.componentInstance.activePresetId()).toBe(secondPreset.id);
    expect(fixture.componentInstance.compilation().nodeCount).toBeGreaterThan(0);

    fixture.componentInstance.setPreviewTheme('dark');
    fixture.detectChanges();
    expect(fixture.componentInstance.previewTheme()).toBe('dark');
  });

  it('renders HomePage hero and filters results in SearchDialogComponent', () => {
    TestBed.configureTestingModule({
      imports: [HomePage, SearchDialogComponent],
      providers: [provideRouter([])],
    });

    const homeFixture = TestBed.createComponent(HomePage);
    homeFixture.detectChanges();
    expect(homeFixture.nativeElement.textContent).toContain('Angular 22 Native Desktop Apps');

    const dialogFixture = TestBed.createComponent(SearchDialogComponent);
    dialogFixture.detectChanges();
    dialogFixture.componentInstance.query.set('Play');
    dialogFixture.detectChanges();
    expect(
      dialogFixture.componentInstance.results().some((entry) => entry.route === '/playground'),
    ).toBe(true);
  });
});

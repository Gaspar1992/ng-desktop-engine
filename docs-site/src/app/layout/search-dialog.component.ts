import {
  afterNextRender,
  Component,
  computed,
  DestroyRef,
  ElementRef,
  inject,
  signal,
  viewChild,
} from '@angular/core';
import { Router } from '@angular/router';
import { PACKAGES_CATALOG } from '../data/packages-catalog.data';
import { PRIMITIVES_CATALOG } from '../data/primitives-catalog.data';

export interface SearchEntry {
  readonly id: string;
  readonly kind: 'Page' | 'Primitive' | 'Package';
  readonly title: string;
  readonly subtitle: string;
  readonly route: string;
  readonly queryParams?: Readonly<Record<string, string>>;
}

const PAGE_ENTRIES: readonly SearchEntry[] = [
  {
    id: 'page-home',
    kind: 'Page',
    title: 'Overview & Comparison',
    subtitle: 'Zero-WebView Angular 22 native desktop runtime overview',
    route: '/',
  },
  {
    id: 'page-getting-started',
    kind: 'Page',
    title: 'Getting Started & CLI',
    subtitle: 'Install with ng add @ng-desktop/builders and package native installers',
    route: '/getting-started',
  },
  {
    id: 'page-architecture',
    kind: 'Page',
    title: 'Engine Architecture',
    subtitle: 'Generational Arena, CommandQueue, JSC/QuickJS-NG, Layout Solver',
    route: '/architecture',
  },
  {
    id: 'page-widgets',
    kind: 'Page',
    title: '29 Native Primitives Explorer',
    subtitle: 'GTK4, WinUI 3, and AppKit native widget mapping table',
    route: '/widgets',
  },
  {
    id: 'page-playground',
    kind: 'Page',
    title: 'Live .ngb Template & CSS Playground',
    subtitle: 'Compile Angular templates and CSS variables in-browser',
    route: '/playground',
  },
  {
    id: 'page-packages',
    kind: 'Page',
    title: 'Packages & Prebuilt Engine Binaries',
    subtitle: 'All 7 @ng-desktop/* npm packages and 5 OS engine targets',
    route: '/packages',
  },
  {
    id: 'page-pokedex',
    kind: 'Page',
    title: 'Pokédex Desktop Case Study',
    subtitle: '1,025-item VirtualGrid, Signal Forms, and local SQLite routes',
    route: '/showcase/pokedex',
  },
];

const ALL_SEARCH_ENTRIES: readonly SearchEntry[] = [
  ...PAGE_ENTRIES,
  ...PRIMITIVES_CATALOG.map((primitive): SearchEntry => ({
    id: `prim-${primitive.id}`,
    kind: 'Primitive',
    title: `${primitive.name} (<${primitive.canonicalTag}>)`,
    subtitle: `${primitive.gtk4} · ${primitive.winui3} · ${primitive.appkit}`,
    route: '/widgets',
    queryParams: { q: primitive.name },
  })),
  ...PACKAGES_CATALOG.map((pkg): SearchEntry => ({
    id: `pkg-${pkg.id}`,
    kind: 'Package',
    title: pkg.name,
    subtitle: pkg.summary,
    route: '/packages',
    queryParams: { q: pkg.name },
  })),
];

@Component({
  selector: 'docs-search-dialog',
  template: `
    <dialog
      #dialogEl
      closedby="any"
      aria-labelledby="docs-search-title"
      class="search-dialog"
      (close)="onClosed()"
    >
      <div class="dialog-head">
        <h2 id="docs-search-title">Quick Search</h2>
        <button
          type="button"
          class="btn btn-ghost close-btn"
          aria-label="Close search dialog"
          (click)="close()"
        >
          Close
        </button>
      </div>

      <div class="search-input-wrap">
        <label for="docs-search-input" class="sr-only">Search docs, primitives, and packages</label>
        <input
          #inputEl
          id="docs-search-input"
          type="search"
          class="search-input"
          placeholder="Search 29 primitives, packages, or guides..."
          [value]="query()"
          (input)="query.set($any($event.target).value)"
          (keydown.enter)="navigateFirst()"
        />
      </div>

      <ul class="results-list" aria-label="Search results">
        @for (entry of results(); track entry.id) {
          <li>
            <button type="button" class="result-item" (click)="selectEntry(entry)">
              <span class="result-kind">{{ entry.kind }}</span>
              <span class="result-copy">
                <strong>{{ entry.title }}</strong>
                <span class="result-sub">{{ entry.subtitle }}</span>
              </span>
            </button>
          </li>
        } @empty {
          <li class="empty-state">No matching documentation entries found for "{{ query() }}".</li>
        }
      </ul>
    </dialog>
  `,
  styles: `
    .search-dialog {
      width: min(640px, calc(100vw - 2rem));
      max-height: min(76vh, 620px);
      margin: auto;
      padding: 0;
      border: 1px solid var(--border-strong);
      border-radius: 16px;
      background: var(--bg-elevated);
      color: var(--text);
      box-shadow: var(--shadow-md);
      overflow: hidden;
    }

    .search-dialog::backdrop {
      background: rgba(10, 12, 18, 0.58);
      backdrop-filter: blur(4px);
    }

    .dialog-head {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0.9rem 1.1rem;
      border-bottom: 1px solid var(--border);
    }

    .dialog-head h2 {
      margin: 0;
      font-size: 1rem;
    }

    .search-input-wrap {
      padding: 0.85rem 1.1rem;
      border-bottom: 1px solid var(--border);
    }

    .search-input {
      width: 100%;
      padding: 0.65rem 0.85rem;
      border-radius: 10px;
      border: 1px solid var(--border-strong);
      background: var(--surface);
      color: var(--text);
      font: inherit;
    }

    .results-list {
      list-style: none;
      margin: 0;
      padding: 0.55rem;
      max-height: 46vh;
      overflow-y: auto;
      display: grid;
      gap: 0.35rem;
    }

    .result-item {
      width: 100%;
      display: flex;
      align-items: flex-start;
      gap: 0.75rem;
      text-align: left;
      padding: 0.65rem 0.8rem;
      border-radius: 10px;
      border: 1px solid transparent;
      background: transparent;
      color: var(--text);
      font: inherit;
      cursor: pointer;
    }

    .result-item:hover,
    .result-item:focus-visible {
      background: var(--surface);
      border-color: var(--border);
    }

    .result-kind {
      flex-shrink: 0;
      font-size: 0.72rem;
      font-weight: 700;
      text-transform: uppercase;
      padding: 0.16rem 0.5rem;
      border-radius: 999px;
      background: var(--accent-soft);
      color: var(--accent);
    }

    .result-copy {
      display: grid;
      gap: 0.12rem;
    }

    .result-sub {
      font-size: 0.84rem;
      color: var(--text-muted);
    }

    .empty-state {
      padding: 1.5rem 1rem;
      text-align: center;
      color: var(--text-muted);
    }
  `,
})
export class SearchDialogComponent {
  private readonly router = inject(Router);
  private readonly destroyRef = inject(DestroyRef);

  private readonly dialogRef = viewChild.required<ElementRef<HTMLDialogElement>>('dialogEl');
  private readonly inputRef = viewChild.required<ElementRef<HTMLInputElement>>('inputEl');

  readonly query = signal<string>('');

  readonly results = computed<readonly SearchEntry[]>(() => {
    const q = this.query().trim().toLowerCase();
    if (!q) {
      return ALL_SEARCH_ENTRIES.slice(0, 12);
    }
    return ALL_SEARCH_ENTRIES.filter(
      (entry) =>
        entry.title.toLowerCase().includes(q) ||
        entry.subtitle.toLowerCase().includes(q) ||
        entry.kind.toLowerCase().includes(q),
    ).slice(0, 14);
  });

  constructor() {
    afterNextRender(() => {
      const dialogEl = this.dialogRef().nativeElement;

      // Light-dismiss fallback for browsers without native `closedBy` support.
      const handleBackdropClick = (event: MouseEvent): void => {
        if ('closedBy' in HTMLDialogElement.prototype) {
          return;
        }
        if (event.target !== dialogEl) {
          return;
        }
        const rect = dialogEl.getBoundingClientRect();
        const outside =
          event.clientX < rect.left ||
          event.clientX > rect.right ||
          event.clientY < rect.top ||
          event.clientY > rect.bottom;
        if (outside) {
          this.close();
        }
      };

      dialogEl.addEventListener('click', handleBackdropClick);
      this.destroyRef.onDestroy(() => dialogEl.removeEventListener('click', handleBackdropClick));
    });
  }

  open(): void {
    const dialogEl = this.dialogRef().nativeElement as HTMLDialogElement & {
      showModal?: () => void;
    };
    if (!dialogEl.open) {
      if (typeof dialogEl.showModal === 'function') {
        dialogEl.showModal();
      } else {
        dialogEl.setAttribute('open', '');
      }
    }
    this.inputRef().nativeElement.focus();
  }

  close(): void {
    const dialogEl = this.dialogRef().nativeElement as HTMLDialogElement & {
      close?: () => void;
    };
    if (typeof dialogEl.close === 'function') {
      dialogEl.close();
    } else {
      dialogEl.removeAttribute('open');
    }
  }

  onClosed(): void {
    this.query.set('');
  }

  navigateFirst(): void {
    const first = this.results()[0];
    if (first) {
      this.selectEntry(first);
    }
  }

  selectEntry(entry: SearchEntry): void {
    this.close();
    void this.router.navigate(
      [entry.route],
      entry.queryParams ? { queryParams: entry.queryParams } : undefined,
    );
  }
}

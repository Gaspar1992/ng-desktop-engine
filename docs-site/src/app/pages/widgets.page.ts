import { Component, computed, effect, input, signal } from '@angular/core';
import {
  INTERNAL_ANCHOR_NOTE,
  PRIMITIVE_CATEGORIES,
  PRIMITIVES_CATALOG,
  type PrimitiveCategory,
  type PrimitiveSpec,
} from '../data/primitives-catalog.data';

@Component({
  selector: 'docs-widgets-page',
  template: `
    <div class="docs-container widgets-shell">
      <header class="page-head">
        <span class="eyebrow">Cross-Platform Widget Matrix</span>
        <h1>29 Native UI Primitives</h1>
        <p>
          Authoritative mapping between Angular custom element selectors and native controls in
          <code>src/engine/widget.rs</code> across GTK4 / Libadwaita, WinUI 3, and AppKit Cocoa.
        </p>
      </header>

      <section class="filter-bar card" aria-label="Filter primitives">
        <div class="search-field">
          <label for="primitive-filter-input">Filter by name, tag, or native class</label>
          <input
            id="primitive-filter-input"
            type="search"
            class="filter-input"
            placeholder="e.g. Window, AdwDialog, NSButton, slider..."
            [value]="searchQuery()"
            (input)="onSearchInput($event)"
          />
        </div>

        <div class="category-pills" role="group" aria-label="Category filter">
          <button
            type="button"
            class="btn"
            [class.btn-primary]="activeCategory() === 'All'"
            (click)="activeCategory.set('All')"
          >
            All ({{ allPrimitives.length }})
          </button>
          @for (cat of categories; track cat) {
            <button
              type="button"
              class="btn"
              [class.btn-primary]="activeCategory() === cat"
              (click)="activeCategory.set(cat)"
            >
              {{ cat }}
            </button>
          }
        </div>
      </section>

      <div class="explorer-layout">
        <div class="table-wrapper" tabindex="0" role="region" aria-labelledby="widgets-table-title">
          <table class="docs-table">
            <caption id="widgets-table-title">
              Showing
              {{
                filteredPrimitives().length
              }}
              of
              {{
                allPrimitives.length
              }}
              native UI primitives
            </caption>
            <thead>
              <tr>
                <th scope="col">Primitive</th>
                <th scope="col">Angular Tag</th>
                <th scope="col">Linux GTK4</th>
                <th scope="col">Windows WinUI 3</th>
                <th scope="col">macOS AppKit</th>
              </tr>
            </thead>
            <tbody>
              @for (item of filteredPrimitives(); track item.id) {
                <tr [class.is-selected]="selectedId() === item.id">
                  <th scope="row">
                    <button
                      type="button"
                      class="row-select-btn"
                      [attr.aria-pressed]="selectedId() === item.id"
                      (click)="selectedId.set(item.id)"
                    >
                      {{ item.name }}
                    </button>
                  </th>
                  <td data-label="Angular Tag">
                    <code>&lt;{{ item.canonicalTag }}&gt;</code>
                  </td>
                  <td data-label="Linux GTK4">
                    <code>{{ item.gtk4 }}</code>
                  </td>
                  <td data-label="Windows WinUI 3">
                    <code>{{ item.winui3 }}</code>
                  </td>
                  <td data-label="macOS AppKit">
                    <code>{{ item.appkit }}</code>
                  </td>
                </tr>
              } @empty {
                <tr>
                  <td colspan="5" class="empty-cell">No primitives match "{{ searchQuery() }}".</td>
                </tr>
              }
            </tbody>
          </table>
        </div>

        @if (selectedPrimitive(); as spec) {
          <aside class="card inspector-card" aria-label="Selected primitive details">
            <div class="inspector-head">
              <div>
                <span class="eyebrow">{{ spec.category }}</span>
                <h2>{{ spec.name }}</h2>
              </div>
              <button type="button" class="btn btn-ghost" (click)="copySnippet(spec)">
                {{ copiedId() === spec.id ? 'Copied' : 'Copy Snippet' }}
              </button>
            </div>

            <p>{{ spec.summary }}</p>

            <dl class="spec-meta">
              <div>
                <dt>Canonical Selector</dt>
                <dd>
                  <code>&lt;{{ spec.canonicalTag }}&gt;</code>
                </dd>
              </div>
              <div>
                <dt>Accepted Aliases</dt>
                <dd>
                  @for (alias of spec.aliases; track $index) {
                    <code>&lt;{{ alias }}&gt;</code>
                  }
                </dd>
              </div>
              <div>
                <dt>Attributes</dt>
                <dd>
                  @for (attr of spec.attributes; track $index) {
                    <code>{{ attr }}</code>
                  }
                </dd>
              </div>
              <div>
                <dt>Events</dt>
                <dd>
                  @if (spec.events.length === 0) {
                    <span class="muted">Structural / passive</span>
                  } @else {
                    @for (ev of spec.events; track $index) {
                      <code>({{ ev }})</code>
                    }
                  }
                </dd>
              </div>
            </dl>

            <pre><code>{{ spec.snippet }}</code></pre>
          </aside>
        }
      </div>

      <p class="anchor-note card">{{ anchorNote }}</p>
    </div>
  `,
  styles: `
    .widgets-shell {
      display: grid;
      gap: 1.5rem;
    }

    .page-head {
      display: grid;
      gap: 0.5rem;
      justify-items: start;
    }

    .page-head h1,
    .page-head p {
      margin: 0;
    }

    .page-head p {
      color: var(--text-muted);
      max-width: 70ch;
    }

    .filter-bar {
      display: grid;
      gap: 1rem;
    }

    .search-field {
      display: grid;
      gap: 0.4rem;
    }

    .search-field label {
      font-weight: 700;
      font-size: 0.9rem;
    }

    .filter-input {
      width: 100%;
      padding: 0.65rem 0.9rem;
      border-radius: 10px;
      border: 1px solid var(--border-strong);
      background: var(--surface);
      color: var(--text);
      font: inherit;
    }

    .category-pills {
      display: flex;
      flex-wrap: wrap;
      gap: 0.5rem;
    }

    .explorer-layout {
      display: grid;
      gap: 1.25rem;
      align-items: start;
    }

    @media (min-width: 1020px) {
      .explorer-layout {
        grid-template-columns: minmax(0, 1.45fr) minmax(320px, 1fr);
      }
    }

    .row-select-btn {
      padding: 0.2rem 0.4rem;
      border-radius: 6px;
      border: 1px solid transparent;
      background: transparent;
      color: inherit;
      font: inherit;
      font-weight: 700;
      cursor: pointer;
      text-align: left;
    }

    .row-select-btn:hover {
      color: var(--accent);
      text-decoration: underline;
    }

    tr.is-selected {
      background: var(--accent-soft);
    }

    .empty-cell {
      text-align: center;
      color: var(--text-muted);
      padding: 2rem;
    }

    .inspector-card {
      position: sticky;
      top: 5rem;
      display: grid;
      gap: 0.9rem;
    }

    .inspector-head {
      display: flex;
      align-items: flex-start;
      justify-content: space-between;
      gap: 0.75rem;
    }

    .inspector-head h2 {
      margin: 0.35rem 0 0;
    }

    .inspector-card p {
      margin: 0;
      color: var(--text-muted);
    }

    .spec-meta {
      margin: 0;
      display: grid;
      gap: 0.65rem;
    }

    .spec-meta dt {
      font-size: 0.78rem;
      font-weight: 700;
      text-transform: uppercase;
      color: var(--text-muted);
      margin-bottom: 0.25rem;
    }

    .spec-meta dd {
      margin: 0;
      display: flex;
      flex-wrap: wrap;
      gap: 0.35rem;
    }

    .muted {
      color: var(--text-muted);
      font-size: 0.9rem;
    }

    .anchor-note {
      margin: 0;
      color: var(--text-muted);
      font-size: 0.92rem;
    }
  `,
})
export class WidgetsPage {
  readonly allPrimitives = PRIMITIVES_CATALOG;
  readonly categories = PRIMITIVE_CATEGORIES;
  readonly anchorNote = INTERNAL_ANCHOR_NOTE;

  readonly q = input<string | undefined>(undefined);
  readonly searchQuery = signal<string>('');
  readonly activeCategory = signal<PrimitiveCategory | 'All'>('All');
  readonly selectedId = signal<string>(PRIMITIVES_CATALOG[0]?.id ?? 'window');
  readonly copiedId = signal<string | null>(null);

  constructor() {
    effect(() => {
      const incoming = this.q();
      if (typeof incoming === 'string') {
        this.searchQuery.set(incoming);
      }
    });
  }

  readonly filteredPrimitives = computed<readonly PrimitiveSpec[]>(() => {
    const query = this.searchQuery().trim().toLowerCase();
    const category = this.activeCategory();

    return this.allPrimitives.filter((item) => {
      if (category !== 'All' && item.category !== category) {
        return false;
      }
      if (!query) {
        return true;
      }
      return (
        item.name.toLowerCase().includes(query) ||
        item.canonicalTag.toLowerCase().includes(query) ||
        item.aliases.some((alias) => alias.toLowerCase().includes(query)) ||
        item.gtk4.toLowerCase().includes(query) ||
        item.winui3.toLowerCase().includes(query) ||
        item.appkit.toLowerCase().includes(query)
      );
    });
  });

  readonly selectedPrimitive = computed<PrimitiveSpec | undefined>(() => {
    const filtered = this.filteredPrimitives();
    return filtered.find((item) => item.id === this.selectedId()) ?? filtered[0];
  });

  onSearchInput(event: Event): void {
    const target = event.target as HTMLInputElement | null;
    this.searchQuery.set(target?.value ?? '');
  }

  async copySnippet(spec: PrimitiveSpec): Promise<void> {
    try {
      await globalThis.navigator?.clipboard?.writeText(spec.snippet);
    } catch {
      // Ignore clipboard denials cleanly.
    }
    this.copiedId.set(spec.id);
    setTimeout(() => {
      if (this.copiedId() === spec.id) {
        this.copiedId.set(null);
      }
    }, 1800);
  }
}

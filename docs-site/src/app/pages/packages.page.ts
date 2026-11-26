import { Component, computed, effect, input, signal } from '@angular/core';
import { PACKAGES_CATALOG, type PackageSpec } from '../data/packages-catalog.data';

@Component({
  selector: 'docs-packages-page',
  template: `
    <div class="docs-container packages-shell">
      <header class="page-head">
        <span class="eyebrow">npm OIDC Registry · v0.7.0</span>
        <h1>&#64;ng-desktop/* Packages &amp; Prebuilt Engine Binaries</h1>
        <p>
          Seven modular Angular &amp; tooling libraries paired with five prebuilt platform engine
          packages published via OIDC Trusted Publishers.
        </p>
      </header>

      <section class="api-links-grid" aria-label="Generated API documentation">
        <a class="card api-card" href="api-ts/">
          <span class="eyebrow">TypeScript API</span>
          <h2>&#64;ng-desktop/* Reference</h2>
          <p>Browse generated interfaces, signals, directives, and CLI helpers under /api-ts/.</p>
        </a>
        <a class="card api-card" href="api-rust/">
          <span class="eyebrow">Rust Crate Docs</span>
          <h2>ng_native_engine rustdoc</h2>
          <p>
            Inspect Rust module graphs, arena safety invariants, and FFI backends under /api-rust/.
          </p>
        </a>
      </section>

      <section class="card filter-bar">
        <div class="filter-controls">
          <label for="pkg-filter-input" class="sr-only">Filter packages</label>
          <input
            id="pkg-filter-input"
            type="search"
            class="filter-input"
            placeholder="Filter packages (e.g. router, forms, sqlite, darwin)..."
            [value]="query()"
            (input)="onQueryInput($event)"
          />
          <div class="kind-buttons" role="group" aria-label="Package kind filter">
            <button
              type="button"
              class="btn"
              [class.btn-primary]="kindFilter() === 'all'"
              [attr.aria-pressed]="kindFilter() === 'all'"
              (click)="kindFilter.set('all')"
            >
              All ({{ allPackages.length }})
            </button>
            <button
              type="button"
              class="btn"
              [class.btn-primary]="kindFilter() === 'library'"
              [attr.aria-pressed]="kindFilter() === 'library'"
              (click)="kindFilter.set('library')"
            >
              Angular Libraries (7)
            </button>
            <button
              type="button"
              class="btn"
              [class.btn-primary]="kindFilter() === 'binary'"
              [attr.aria-pressed]="kindFilter() === 'binary'"
              (click)="kindFilter.set('binary')"
            >
              OS Binaries (5)
            </button>
          </div>
        </div>
      </section>

      <section class="packages-grid" aria-label="Package directory">
        @for (pkg of filteredPackages(); track pkg.id) {
          <article class="card pkg-card">
            <div class="pkg-head">
              <div>
                <span class="pkg-kind">{{
                  pkg.kind === 'library' ? 'Library' : 'Native Binary'
                }}</span>
                <h2>{{ pkg.name }}</h2>
              </div>
              <span class="pkg-version">v{{ pkg.version }}</span>
            </div>

            <p>{{ pkg.summary }}</p>

            <div class="exports-row">
              @for (exp of pkg.exportsList; track exp) {
                <code>{{ exp }}</code>
              }
            </div>

            <pre><code>{{ pkg.snippet }}</code></pre>
          </article>
        }
      </section>
    </div>
  `,
  styles: `
    .packages-shell {
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

    .api-links-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
      gap: 1rem;
    }

    .api-card {
      text-decoration: none;
      color: var(--text);
      display: grid;
      gap: 0.45rem;
      justify-items: start;
    }

    .api-card:hover {
      border-color: var(--accent);
    }

    .api-card h2,
    .api-card p {
      margin: 0;
    }

    .api-card p {
      color: var(--text-muted);
      font-size: 0.92rem;
    }

    .filter-controls {
      display: flex;
      flex-wrap: wrap;
      gap: 0.75rem;
      align-items: center;
      justify-content: space-between;
    }

    .filter-input {
      flex: 1 1 280px;
      padding: 0.62rem 0.88rem;
      border-radius: 10px;
      border: 1px solid var(--border-strong);
      background: var(--surface);
      color: var(--text);
      font: inherit;
    }

    .kind-buttons {
      display: flex;
      flex-wrap: wrap;
      gap: 0.5rem;
    }

    .packages-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
      gap: 1rem;
    }

    .pkg-card {
      display: grid;
      gap: 0.75rem;
      align-content: start;
    }

    .pkg-head {
      display: flex;
      align-items: flex-start;
      justify-content: space-between;
      gap: 0.75rem;
    }

    .pkg-head h2 {
      margin: 0.25rem 0 0;
      font-size: 1.12rem;
      word-break: break-word;
    }

    .pkg-kind {
      font-size: 0.72rem;
      font-weight: 700;
      text-transform: uppercase;
      color: var(--accent);
    }

    .pkg-version {
      font-family: var(--font-mono);
      font-size: 0.8rem;
      padding: 0.16rem 0.5rem;
      border-radius: 999px;
      background: var(--surface-muted);
      border: 1px solid var(--border);
    }

    .pkg-card p {
      margin: 0;
      color: var(--text-muted);
      font-size: 0.92rem;
    }

    .exports-row {
      display: flex;
      flex-wrap: wrap;
      gap: 0.35rem;
    }
  `,
})
export class PackagesPage {
  readonly allPackages = PACKAGES_CATALOG;

  readonly q = input<string | undefined>(undefined);
  readonly query = signal<string>('');
  readonly kindFilter = signal<'all' | 'library' | 'binary'>('all');

  constructor() {
    effect(() => {
      const incoming = this.q();
      if (typeof incoming === 'string') {
        this.query.set(incoming);
      }
    });
  }

  readonly filteredPackages = computed<readonly PackageSpec[]>(() => {
    const q = this.query().trim().toLowerCase();
    const kind = this.kindFilter();
    return this.allPackages.filter((pkg) => {
      if (kind !== 'all' && pkg.kind !== kind) {
        return false;
      }
      if (!q) {
        return true;
      }
      return (
        pkg.name.toLowerCase().includes(q) ||
        pkg.summary.toLowerCase().includes(q) ||
        pkg.exportsList.some((exp) => exp.toLowerCase().includes(q))
      );
    });
  });

  onQueryInput(event: Event): void {
    const target = event.target as HTMLInputElement | null;
    this.query.set(target?.value ?? '');
  }
}

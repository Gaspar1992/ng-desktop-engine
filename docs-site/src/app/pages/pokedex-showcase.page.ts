import { Component } from '@angular/core';
import { RouterLink } from '@angular/router';

@Component({
  selector: 'docs-pokedex-showcase-page',
  imports: [RouterLink],
  template: `
    <div class="docs-container showcase-shell">
      <header class="page-head">
        <span class="eyebrow">Reference Application · examples/counter-app</span>
        <h1>Pokédex Desktop Showcase</h1>
        <p>
          An end-to-end Angular 22 desktop application exercising 1,025-item 2D virtualization,
          Signal Forms, local SQLite server routes, reactive dark/light CSS tokens, and OS-native
          packaging.
        </p>
      </header>

      <section class="highlights-grid" aria-label="Pokédex architectural highlights">
        <article class="card">
          <h2>1. Responsive 2D Virtual Grid</h2>
          <p>
            <code>&lt;ng-virtual-grid&gt;</code> uses an <code>O(log n)</code> Fenwick prefix-sum
            tree so scrolling thousands of Pokémon entries binds only visible native cells inside
            <code>&lt;ng-scroll&gt;</code>.
          </p>
        </article>
        <article class="card">
          <h2>2. Signal Forms &amp; Local SQLite Routes</h2>
          <p>
            Trainer notes in <code>PokemonDetailView</code> bind directly via Angular 22
            <code>form()</code> + <code>[formField]</code> and persist offline through
            <code>&#64;ng-desktop/server</code> SQLite route handlers.
          </p>
        </article>
        <article class="card">
          <h2>3. Single Master-Detail Router Outlet</h2>
          <p>
            A single <code>&lt;ng-paned&gt;</code> master-detail shell coordinates list filtering,
            <code>Ctrl+F</code> search focus tokens, and <code>NativeStackRouteReuseStrategy</code>
            without duplicate outlets or nested scrollports.
          </p>
        </article>
        <article class="card">
          <h2>4. Automatic Light/Dark Theme Tokens</h2>
          <p>
            Shared <code>:root</code> CSS variables rewrite dynamically on <code>SetTheme</code>,
            updating cards, badges, headers, and borders live across GTK4, WinUI 3, and AppKit.
          </p>
        </article>
      </section>

      <section class="card code-walkthrough">
        <h2>Core Master-Detail Composition</h2>
        <pre><code>{{ sampleTemplate }}</code></pre>
        <div class="cta-row">
          <a routerLink="/playground" class="btn btn-primary">Try Pokédex Preset in Playground</a>
          <a routerLink="/widgets" class="btn">Inspect Paned &amp; Virtual Grid Primitives</a>
        </div>
      </section>
    </div>
  `,
  styles: `
    .showcase-shell {
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

    .highlights-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
      gap: 1rem;
    }

    .highlights-grid h2 {
      margin: 0 0 0.45rem;
      font-size: 1.1rem;
    }

    .highlights-grid p {
      margin: 0;
      color: var(--text-muted);
      font-size: 0.94rem;
    }

    .code-walkthrough {
      display: grid;
      gap: 0.9rem;
    }

    .code-walkthrough h2 {
      margin: 0;
      font-size: 1.15rem;
    }

    .cta-row {
      display: flex;
      flex-wrap: wrap;
      gap: 0.75rem;
    }
  `,
})
export class PokedexShowcasePage {
  readonly sampleTemplate = `<ng-window title="Pokédex Desktop" [default-width]="1080" [default-height]="720" [theme]="theme()">
  <ng-header-bar title="Pokédex — Angular 22 Native">
    <ng-button [label]="themeLabel()" (click)="toggleTheme()"></ng-button>
  </ng-header-bar>
  <ng-paned orientation="horizontal" [position]="460">
    <app-catalog-view />
    <router-outlet />
  </ng-paned>
</ng-window>`;
}

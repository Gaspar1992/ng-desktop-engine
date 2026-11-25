import { Component, HostListener, viewChild } from '@angular/core';
import { RouterLink, RouterOutlet } from '@angular/router';
import { DocsNavbarComponent } from './layout/docs-navbar.component';
import { SearchDialogComponent } from './layout/search-dialog.component';

@Component({
  selector: 'docs-root',
  imports: [RouterOutlet, RouterLink, DocsNavbarComponent, SearchDialogComponent],
  template: `
    <a class="skip-link" href="#main-content">Skip to main content</a>
    <docs-navbar (openSearch)="openSearchDialog()" />

    <main id="main-content" class="docs-main">
      <router-outlet />
    </main>

    <footer class="site-footer">
      <div class="docs-container footer-inner">
        <div>
          <strong>ng-desktop-engine</strong>
          <p class="footer-note">
            Angular 22+ compiled straight to GTK4, WinUI 3, and AppKit native widgets — zero
            WebView, zero Electron.
          </p>
        </div>
        <div class="footer-links">
          <a routerLink="/widgets">29 Primitives</a>
          <a routerLink="/playground">Live Playground</a>
          <a routerLink="/packages">Packages</a>
          <a href="api-ts/">TypeScript API</a>
          <a href="api-rust/">Rust API</a>
          <a
            href="https://github.com/Gaspar1992/ng-desktop-engine"
            target="_blank"
            rel="noopener noreferrer"
          >
            GitHub
          </a>
        </div>
      </div>
    </footer>

    <docs-search-dialog #searchDialog />
  `,
  styles: `
    :host {
      display: flex;
      flex-direction: column;
      min-height: 100dvh;
    }

    .skip-link {
      position: absolute;
      left: -9999px;
      top: 0;
      z-index: 100;
      padding: 0.65rem 1rem;
      background: var(--accent);
      color: #ffffff;
      font-weight: 700;
      border-radius: 0 0 8px 0;
      text-decoration: none;
    }

    .skip-link:focus {
      left: 0;
    }

    .docs-main {
      flex: 1 0 auto;
      padding-block: 2rem 4rem;
    }

    .site-footer {
      border-top: 1px solid var(--border);
      background: var(--bg-elevated);
      padding-block: 2rem;
    }

    .footer-inner {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      justify-content: space-between;
      gap: 1.5rem;
    }

    .footer-note {
      margin: 0.35rem 0 0;
      color: var(--text-muted);
      font-size: 0.9rem;
    }

    .footer-links {
      display: flex;
      flex-wrap: wrap;
      gap: 1rem;
      font-size: 0.9rem;
    }

    .footer-links a {
      text-decoration: none;
      color: var(--text-muted);
      font-weight: 600;
    }

    .footer-links a:hover {
      color: var(--text);
    }
  `,
})
export class App {
  private readonly searchDialog = viewChild.required(SearchDialogComponent);

  @HostListener('window:keydown', ['$event'])
  handleGlobalShortcut(event: KeyboardEvent): void {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      this.openSearchDialog();
    }
  }

  openSearchDialog(): void {
    this.searchDialog().open();
  }
}

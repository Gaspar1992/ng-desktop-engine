import {
  afterNextRender,
  Component,
  DestroyRef,
  ElementRef,
  inject,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { RouterLink, RouterLinkActive } from '@angular/router';
import { DocsThemeService } from '../core/theme.service';

@Component({
  selector: 'docs-navbar',
  imports: [RouterLink, RouterLinkActive],
  template: `
    <header #headerEl class="site-header" [class.is-wide]="isWide()">
      <div class="header-inner docs-container">
        <a routerLink="/" class="brand-link" aria-label="ng-desktop-engine home">
          <span class="brand-badge" aria-hidden="true">
            <svg viewBox="0 0 64 64" fill="none" class="brand-icon">
              <defs>
                <linearGradient
                  id="navbarShieldGrad"
                  x1="8"
                  y1="6"
                  x2="56"
                  y2="58"
                  gradientUnits="userSpaceOnUse"
                >
                  <stop offset="0%" stop-color="#dd0031" />
                  <stop offset="100%" stop-color="#7c3aed" />
                </linearGradient>
              </defs>
              <path
                d="M32 4 8 13l3.6 31.2L32 60l20.4-15.8L56 13 32 4Z"
                fill="url(#navbarShieldGrad)"
              />
              <rect
                x="17"
                y="18"
                width="30"
                height="22"
                rx="4"
                fill="#ffffff"
                fill-opacity="0.95"
              />
              <rect x="17" y="18" width="30" height="6" rx="3" fill="#18181b" fill-opacity="0.16" />
              <circle cx="22" cy="21" r="1.5" fill="#dd0031" />
              <circle cx="26.5" cy="21" r="1.5" fill="#f59e0b" />
              <circle cx="31" cy="21" r="1.5" fill="#10b981" />
              <path d="M25 35.5 32 26l7 9.5h-3.6L32 30.8l-3.4 4.7H25Z" fill="#18181b" />
            </svg>
          </span>
          <span class="brand-text">
            <strong>ng-desktop</strong>
            <span class="version-pill">v0.7.1</span>
          </span>
        </a>

        <button
          type="button"
          class="btn btn-ghost menu-toggle"
          popovertarget="docs-primary-nav"
          aria-controls="docs-primary-nav"
          aria-label="Toggle navigation menu"
        >
          Menu
        </button>

        <nav
          #navEl
          id="docs-primary-nav"
          aria-label="Primary"
          class="primary-nav"
          [class.is-wide]="isWide()"
        >
          <a
            routerLink="/getting-started"
            routerLinkActive="is-active"
            ariaCurrentWhenActive="page"
            (click)="closePopoverIfNarrow()"
          >
            Getting Started
          </a>
          <a
            routerLink="/architecture"
            routerLinkActive="is-active"
            ariaCurrentWhenActive="page"
            (click)="closePopoverIfNarrow()"
          >
            Architecture
          </a>
          <a
            routerLink="/widgets"
            routerLinkActive="is-active"
            ariaCurrentWhenActive="page"
            (click)="closePopoverIfNarrow()"
          >
            Primitives
          </a>
          <a
            routerLink="/playground"
            routerLinkActive="is-active"
            ariaCurrentWhenActive="page"
            (click)="closePopoverIfNarrow()"
          >
            Playground
          </a>
          <a
            routerLink="/packages"
            routerLinkActive="is-active"
            ariaCurrentWhenActive="page"
            (click)="closePopoverIfNarrow()"
          >
            Packages
          </a>
          <a
            routerLink="/showcase/pokedex"
            routerLinkActive="is-active"
            ariaCurrentWhenActive="page"
            (click)="closePopoverIfNarrow()"
          >
            Pokédex
          </a>
        </nav>

        <div class="header-actions">
          <button
            type="button"
            class="btn btn-ghost search-btn"
            aria-label="Search documentation (Ctrl+K)"
            (click)="openSearch.emit()"
          >
            <span>Search</span>
            <kbd aria-hidden="true">Ctrl+K</kbd>
          </button>

          <button
            type="button"
            class="btn btn-ghost theme-btn"
            [attr.aria-label]="
              theme.effectiveTheme() === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'
            "
            (click)="theme.toggle()"
          >
            {{ theme.effectiveTheme() === 'dark' ? 'Light' : 'Dark' }}
          </button>

          <a
            class="btn btn-ghost github-link"
            href="https://github.com/Gaspar1992/ng-desktop-engine"
            target="_blank"
            rel="noopener noreferrer"
          >
            GitHub
          </a>
        </div>
      </div>
    </header>
  `,
  styles: `
    .site-header {
      position: sticky;
      top: 0;
      z-index: 40;
      background: var(--bg-elevated);
      border-bottom: 1px solid var(--border);
      backdrop-filter: blur(10px);
    }

    .header-inner {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 1rem;
      min-height: 4rem;
      padding-block: 0.5rem;
    }

    .brand-link {
      display: inline-flex;
      align-items: center;
      gap: 0.6rem;
      text-decoration: none;
      color: var(--text);
      flex-shrink: 0;
    }

    .brand-badge {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      width: 2rem;
      height: 2rem;
      flex-shrink: 0;
    }

    .brand-icon {
      width: 100%;
      height: 100%;
      display: block;
    }

    .brand-text {
      display: inline-flex;
      align-items: center;
      gap: 0.45rem;
    }

    .version-pill {
      font-size: 0.72rem;
      font-weight: 700;
      padding: 0.1rem 0.45rem;
      border-radius: 999px;
      background: var(--surface-muted);
      border: 1px solid var(--border);
      color: var(--text-muted);
    }

    .menu-toggle {
      display: inline-flex;
      anchor-name: --menu-button;
    }

    .site-header.is-wide .menu-toggle {
      display: none;
    }

    .primary-nav {
      display: none;
    }

    .primary-nav.is-wide {
      display: flex;
      position: static;
      inset: auto;
      align-items: center;
      gap: 0.25rem;
      flex-wrap: wrap;
      background: transparent;
      border: none;
      padding: 0;
      margin: 0;
      box-shadow: none;
    }

    .primary-nav:popover-open {
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
      min-width: 14rem;
      padding: 0.65rem;
      margin: 0;
      border: 1px solid var(--border-strong);
      border-radius: 12px;
      background: var(--bg-elevated);
      box-shadow: var(--shadow-md);
      position: fixed;
      top: 4.25rem;
      left: 1rem;
      right: auto;
      bottom: auto;
    }

    @supports (anchor-name: --menu-button) {
      .primary-nav:popover-open {
        top: anchor(--menu-button bottom);
        left: anchor(--menu-button left);
        margin-top: 0.4rem;
      }
    }

    .primary-nav a {
      position: relative;
      padding: 0.45rem 0.72rem;
      border-radius: 8px;
      text-decoration: none;
      color: var(--text-muted);
      font-weight: 600;
      font-size: 0.9rem;
      border-bottom: 2px solid transparent;
    }

    .primary-nav a:hover {
      color: var(--text);
      background: var(--surface);
    }

    .primary-nav a.is-active {
      color: var(--text);
      background: var(--accent-soft);
      border-bottom-color: var(--accent);
    }

    .header-actions {
      display: flex;
      align-items: center;
      gap: 0.45rem;
      flex-shrink: 0;
    }

    kbd {
      font-family: var(--font-mono);
      font-size: 0.72rem;
      padding: 0.1rem 0.35rem;
      border-radius: 5px;
      border: 1px solid var(--border-strong);
      background: var(--surface);
      color: var(--text-muted);
    }

    @media (max-width: 640px) {
      .github-link,
      .search-btn kbd {
        display: none;
      }
    }
  `,
})
export class DocsNavbarComponent {
  readonly theme = inject(DocsThemeService);
  private readonly destroyRef = inject(DestroyRef);

  readonly openSearch = output<void>();
  readonly isWide = signal<boolean>(true);

  private readonly headerRef = viewChild.required<ElementRef<HTMLElement>>('headerEl');
  private readonly navRef = viewChild.required<ElementRef<HTMLElement>>('navEl');

  constructor() {
    afterNextRender(() => {
      const headerEl = this.headerRef().nativeElement;
      const updateLayoutMode = (width: number): void => {
        const wide = width >= 920;
        this.isWide.set(wide);
        const navEl = this.navRef().nativeElement;
        if (wide) {
          navEl.removeAttribute('popover');
        } else {
          navEl.setAttribute('popover', 'auto');
        }
      };

      const initialWidth =
        headerEl.clientWidth > 0 ? headerEl.clientWidth : globalThis.innerWidth || 1024;
      updateLayoutMode(initialWidth);

      if (typeof ResizeObserver !== 'undefined') {
        const observer = new ResizeObserver((entries) => {
          const entryWidth = entries[0]?.contentRect.width;
          if (typeof entryWidth === 'number' && entryWidth > 0) {
            updateLayoutMode(entryWidth);
          }
        });
        observer.observe(headerEl);
        this.destroyRef.onDestroy(() => observer.disconnect());
      }
    });
  }

  closePopoverIfNarrow(): void {
    const navEl = this.navRef().nativeElement as HTMLElement & {
      hidePopover?: () => void;
    };
    if (!this.isWide() && navEl.hasAttribute('popover')) {
      try {
        navEl.hidePopover?.();
      } catch {
        // Ignore if popover is not open in current browser/jsdom environment.
      }
    }
  }
}

import { Component, signal } from '@angular/core';
import { RouterLink } from '@angular/router';

@Component({
  selector: 'docs-getting-started-page',
  imports: [RouterLink],
  template: `
    <div class="docs-container guide-shell">
      <header class="page-head">
        <span class="eyebrow">Quickstart Guide</span>
        <h1>Getting Started with ng-desktop-engine</h1>
        <p>
          Create a native Linux, Windows, or macOS desktop app from any Angular 22+ workspace in
          four small steps.
        </p>
      </header>

      <section class="card step-card">
        <h2>1. Prerequisites</h2>
        <ul class="req-list">
          <li><strong>Node.js 24+</strong> &amp; Angular CLI 22+ (with pnpm or npm).</li>
          <li>
            <strong>OS Desktop Libraries (Linux runtime / local dev):</strong> GTK4 &amp; Libadwaita
            development headers (Windows WinUI 3 and macOS AppKit use built-in OS SDKs).
          </li>
          <li>
            <strong>Prebuilt Engine Binaries:</strong> automatically installed via
            <code>optionalDependencies</code> on x86_64/arm64 Linux, Windows x64, and macOS Apple
            Silicon/Intel — zero Rust toolchain needed for normal JS builds.
          </li>
        </ul>
      </section>

      <section class="card step-card">
        <div class="card-head">
          <h2>2. Scaffold an Existing Angular Workspace</h2>
          <button type="button" class="btn btn-ghost" (click)="copyBlock('scaffold', scaffoldCmd)">
            {{ copiedKey() === 'scaffold' ? 'Copied' : 'Copy' }}
          </button>
        </div>
        <p>
          Run <code>ng add &#64;ng-desktop/builders</code> inside any Angular CLI workspace. It adds
          the <code>"native"</code> architect target to <code>angular.json</code> without breaking
          existing web builds.
        </p>
        <pre><code>{{ scaffoldCmd }}</code></pre>
      </section>

      <section class="card step-card">
        <div class="card-head">
          <h2>3. Configure Native Bootstrap</h2>
          <button
            type="button"
            class="btn btn-ghost"
            (click)="copyBlock('bootstrap', bootstrapSnippet)"
          >
            {{ copiedKey() === 'bootstrap' ? 'Copied' : 'Copy' }}
          </button>
        </div>
        <p>
          Swap <code>bootstrapApplication</code> for <code>bootstrapNativeApplication</code> in
          <code>src/main.ts</code> and register <code>provideNativePlatform()</code> along with
          optional router, forms, HTTP, desktop, and SQLite server providers.
        </p>
        <pre><code>{{ bootstrapSnippet }}</code></pre>
      </section>

      <section class="card step-card">
        <div class="card-head">
          <h2>4. Run with HMR or Package OS Installers</h2>
          <button type="button" class="btn btn-ghost" (click)="copyBlock('cli', cliCommands)">
            {{ copiedKey() === 'cli' ? 'Copied' : 'Copy' }}
          </button>
        </div>
        <pre><code>{{ cliCommands }}</code></pre>
        <div class="next-links">
          <a routerLink="/widgets" class="btn btn-primary">Browse 29 Primitives</a>
          <a routerLink="/architecture" class="btn">Read Architecture</a>
          <a routerLink="/showcase/pokedex" class="btn">Inspect Pokédex Case Study</a>
        </div>
      </section>
    </div>
  `,
  styles: `
    .guide-shell {
      display: grid;
      gap: 1.5rem;
    }

    .page-head {
      display: grid;
      gap: 0.55rem;
      justify-items: start;
    }

    .page-head h1 {
      margin: 0;
      font-size: clamp(1.85rem, 3.2vw, 2.5rem);
    }

    .page-head p {
      margin: 0;
      color: var(--text-muted);
      max-width: 66ch;
    }

    .step-card {
      display: grid;
      gap: 0.85rem;
    }

    .step-card h2,
    .step-card p {
      margin: 0;
    }

    .card-head {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 1rem;
    }

    .req-list {
      margin: 0;
      padding-left: 1.25rem;
      display: grid;
      gap: 0.45rem;
    }

    .next-links {
      display: flex;
      flex-wrap: wrap;
      gap: 0.75rem;
      margin-top: 0.35rem;
    }
  `,
})
export class GettingStartedPage {
  readonly copiedKey = signal<string | null>(null);

  readonly scaffoldCmd = `ng new my-desktop-app --minimal
cd my-desktop-app
ng add @ng-desktop/builders`;

  readonly bootstrapSnippet = `// src/main.ts
import { bootstrapNativeApplication, provideNativePlatform } from '@ng-desktop/platform';
import { provideNativeRouter } from '@ng-desktop/router';
import { provideNativeHttp } from '@ng-desktop/http';
import { App } from './app/app';
import { routes } from './app/app.routes';

bootstrapNativeApplication(App, {
  providers: [
    provideNativePlatform(),
    provideNativeRouter(routes),
    provideNativeHttp(),
  ],
});`;

  readonly cliCommands = `# Live desktop preview + template/CSS Hot Module Reload
pnpm exec ng-desktop dev

# Inspect sandbox capabilities inferred from app usage
pnpm exec ng-desktop permissions

# Build production .ngb bundle and generate native installers
pnpm exec ng-desktop build
pnpm exec ng-desktop package --target deb   # or appimage | flatpak | msix | dmg`;

  async copyBlock(key: string, text: string): Promise<void> {
    try {
      await globalThis.navigator?.clipboard?.writeText(text);
    } catch {
      // Ignore clipboard denials in headless environments.
    }
    this.copiedKey.set(key);
    setTimeout(() => {
      if (this.copiedKey() === key) {
        this.copiedKey.set(null);
      }
    }, 1800);
  }
}

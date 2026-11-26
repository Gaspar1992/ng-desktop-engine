import { Component, computed, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { compilePlaygroundToNgb } from '../core/ngb-playground-compiler';
import { DocsThemeService } from '../core/theme.service';
import { COMPARISON_ROWS } from '../data/architecture-pillars.data';
import { PLAYGROUND_PRESETS } from '../data/playground-presets.data';

@Component({
  selector: 'docs-home-page',
  imports: [RouterLink],
  template: `
    <div class="docs-container home-shell">
      <section class="hero-card">
        <span class="eyebrow">v0.7.0 · Zero WebView · Zero Electron</span>
        <h1>Angular 22 Native Desktop Apps</h1>
        <p class="hero-lead">
          Compile standard Angular templates, Signals, Control Flow, and scoped CSS directly into
          real GTK4 / Libadwaita, Windows 11 WinUI 3, and macOS AppKit widgets powered by a
          lightweight Rust runtime.
        </p>

        <div class="cli-install-bar" role="region" aria-label="Quick install command">
          <code>ng add &#64;ng-desktop/builders</code>
          <button type="button" class="btn btn-primary copy-btn" (click)="copyInstallCommand()">
            {{ copiedInstall() ? 'Copied' : 'Copy Command' }}
          </button>
        </div>

        <div class="hero-actions">
          <a routerLink="/getting-started" class="btn btn-primary">Get Started</a>
          <a routerLink="/widgets" class="btn">Explore 29 Primitives</a>
          <a routerLink="/playground" class="btn">Open Live .ngb Playground</a>
        </div>
      </section>

      <section class="metrics-grid" aria-label="Platform highlights">
        <article class="card metric-card">
          <strong class="metric-value">29</strong>
          <span class="metric-label">OS-native UI primitives</span>
          <p>Window, HeaderBar, Paned, Stack, VirtualGrid, Signal Form controls, and meters.</p>
        </article>
        <article class="card metric-card">
          <strong class="metric-value">3 OS</strong>
          <span class="metric-label">GTK4 · WinUI 3 · AppKit</span>
          <p>Shared Rust Flexbox/Grid solver driving native platform widget toolkits.</p>
        </article>
        <article class="card metric-card">
          <strong class="metric-value">7 + 5</strong>
          <span class="metric-label">Modular &#64;ng-desktop/* pkgs</span>
          <p>7 Angular/tooling libraries paired with 5 OIDC-published native binaries.</p>
        </article>
        <article class="card metric-card">
          <strong class="metric-value">~20 MB</strong>
          <span class="metric-label">Idle footprint</span>
          <p>JavaScriptCore / QuickJS-NG VM + generational Rust arena instead of Chromium.</p>
        </article>
      </section>

      <section class="comparison-section">
        <div class="section-header">
          <h2 id="comparison-heading">How ng-desktop compares</h2>
          <p>
            Unlike Electron or WebView wrappers, Angular Ivy drives OS controls without shipping or
            booting a browser engine.
          </p>
        </div>

        <div class="table-wrapper" tabindex="0" role="region" aria-labelledby="comparison-heading">
          <table class="docs-table">
            <caption>
              Runtime architecture comparison across Angular desktop toolchains
            </caption>
            <thead>
              <tr>
                <th scope="col">Dimension</th>
                <th scope="col">ng-desktop-engine</th>
                <th scope="col">Electron</th>
                <th scope="col">Tauri</th>
              </tr>
            </thead>
            <tbody>
              @for (row of comparisonRows; track row.aspect) {
                <tr>
                  <th scope="row">{{ row.aspect }}</th>
                  <td data-label="ng-desktop-engine">
                    <strong>{{ row.ngDesktop }}</strong>
                  </td>
                  <td data-label="Electron">{{ row.electron }}</td>
                  <td data-label="Tauri">{{ row.tauri }}</td>
                </tr>
              }
            </tbody>
          </table>
        </div>
      </section>

      <section class="preview-cta card">
        <div class="preview-copy">
          <span class="eyebrow">Live In-Browser Compiler</span>
          <h2>Inspect <code>.ngb</code> compilation in real time</h2>
          <p>
            Try Angular templates and CSS variables in the interactive playground powered directly
            by
            <code>&#64;ng-desktop/platform</code>'s style and tag resolution helpers.
          </p>
          <a routerLink="/playground" class="btn btn-primary">Launch Full Playground</a>
        </div>
        <pre class="mini-output"><code>@for (line of sampleInstructions(); track $index) {
{{ line.opcode.padEnd(12, ' ') }} {{ line.summary }}
}</code></pre>
      </section>
    </div>
  `,
  styles: `
    .home-shell {
      display: grid;
      gap: 2.5rem;
    }

    .hero-card {
      padding: 2.25rem;
      border-radius: 20px;
      border: 1px solid var(--border);
      background:
        radial-gradient(circle at top right, var(--accent-soft), transparent 46%),
        radial-gradient(circle at bottom left, var(--violet-soft), transparent 46%),
        var(--bg-elevated);
      box-shadow: var(--shadow-md);
      display: grid;
      gap: 1.15rem;
      justify-items: start;
    }

    h1 {
      margin: 0;
      font-size: clamp(2rem, 4.2vw, 3.1rem);
      line-height: 1.12;
      letter-spacing: -0.02em;
    }

    .hero-lead {
      margin: 0;
      max-width: 68ch;
      font-size: 1.08rem;
      color: var(--text-muted);
    }

    .cli-install-bar {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      gap: 0.75rem;
      padding: 0.45rem 0.45rem 0.45rem 1rem;
      border-radius: 12px;
      background: var(--code-bg);
      border: 1px solid var(--border-strong);
    }

    .cli-install-bar code {
      background: transparent;
      border: none;
      padding: 0;
      font-size: 0.96rem;
      font-weight: 600;
    }

    .hero-actions {
      display: flex;
      flex-wrap: wrap;
      gap: 0.75rem;
    }

    .metrics-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
      gap: 1rem;
    }

    .metric-card {
      display: grid;
      gap: 0.35rem;
    }

    .metric-value {
      font-size: 1.9rem;
      line-height: 1.1;
      color: var(--accent);
    }

    .metric-label {
      font-weight: 700;
    }

    .metric-card p {
      margin: 0.25rem 0 0;
      color: var(--text-muted);
      font-size: 0.92rem;
    }

    .section-header h2 {
      margin: 0 0 0.35rem;
      font-size: 1.55rem;
    }

    .section-header p {
      margin: 0 0 1rem;
      color: var(--text-muted);
    }

    .preview-cta {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(290px, 1fr));
      gap: 1.5rem;
      align-items: center;
    }

    .preview-copy {
      display: grid;
      gap: 0.75rem;
      justify-items: start;
    }

    .preview-copy h2,
    .preview-copy p {
      margin: 0;
    }

    .mini-output {
      max-height: 240px;
    }
  `,
})
export class HomePage {
  private readonly theme = inject(DocsThemeService);
  readonly comparisonRows = COMPARISON_ROWS;
  readonly copiedInstall = signal<boolean>(false);

  readonly sampleInstructions = computed(() => {
    const preset = PLAYGROUND_PRESETS[0];
    if (!preset) {
      return [];
    }
    return compilePlaygroundToNgb(
      preset.template,
      preset.css,
      this.theme.effectiveTheme(),
    ).instructions.slice(0, 9);
  });

  async copyInstallCommand(): Promise<void> {
    if (!globalThis.navigator?.clipboard?.writeText) {
      return;
    }
    try {
      await globalThis.navigator.clipboard.writeText('ng add @ng-desktop/builders');
      this.copiedInstall.set(true);
      setTimeout(() => this.copiedInstall.set(false), 1800);
    } catch {
      // Do not indicate success when clipboard write fails.
    }
  }
}

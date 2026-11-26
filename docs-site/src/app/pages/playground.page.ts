import { Component, computed, inject, signal } from '@angular/core';
import { compilePlaygroundToNgb } from '../core/ngb-playground-compiler';
import { DocsThemeService } from '../core/theme.service';
import { PLAYGROUND_PRESETS, type PlaygroundPreset } from '../data/playground-presets.data';
import { PlaygroundPreviewNodeComponent } from './playground-preview-node.component';

@Component({
  selector: 'docs-playground-page',
  imports: [PlaygroundPreviewNodeComponent],
  template: `
    <div class="docs-container playground-shell">
      <header class="page-head">
        <span class="eyebrow">Real &#64;ng-desktop/platform Engine Parser</span>
        <h1>Live .ngb Template &amp; CSS Playground</h1>
        <p>
          Edit Angular desktop templates and scoped stylesheets below to inspect the compiled
          <code>.ngb</code> opcodes, theme variable cascades, and simulated widget hierarchy.
        </p>
      </header>

      <div class="preset-bar card">
        <div class="preset-buttons" role="group" aria-label="Playground presets">
          @for (preset of presets; track preset.id) {
            <button
              type="button"
              class="btn"
              [class.btn-primary]="activePresetId() === preset.id"
              [attr.aria-pressed]="activePresetId() === preset.id"
              (click)="loadPreset(preset)"
            >
              {{ preset.name }}
            </button>
          }
        </div>

        <div class="preview-controls" role="group" aria-label="Simulated OS theme">
          <span>Simulated OS Theme:</span>
          <button
            type="button"
            class="btn"
            [class.btn-primary]="pinnedPreviewTheme() === null"
            [attr.aria-pressed]="pinnedPreviewTheme() === null"
            (click)="setPreviewTheme(null)"
          >
            Auto
          </button>
          <button
            type="button"
            class="btn"
            [class.btn-primary]="pinnedPreviewTheme() === 'light'"
            [attr.aria-pressed]="pinnedPreviewTheme() === 'light'"
            (click)="setPreviewTheme('light')"
          >
            Light
          </button>
          <button
            type="button"
            class="btn"
            [class.btn-primary]="pinnedPreviewTheme() === 'dark'"
            [attr.aria-pressed]="pinnedPreviewTheme() === 'dark'"
            (click)="setPreviewTheme('dark')"
          >
            Dark
          </button>
        </div>
      </div>

      <div class="workspace-grid">
        <section class="editor-column">
          <div class="card editor-card">
            <label for="playground-template">Angular Template (&lt;ng-*&gt; or HTML)</label>
            <textarea
              id="playground-template"
              class="code-editor"
              rows="12"
              [value]="templateSource()"
              (input)="onTemplateInput($event)"
            ></textarea>
          </div>

          <div class="card editor-card">
            <label for="playground-css">Stylesheet (:root vars &amp; &#64;media theme rules)</label>
            <textarea
              id="playground-css"
              class="code-editor"
              rows="12"
              [value]="cssSource()"
              (input)="onCssInput($event)"
            ></textarea>
          </div>
        </section>

        <section class="output-column">
          <div class="stats-row">
            <div class="card stat-pill">
              <strong data-testid="node-count">{{ compilation().nodeCount }}</strong>
              <span>Native Nodes</span>
            </div>
            <div class="card stat-pill">
              <strong data-testid="style-count">{{ compilation().styleRuleCount }}</strong>
              <span>Style Rules</span>
            </div>
            <div class="card stat-pill">
              <strong data-testid="instruction-count">{{ compilation().instructionCount }}</strong>
              <span>.ngb Opcodes</span>
            </div>
          </div>

          @if (compilation().warnings.length > 0) {
            <div class="card warnings-box" role="status">
              @for (warning of compilation().warnings; track warning) {
                <p>{{ warning }}</p>
              }
            </div>
          }

          <article
            class="card window-preview"
            [class.preview-dark]="previewTheme() === 'dark'"
            [class.preview-light]="previewTheme() === 'light'"
            aria-label="Simulated native widget preview"
          >
            <div class="window-chrome">
              <span class="chrome-dots" aria-hidden="true">
                <span></span><span></span><span></span>
              </span>
              <strong>Native Widget Tree ({{ previewTheme() }})</strong>
            </div>

            <div class="preview-body">
              @for (root of compilation().roots; track root.id) {
                <docs-playground-preview-node [node]="root" />
              } @empty {
                <p class="empty-preview">Write markup in the template editor to render nodes.</p>
              }
            </div>
          </article>

          <article class="card opcodes-card">
            <h2>Compiled .ngb Instruction Stream</h2>
            <pre class="opcodes-pre"><code>@for (line of compilation().instructions; track $index) {
{{ line.opcode.padEnd(12, ' ') }} {{ line.summary }}
}</code></pre>
          </article>
        </section>
      </div>
    </div>
  `,
  styles: `
    .playground-shell {
      display: grid;
      gap: 1.4rem;
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
      max-width: 72ch;
    }

    .preset-bar {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      justify-content: space-between;
      gap: 1rem;
    }

    .preset-buttons,
    .preview-controls {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      gap: 0.5rem;
    }

    .preview-controls span {
      font-weight: 600;
      font-size: 0.9rem;
      color: var(--text-muted);
    }

    .workspace-grid {
      display: grid;
      gap: 1.25rem;
      align-items: start;
    }

    @media (min-width: 980px) {
      .workspace-grid {
        grid-template-columns: minmax(0, 1fr) minmax(0, 1.05fr);
      }
    }

    .editor-column,
    .output-column {
      display: grid;
      gap: 1rem;
    }

    .editor-card {
      display: grid;
      gap: 0.55rem;
    }

    .editor-card label {
      font-weight: 700;
      font-size: 0.92rem;
    }

    textarea.code-editor {
      width: 100%;
      padding: 0.85rem;
      border-radius: 10px;
      border: 1px solid var(--border-strong);
      background: var(--code-bg);
      color: var(--text);
      font-size: 0.85rem;
      line-height: 1.5;
      resize: vertical;
    }

    .stats-row {
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 0.75rem;
    }

    .stat-pill {
      display: grid;
      gap: 0.15rem;
      text-align: center;
      padding: 0.85rem;
    }

    .stat-pill strong {
      font-size: 1.45rem;
      color: var(--accent);
    }

    .stat-pill span {
      font-size: 0.8rem;
      color: var(--text-muted);
      font-weight: 600;
    }

    .warnings-box {
      border-color: var(--warning);
      color: var(--warning);
    }

    .warnings-box p {
      margin: 0;
      font-size: 0.9rem;
    }

    .window-preview {
      padding: 0;
      overflow: hidden;
    }

    .window-preview.preview-light {
      --bg-elevated: #ffffff;
      --surface: #f4f5f7;
      --border: #d8dce6;
      --text: #151821;
      background: #f6f7fb;
      color: #151821;
    }

    .window-preview.preview-dark {
      --bg-elevated: #171a23;
      --surface: #202431;
      --border: #32384b;
      --text: #eef1f8;
      background: #11141c;
      color: #eef1f8;
    }

    .window-chrome {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      padding: 0.7rem 1rem;
      background: var(--surface);
      border-bottom: 1px solid var(--border);
      font-size: 0.88rem;
    }

    .chrome-dots {
      display: inline-flex;
      gap: 0.35rem;
    }

    .chrome-dots span {
      width: 0.68rem;
      height: 0.68rem;
      border-radius: 50%;
      background: var(--border-strong);
    }

    .preview-body {
      padding: 1rem;
      display: grid;
      gap: 0.75rem;
    }

    .empty-preview {
      margin: 0;
      color: var(--text-muted);
    }

    .opcodes-card {
      display: grid;
      gap: 0.65rem;
    }

    .opcodes-card h2 {
      margin: 0;
      font-size: 1.05rem;
    }

    .opcodes-pre {
      max-height: 290px;
    }
  `,
})
export class PlaygroundPage {
  private readonly portalTheme = inject(DocsThemeService);
  readonly presets = PLAYGROUND_PRESETS;

  private readonly initialPreset = PLAYGROUND_PRESETS[0]!;
  readonly activePresetId = signal<string>(this.initialPreset.id);
  readonly templateSource = signal<string>(this.initialPreset.template);
  readonly cssSource = signal<string>(this.initialPreset.css);

  readonly pinnedPreviewTheme = signal<'light' | 'dark' | null>(null);

  readonly previewTheme = computed<'light' | 'dark'>(
    () => this.pinnedPreviewTheme() ?? this.portalTheme.effectiveTheme(),
  );

  readonly compilation = computed(() =>
    compilePlaygroundToNgb(this.templateSource(), this.cssSource(), this.previewTheme()),
  );

  loadPreset(preset: PlaygroundPreset): void {
    this.activePresetId.set(preset.id);
    this.templateSource.set(preset.template);
    this.cssSource.set(preset.css);
  }

  setPreviewTheme(mode: 'light' | 'dark' | null): void {
    this.pinnedPreviewTheme.set(mode);
  }

  onTemplateInput(event: Event): void {
    const target = event.target as HTMLTextAreaElement | null;
    this.templateSource.set(target?.value ?? '');
  }

  onCssInput(event: Event): void {
    const target = event.target as HTMLTextAreaElement | null;
    this.cssSource.set(target?.value ?? '');
  }
}

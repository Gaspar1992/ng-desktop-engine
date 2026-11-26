import { Component, computed, ElementRef, signal, viewChildren } from '@angular/core';
import { RouterLink } from '@angular/router';
import { PIPELINE_STAGES, type PipelineStage } from '../data/architecture-pillars.data';

@Component({
  selector: 'docs-architecture-page',
  imports: [RouterLink],
  template: `
    <div class="docs-container arch-shell">
      <header class="page-head">
        <span class="eyebrow">System Architecture</span>
        <h1>Inside the Rust &amp; Angular Execution Pipeline</h1>
        <p>
          Select any stage below to inspect how Angular 22 change detection reaches native OS
          toolkits without a Chromium DOM layer.
        </p>
      </header>

      <div class="stage-tabs" role="tablist" aria-label="Pipeline stages">
        @for (stage of stages; track stage.id; let idx = $index) {
          <button
            #tabBtn
            type="button"
            role="tab"
            [id]="'tab-' + stage.id"
            class="stage-tab"
            [class.is-selected]="selectedId() === stage.id"
            [attr.aria-selected]="selectedId() === stage.id"
            [attr.aria-controls]="'panel-' + stage.id"
            [attr.tabindex]="selectedId() === stage.id ? 0 : -1"
            (click)="selectedId.set(stage.id)"
            (keydown)="onTabKeydown($event, idx)"
          >
            <span class="stage-step">{{ stage.step }}</span>
            <span class="stage-title">{{ stage.title }}</span>
          </button>
        }
      </div>

      @if (activeStage(); as stage) {
        <section
          class="card stage-detail"
          role="tabpanel"
          tabindex="0"
          [id]="'panel-' + stage.id"
          [attr.aria-labelledby]="'tab-' + stage.id"
        >
          <div class="detail-meta">
            <span class="eyebrow">Stage {{ stage.step }}</span>
            <div class="module-chips">
              @for (mod of stage.modules; track mod) {
                <code>{{ mod }}</code>
              }
            </div>
          </div>
          <h2>{{ stage.title }}</h2>
          <p class="stage-summary">{{ stage.summary }}</p>
          <p>{{ stage.detail }}</p>
        </section>
      }

      <section class="pillars-grid" aria-label="Core engineering guarantees">
        <article class="card">
          <h3>Generational Arena Safety</h3>
          <p>
            Every widget lives in a generational <code>NodeId</code> slot table rather than
            reference-counted cycles, preventing use-after-destroy crashes during rapid view swaps.
          </p>
        </article>
        <article class="card">
          <h3>Reactive Dark &amp; Light CSS Cascade</h3>
          <p>
            Build-time and runtime style compilers resolve <code>:root</code> custom properties,
            pseudo-classes, transitions, and <code>&#64;media (prefers-color-scheme)</code> rules
            onto native controls with <code>SetTheme</code>.
          </p>
        </article>
        <article class="card">
          <h3>GPU-Aware Native Toolkits</h3>
          <p>
            GTK4 leverages GSK GPU acceleration (<code>vulkan</code>, <code>gl</code>,
            <code>ngl</code>), AppKit enables <code>setWantsLayer:YES</code> Metal compositing, and
            Windows drives DWM-composited HWNDs. Override via
            <code>NG_DESKTOP_GPU_BACKEND=auto|vulkan|gl|ngl|metal|dwm|software</code>.
          </p>
        </article>
      </section>

      <div class="arch-actions">
        <a routerLink="/playground" class="btn btn-primary">Inspect Compiled .ngb Instructions</a>
        <a routerLink="/widgets" class="btn">View All 29 Widget Mappings</a>
      </div>
    </div>
  `,
  styles: `
    .arch-shell {
      display: grid;
      gap: 1.75rem;
    }

    .page-head {
      display: grid;
      gap: 0.55rem;
      justify-items: start;
    }

    .page-head h1,
    .page-head p {
      margin: 0;
    }

    .page-head p {
      color: var(--text-muted);
      max-width: 68ch;
    }

    .stage-tabs {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
      gap: 0.75rem;
    }

    .stage-tab {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      padding: 0.85rem 1rem;
      border-radius: 12px;
      border: 1px solid var(--border);
      background: var(--bg-elevated);
      color: var(--text);
      font: inherit;
      text-align: left;
      cursor: pointer;
    }

    .stage-tab:hover {
      background: var(--surface);
    }

    .stage-tab.is-selected {
      border-color: var(--accent);
      background: var(--accent-soft);
    }

    .stage-step {
      font-family: var(--font-mono);
      font-weight: 800;
      font-size: 0.85rem;
      color: var(--accent);
    }

    .stage-title {
      font-weight: 700;
      font-size: 0.94rem;
    }

    .stage-detail {
      display: grid;
      gap: 0.75rem;
    }

    .stage-detail h2,
    .stage-detail p {
      margin: 0;
    }

    .detail-meta {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      justify-content: space-between;
      gap: 0.75rem;
    }

    .module-chips {
      display: flex;
      flex-wrap: wrap;
      gap: 0.45rem;
    }

    .stage-summary {
      font-weight: 600;
    }

    .pillars-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
      gap: 1rem;
    }

    .pillars-grid h3 {
      margin: 0 0 0.45rem;
    }

    .pillars-grid p {
      margin: 0;
      color: var(--text-muted);
      font-size: 0.94rem;
    }

    .arch-actions {
      display: flex;
      flex-wrap: wrap;
      gap: 0.75rem;
    }
  `,
})
export class ArchitecturePage {
  readonly stages = PIPELINE_STAGES;
  readonly selectedId = signal<string>(PIPELINE_STAGES[0]?.id ?? 'angular-ivy');
  private readonly tabButtons = viewChildren<ElementRef<HTMLButtonElement>>('tabBtn');

  readonly activeStage = computed<PipelineStage | undefined>(() =>
    this.stages.find((stage) => stage.id === this.selectedId()),
  );

  onTabKeydown(event: KeyboardEvent, currentIndex: number): void {
    const count = this.stages.length;
    if (count === 0) {
      return;
    }
    let nextIndex: number | null = null;
    if (event.key === 'ArrowRight' || event.key === 'ArrowDown') {
      nextIndex = (currentIndex + 1) % count;
    } else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') {
      nextIndex = (currentIndex - 1 + count) % count;
    } else if (event.key === 'Home') {
      nextIndex = 0;
    } else if (event.key === 'End') {
      nextIndex = count - 1;
    }

    if (nextIndex === null) {
      return;
    }
    event.preventDefault();
    const targetStage = this.stages[nextIndex];
    if (!targetStage) {
      return;
    }
    this.selectedId.set(targetStage.id);
    this.tabButtons()[nextIndex]?.nativeElement.focus();
  }
}

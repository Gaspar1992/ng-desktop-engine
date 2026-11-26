import { Component, computed, input } from '@angular/core';
import type { PlaygroundPreviewNode } from '../core/ngb-playground-compiler';

@Component({
  selector: 'docs-playground-preview-node',
  imports: [PlaygroundPreviewNodeComponent],
  template: `
    <div
      class="native-node"
      [attr.data-tag]="node().nativeTag"
      [style.background]="node().style.background ?? null"
      [style.color]="node().style.color ?? null"
      [style.padding.px]="node().style.paddingPx ?? null"
      [style.gap.px]="node().style.gapPx ?? null"
      [style.border-radius.px]="node().style.borderRadiusPx ?? null"
      [style.font-size.px]="node().style.fontSizePx ?? null"
      [style.font-weight]="node().style.fontWeight ?? null"
      [style.opacity]="node().style.opacity ?? null"
    >
      <div class="node-header">
        <code class="node-tag">&lt;{{ node().nativeTag }}&gt;</code>
        @if (primaryLabel(); as label) {
          <span class="node-label">{{ label }}</span>
        }
        @for (cls of node().classes; track $index) {
          <span class="node-class">.{{ cls }}</span>
        }
      </div>

      @if (node().nativeTag === 'ng-level-bar' || node().nativeTag === 'ng-progress-bar') {
        <div class="native-meter" aria-hidden="true">
          <div class="native-meter-fill" [style.width.%]="meterPercent()"></div>
        </div>
      }

      @if (node().children.length > 0) {
        <div class="node-children" [class.is-horizontal]="isHorizontal()">
          @for (child of node().children; track child.id) {
            <docs-playground-preview-node [node]="child" />
          }
        </div>
      }
    </div>
  `,
  styles: `
    .native-node {
      border: 1px solid var(--border);
      border-radius: 10px;
      padding: 0.65rem 0.8rem;
      display: grid;
      gap: 0.5rem;
    }

    .node-header {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      gap: 0.45rem;
    }

    .node-tag {
      font-size: 0.76rem;
      font-weight: 700;
    }

    .node-label {
      font-weight: 600;
      font-size: 0.92rem;
    }

    .node-class {
      font-family: var(--font-mono);
      font-size: 0.72rem;
      padding: 0.1rem 0.4rem;
      border-radius: 999px;
      border: 1px solid var(--border);
      opacity: 0.85;
    }

    .native-meter {
      height: 0.5rem;
      border-radius: 999px;
      background: rgba(128, 128, 140, 0.25);
      overflow: hidden;
    }

    .native-meter-fill {
      height: 100%;
      background: var(--accent);
    }

    .node-children {
      display: grid;
      gap: 0.55rem;
    }

    .node-children.is-horizontal {
      grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    }
  `,
})
export class PlaygroundPreviewNodeComponent {
  readonly node = input.required<PlaygroundPreviewNode>();

  readonly primaryLabel = computed<string>(() => {
    const current = this.node();
    const preferred = current.attributes.find(
      (attr) =>
        attr.name === 'title' ||
        attr.name === 'label' ||
        attr.name === 'text' ||
        attr.name === 'placeholder',
    );
    if (preferred) {
      return preferred.value;
    }
    return current.textContent;
  });

  readonly isHorizontal = computed<boolean>(() => {
    const current = this.node();
    const orientation = current.attributes.find((attr) => attr.name === 'orientation')?.value;
    return (
      orientation === 'horizontal' ||
      current.nativeTag === 'ng-paned' ||
      current.nativeTag === 'ng-flow'
    );
  });

  readonly meterPercent = computed<number>(() => {
    const current = this.node();
    const fraction = current.attributes.find((attr) => attr.name === 'fraction')?.value;
    if (fraction) {
      const parsed = Number.parseFloat(fraction);
      if (Number.isFinite(parsed)) {
        return Math.max(4, Math.min(100, Math.round(parsed * 100)));
      }
    }
    const rawValue = current.attributes.find((attr) => attr.name === 'value')?.value;
    const rawMax = current.attributes.find((attr) => attr.name === 'max')?.value;
    const val = rawValue ? Number.parseFloat(rawValue) : 45;
    const max = rawMax ? Number.parseFloat(rawMax) : 100;
    if (Number.isFinite(val) && Number.isFinite(max) && max > 0) {
      return Math.max(4, Math.min(100, Math.round((val / max) * 100)));
    }
    return 45;
  });
}

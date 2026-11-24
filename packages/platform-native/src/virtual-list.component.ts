import { NgTemplateOutlet } from '@angular/common';
import {
  Component,
  computed,
  contentChild,
  effect,
  input,
  output,
  signal,
  TemplateRef,
} from '@angular/core';
import {
  NativeContainerDirective,
  NativeElementDirective,
  NativeListDirective,
  NativeTextWidgetDirective,
} from './native-directives';
import {
  computeVariableVirtualSliceRange,
  computeVirtualSliceRange,
  FenwickOffsetTree,
  type VirtualSliceRange,
} from './virtual-layout';

export { computeVirtualSliceRange, type VirtualSliceRange } from './virtual-layout';

/** Template context provided to each rendered item row in `<ng-virtual-list>`. */
export interface NativeVirtualItemContext<T> {
  $implicit: T;
  index: number;
}

/** Internal materialized slice entry paired with its stable tracking key. */
export interface NativeVirtualListEntry<T> {
  item: T;
  index: number;
  key: unknown;
}

/** Payload emitted when a row inside `<ng-virtual-list>` is activated natively. */
export interface NativeVirtualRowActivatedEvent<T> {
  index: number;
  item: T;
}

/**
 * Virtualized native desktop list component rendering only the visible viewport slice + overscan
 * inside an `<ng-scroll>` + `<ng-list>` hierarchy to keep native widget counts bounded at scale.
 * Supports fixed row heights as well as O(log n) Fenwick-tree variable/measured row heights.
 */
@Component({
  selector: 'ng-virtual-list',
  standalone: true,
  imports: [
    NgTemplateOutlet,
    NativeElementDirective,
    NativeContainerDirective,
    NativeListDirective,
    NativeTextWidgetDirective,
  ],
  template: `
    <ng-scroll
      [hexpand]="true"
      [vexpand]="true"
      [min-height]="viewportHeight()"
      [scroll-to]="scrollToAttr()"
      (scroll)="onNativeScroll($event)"
      (scrollBottom)="scrollBottom.emit()"
    >
      <ng-box orientation="vertical" [spacing]="0" [hexpand]="true">
        @if (range().topSpacerHeight > 0) {
          <ng-box [height]="range().topSpacerHeight"></ng-box>
        }
        <ng-list
          [selection-mode]="selectionMode()"
          [show-separators]="showSeparators()"
          (rowActivated)="onRowActivated($event)"
        >
          @for (entry of visibleEntries(); track entry.key) {
            @if (resolvedTemplate(); as tpl) {
              <ng-container
                [ngTemplateOutlet]="tpl"
                [ngTemplateOutletContext]="{ $implicit: entry.item, index: entry.index }"
              ></ng-container>
            } @else {
              <ng-label [text]="'' + entry.item"></ng-label>
            }
          }
        </ng-list>
        @if (range().bottomSpacerHeight > 0) {
          <ng-box [height]="range().bottomSpacerHeight"></ng-box>
        }
      </ng-box>
    </ng-scroll>
  `,
})
export class NativeVirtualListComponent<T = unknown> {
  readonly items = input<readonly T[]>([]);
  readonly itemHeight = input<number>(48);
  readonly itemHeightResolver = input<((index: number, item: T) => number) | null>(null);
  readonly viewportHeight = input<number>(480);
  readonly overscan = input<number>(4);
  readonly selectionMode = input<'none' | 'single' | 'browse' | 'multiple'>('single');
  readonly showSeparators = input<boolean>(false);
  readonly trackBy = input<(index: number, item: T) => unknown>((index, _item) => index);
  readonly itemTemplateInput = input<TemplateRef<NativeVirtualItemContext<T>> | null>(null, {
    alias: 'itemTemplate',
  });
  readonly projectedTemplate = contentChild<TemplateRef<NativeVirtualItemContext<T>>>(TemplateRef);

  readonly rangeChange = output<VirtualSliceRange>();
  readonly rowActivated = output<NativeVirtualRowActivatedEvent<T>>();
  readonly scrollBottom = output<void>();

  readonly scrollTop = signal(0);
  readonly scrollToAttr = signal<string | undefined>(undefined);

  private readonly heightTree = new FenwickOffsetTree(0, 48);
  private readonly measuredOverrides = new Map<number, number>();
  private readonly measureRevision = signal(0);

  readonly resolvedTemplate = computed(
    () => this.itemTemplateInput() ?? this.projectedTemplate() ?? null,
  );

  readonly isVariableHeight = computed(
    () =>
      this.itemHeightResolver() !== null ||
      (this.measureRevision() >= 0 && this.measuredOverrides.size > 0),
  );

  readonly range = computed<VirtualSliceRange>(() => {
    const source = this.items();
    const resolver = this.itemHeightResolver();
    this.measureRevision();
    const defaultHeight = Math.max(1, this.itemHeight());

    if (resolver === null && this.measuredOverrides.size === 0) {
      return computeVirtualSliceRange(
        source.length,
        this.scrollTop(),
        this.viewportHeight(),
        defaultHeight,
        this.overscan(),
      );
    }

    const overrides = new Map<number, number>();
    if (resolver !== null) {
      for (let i = 0; i < source.length; i += 1) {
        overrides.set(i, resolver(i, source[i]!));
      }
    }
    for (const [idx, h] of this.measuredOverrides) {
      if (idx >= 0 && idx < source.length) {
        overrides.set(idx, h);
      }
    }
    this.heightTree.reset(source.length, defaultHeight, overrides);
    return computeVariableVirtualSliceRange(
      this.heightTree,
      this.scrollTop(),
      this.viewportHeight(),
      this.overscan(),
    );
  });

  readonly visibleEntries = computed<readonly NativeVirtualListEntry<T>[]>(() => {
    const source = this.items();
    const { startIndex, endIndex } = this.range();
    const trackFn = this.trackBy();
    const entries: NativeVirtualListEntry<T>[] = [];
    for (let idx = startIndex; idx < endIndex; idx += 1) {
      const item = source[idx]!;
      entries.push({ item, index: idx, key: trackFn(idx, item) });
    }
    return entries;
  });

  private lastEmittedRangeKey = '';

  constructor() {
    effect(() => {
      const next = this.range();
      const key = `${next.startIndex}:${next.endIndex}:${next.totalItems}:${next.topSpacerHeight}:${next.bottomSpacerHeight}`;
      if (key !== this.lastEmittedRangeKey) {
        this.lastEmittedRangeKey = key;
        this.rangeChange.emit(next);
      }
    });
  }

  /** Records a dynamic measured pixel height for row `index` and updates viewport offsets. */
  measureItemHeight(index: number, height: number): void {
    const idx = Math.floor(index);
    if (!Number.isFinite(index) || idx < 0 || !Number.isFinite(height)) {
      return;
    }
    const clamped = Math.max(1, height);
    if (this.measuredOverrides.get(idx) !== clamped) {
      this.measuredOverrides.set(idx, clamped);
      this.measureRevision.update((rev) => rev + 1);
    }
  }

  /** Clears all dynamically measured height overrides. */
  resetMeasuredHeights(): void {
    if (this.measuredOverrides.size === 0) {
      return;
    }
    this.measuredOverrides.clear();
    this.measureRevision.update((rev) => rev + 1);
  }

  /** Returns the cumulative Y offset for row `index`. */
  offsetForIndex(index: number): number {
    const source = this.items();
    const idx = Math.max(0, Math.min(source.length, Math.floor(index)));
    this.range();
    if (!this.isVariableHeight()) {
      return idx * Math.max(1, this.itemHeight());
    }
    return Math.round(this.heightTree.offsetOf(idx));
  }

  /** Scrolls the native viewport to bring row `index` into view. */
  scrollToIndex(index: number): void {
    const y = this.offsetForIndex(index);
    this.scrollTop.set(y);
    this.scrollToAttr.set(`0,${y}`);
  }

  onNativeScroll(event: unknown): void {
    if (!event || typeof event !== 'object') {
      return;
    }
    const obj = event as Record<string, unknown>;
    let top: unknown = obj['scrollTop'];
    if (typeof top !== 'number' && obj['target'] && typeof obj['target'] === 'object') {
      top = (obj['target'] as Record<string, unknown>)['scrollTop'];
    }
    if (typeof top === 'number' && Number.isFinite(top)) {
      this.scrollTop.set(Math.max(0, top));
    }
  }

  onRowActivated(event: unknown): void {
    if (!event || typeof event !== 'object') {
      return;
    }
    const obj = event as Record<string, unknown>;
    const rawIdx =
      typeof obj['value'] === 'number'
        ? obj['value']
        : typeof obj['index'] === 'number'
          ? obj['index']
          : -1;
    const localIdx = Math.floor(rawIdx);
    const entry = this.visibleEntries()[localIdx];
    if (entry) {
      this.rowActivated.emit({ index: entry.index, item: entry.item });
    }
  }
}

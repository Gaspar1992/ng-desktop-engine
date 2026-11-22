import { NgTemplateOutlet } from '@angular/common';
import {
  ChangeDetectionStrategy,
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

/** Inclusive start / exclusive end index range and spacer heights for a virtualized viewport. */
export interface VirtualSliceRange {
  startIndex: number;
  endIndex: number;
  totalItems: number;
  topSpacerHeight: number;
  bottomSpacerHeight: number;
}

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
 * Computes the visible item slice and spacer heights for a fixed-row-height viewport.
 */
export function computeVirtualSliceRange(
  totalItems: number,
  scrollTop: number,
  viewportHeight: number,
  itemHeight: number,
  overscan = 4,
): VirtualSliceRange {
  const safeTotal = Math.max(0, Math.floor(totalItems));
  if (safeTotal === 0) {
    return {
      startIndex: 0,
      endIndex: 0,
      totalItems: 0,
      topSpacerHeight: 0,
      bottomSpacerHeight: 0,
    };
  }
  const safeItemHeight = Math.max(1, itemHeight);
  const safeViewport = Math.max(safeItemHeight, viewportHeight);
  const safeOverscan = Math.max(0, Math.floor(overscan));
  const maxScrollTop = Math.max(0, safeTotal * safeItemHeight - safeViewport);
  const clampedScrollTop = Math.min(Math.max(0, scrollTop), maxScrollTop);

  const firstVisible = Math.floor(clampedScrollTop / safeItemHeight);
  const visibleCount = Math.ceil(safeViewport / safeItemHeight);
  const startIndex = Math.max(0, firstVisible - safeOverscan);
  const endIndex = Math.min(safeTotal, firstVisible + visibleCount + safeOverscan);

  return {
    startIndex,
    endIndex,
    totalItems: safeTotal,
    topSpacerHeight: startIndex * safeItemHeight,
    bottomSpacerHeight: (safeTotal - endIndex) * safeItemHeight,
  };
}

/**
 * Virtualized native desktop list component rendering only the visible viewport slice + overscan
 * inside an `<ng-scroll>` + `<ng-list>` hierarchy to keep native widget counts bounded at scale.
 */
@Component({
  selector: 'ng-virtual-list',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
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
      (scroll)="onNativeScroll($event)"
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

  readonly scrollTop = signal(0);

  readonly resolvedTemplate = computed(
    () => this.itemTemplateInput() ?? this.projectedTemplate() ?? null,
  );

  readonly range = computed<VirtualSliceRange>(() =>
    computeVirtualSliceRange(
      this.items().length,
      this.scrollTop(),
      this.viewportHeight(),
      this.itemHeight(),
      this.overscan(),
    ),
  );

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
      const key = `${next.startIndex}:${next.endIndex}:${next.totalItems}`;
      if (key !== this.lastEmittedRangeKey) {
        this.lastEmittedRangeKey = key;
        this.rangeChange.emit(next);
      }
    });
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

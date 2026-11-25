import { NgTemplateOutlet } from '@angular/common';
import {
  Component,
  computed,
  contentChild,
  effect,
  ElementRef,
  inject,
  input,
  output,
  Renderer2,
  signal,
  TemplateRef,
  viewChild,
} from '@angular/core';
import {
  NativeContainerDirective,
  NativeElementDirective,
  NativeTextWidgetDirective,
} from './native-directives';
import { NativeElementNode } from './native-element';
import {
  computeVirtualGridSliceRange,
  FenwickOffsetTree,
  type VirtualGridSliceRange,
} from './virtual-layout';

/** Template context provided to each rendered cell inside `<ng-virtual-grid>`. */
export interface NativeVirtualGridItemContext<T> {
  $implicit: T;
  index: number;
  row: number;
  col: number;
}

/** Internal materialized 2D cell entry with row/column placement and tracking key. */
export interface NativeVirtualGridEntry<T> {
  item: T;
  index: number;
  row: number;
  col: number;
  key: unknown;
}

/**
 * 2D virtualized native grid rendering only visible rows + overscan inside `<ng-scroll>` + `<ng-grid>`.
 * Supports responsive column sizing from viewport width as well as variable row heights via `FenwickOffsetTree`.
 */
@Component({
  selector: 'ng-virtual-grid',
  imports: [
    NgTemplateOutlet,
    NativeElementDirective,
    NativeContainerDirective,
    NativeTextWidgetDirective,
  ],
  template: `
    <ng-scroll
      #scrollHost
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
        @if (items().length === 0) {
          <ng-content select="[ngVirtualEmpty]"></ng-content>
        } @else {
          <ng-grid
            [attr.id]="gridId() || null"
            [class]="gridClass()"
            [columns]="range().columns"
            [row-spacing]="rowGap()"
            [column-spacing]="columnGap()"
            [hexpand]="true"
          >
            @for (entry of visibleEntries(); track entry.key) {
              @if (resolvedTemplate(); as tpl) {
                <ng-container
                  [ngTemplateOutlet]="tpl"
                  [ngTemplateOutletContext]="{
                    $implicit: entry.item,
                    index: entry.index,
                    row: entry.row,
                    col: entry.col,
                  }"
                ></ng-container>
              } @else {
                <ng-label [text]="'' + entry.item"></ng-label>
              }
            }
          </ng-grid>
        }
        @if (range().bottomSpacerHeight > 0) {
          <ng-box [height]="range().bottomSpacerHeight"></ng-box>
        }
      </ng-box>
    </ng-scroll>
  `,
})
export class NativeVirtualGridComponent<T = unknown> {
  readonly items = input<readonly T[]>([]);
  readonly rowHeight = input<number>(200);
  readonly minColumnWidth = input<number>(200);
  readonly columns = input<number | null>(null);
  readonly viewportHeight = input<number>(480);
  readonly viewportWidth = input<number>(960);
  readonly rowGap = input<number>(12);
  readonly columnGap = input<number>(12);
  readonly overscanRows = input<number>(2);
  readonly gridClass = input<string>('');
  readonly gridId = input<string>('');
  readonly rowHeightResolver = input<((rowIndex: number) => number) | null>(null);
  readonly trackBy = input<(index: number, item: T) => unknown>((index, _item) => index);
  readonly itemTemplateInput = input<TemplateRef<NativeVirtualGridItemContext<T>> | null>(null, {
    alias: 'itemTemplate',
  });
  readonly projectedTemplate =
    contentChild<TemplateRef<NativeVirtualGridItemContext<T>>>(TemplateRef);

  readonly rangeChange = output<VirtualGridSliceRange>();
  readonly scrollBottom = output<void>();

  private readonly renderer = inject(Renderer2);
  private readonly scrollHost = viewChild<ElementRef<NativeElementNode>>('scrollHost');

  readonly scrollTop = signal(0);
  readonly measuredWidth = signal<number | null>(null);
  readonly measuredHeight = signal<number | null>(null);
  readonly scrollToAttr = signal<string | undefined>(undefined);

  private readonly rowTree = new FenwickOffsetTree(0, 200);
  private readonly rowOverrides = new Map<number, number>();
  private readonly measureRevision = signal(0);

  readonly resolvedTemplate = computed(
    () => this.itemTemplateInput() ?? this.projectedTemplate() ?? null,
  );

  readonly effectiveWidth = computed(() => this.measuredWidth() ?? this.viewportWidth());
  readonly effectiveHeight = computed(() => this.measuredHeight() ?? this.viewportHeight());

  /** Builds/updates the Fenwick row-offset tree only when layout geometry or items change, not on scroll. */
  private readonly rowHeightTreeState = computed(() => {
    const source = this.items();
    const resolver = this.rowHeightResolver();
    this.measureRevision();
    const baseRowHeight = Math.max(1, this.rowHeight());
    const effWidth = this.effectiveWidth();
    const effHeight = this.effectiveHeight();

    const geometrySlice = computeVirtualGridSliceRange({
      totalItems: source.length,
      scrollTop: 0,
      viewportHeight: effHeight,
      viewportWidth: effWidth,
      rowHeight: baseRowHeight,
      minColumnWidth: this.minColumnWidth(),
      columns: this.columns(),
      rowGap: this.rowGap(),
      columnGap: this.columnGap(),
      overscanRows: this.overscanRows(),
    });

    if (resolver === null && this.rowOverrides.size === 0) {
      return {
        tree: null as FenwickOffsetTree | null,
        baseRowHeight,
        columns: geometrySlice.columns,
      };
    }

    const totalRows = geometrySlice.totalRows;
    const overrides = new Map<number, number>();
    if (resolver !== null) {
      for (let r = 0; r < totalRows; r += 1) {
        overrides.set(r, resolver(r));
      }
    }
    for (const [r, h] of this.rowOverrides) {
      if (r >= 0 && r < totalRows) {
        overrides.set(r, h);
      }
    }
    this.rowTree.reset(totalRows, baseRowHeight, overrides);
    return { tree: this.rowTree, baseRowHeight, columns: geometrySlice.columns };
  });

  readonly range = computed<VirtualGridSliceRange>(() => {
    const source = this.items();
    const { tree, baseRowHeight, columns } = this.rowHeightTreeState();

    return computeVirtualGridSliceRange({
      totalItems: source.length,
      scrollTop: this.scrollTop(),
      viewportHeight: this.effectiveHeight(),
      viewportWidth: this.effectiveWidth(),
      rowHeight: baseRowHeight,
      minColumnWidth: this.minColumnWidth(),
      columns,
      rowGap: this.rowGap(),
      columnGap: this.columnGap(),
      overscanRows: this.overscanRows(),
      rowHeights: tree ?? undefined,
    });
  });

  readonly visibleEntries = computed<readonly NativeVirtualGridEntry<T>[]>(() => {
    const source = this.items();
    const { startIndex, endIndex, columns } = this.range();
    const trackFn = this.trackBy();
    const entries: NativeVirtualGridEntry<T>[] = [];
    for (let idx = startIndex; idx < endIndex; idx += 1) {
      const item = source[idx]!;
      const localIdx = idx - startIndex;
      const row = Math.floor(localIdx / columns);
      const col = localIdx % columns;
      entries.push({ item, index: idx, row, col, key: trackFn(idx, item) });
    }
    return entries;
  });

  private lastEmittedRangeKey = '';

  constructor() {
    effect(() => {
      const next = this.range();
      const key = `${next.startIndex}:${next.endIndex}:${next.columns}:${next.totalItems}:${next.topSpacerHeight}:${next.bottomSpacerHeight}`;
      if (key !== this.lastEmittedRangeKey) {
        this.lastEmittedRangeKey = key;
        this.rangeChange.emit(next);
      }
    });
  }

  /** Overrides the height of grid row `rowIndex` in O(log R). */
  measureRowHeight(rowIndex: number, height: number): void {
    const row = Math.floor(rowIndex);
    if (!Number.isFinite(rowIndex) || row < 0 || !Number.isFinite(height)) {
      return;
    }
    const clamped = Math.max(1, height);
    if (this.rowOverrides.get(row) !== clamped) {
      this.rowOverrides.set(row, clamped);
      this.measureRevision.update((rev) => rev + 1);
    }
  }

  /** Clears all dynamically measured row height overrides. */
  resetMeasuredRows(): void {
    if (this.rowOverrides.size === 0) {
      return;
    }
    this.rowOverrides.clear();
    this.measureRevision.update((rev) => rev + 1);
  }

  /** Scrolls the native grid viewport to bring item `index` into view. */
  scrollToIndex(index: number): void {
    const safeTotal = this.items().length;
    if (safeTotal === 0) {
      return;
    }
    const clampedIdx = Math.max(0, Math.min(safeTotal - 1, Math.floor(index)));
    const slice = this.range();
    const targetRow = Math.floor(clampedIdx / Math.max(1, slice.columns));
    const rowGap = Math.max(0, this.rowGap());
    const y =
      this.rowHeightResolver() !== null || this.rowOverrides.size > 0
        ? Math.round(this.rowTree.offsetOf(targetRow) + targetRow * rowGap)
        : Math.round(targetRow * (Math.max(1, this.rowHeight()) + rowGap));
    const coords = `0,${y}`;
    const sameCoords = this.scrollToAttr() === coords;
    this.scrollTop.set(y);
    this.scrollToAttr.set(coords);
    if (sameCoords) {
      const hostEl = this.scrollHost()?.nativeElement;
      if (hostEl) {
        this.renderer.setAttribute(hostEl, 'scroll-to', coords);
      }
    }
  }

  onNativeScroll(event: unknown): void {
    if (!event || typeof event !== 'object') {
      return;
    }
    const obj = event as Record<string, unknown>;
    const target =
      obj['target'] && typeof obj['target'] === 'object'
        ? (obj['target'] as Record<string, unknown>)
        : undefined;

    const top = typeof obj['scrollTop'] === 'number' ? obj['scrollTop'] : target?.['scrollTop'];
    if (typeof top === 'number' && Number.isFinite(top)) {
      this.scrollTop.set(Math.max(0, top));
    }
    const width =
      typeof obj['clientWidth'] === 'number' ? obj['clientWidth'] : target?.['clientWidth'];
    if (typeof width === 'number' && Number.isFinite(width) && width > 0) {
      this.measuredWidth.set(width);
    }
    const height =
      typeof obj['clientHeight'] === 'number' ? obj['clientHeight'] : target?.['clientHeight'];
    if (typeof height === 'number' && Number.isFinite(height) && height > 0) {
      this.measuredHeight.set(height);
    }
  }
}

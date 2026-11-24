/** Inclusive start / exclusive end index range and spacer heights for a virtualized viewport. */
export interface VirtualSliceRange {
  startIndex: number;
  endIndex: number;
  totalItems: number;
  topSpacerHeight: number;
  bottomSpacerHeight: number;
}

/** Input parameters for 2D responsive virtualized grid slicing. */
export interface VirtualGridSliceOptions {
  totalItems: number;
  scrollTop: number;
  viewportHeight: number;
  viewportWidth: number;
  rowHeight: number;
  minColumnWidth?: number;
  columns?: number | null;
  rowGap?: number;
  columnGap?: number;
  overscanRows?: number;
  rowHeights?: FenwickOffsetTree | null;
}

/** Inclusive start / exclusive end slice metadata for a 2D virtualized grid viewport. */
export interface VirtualGridSliceRange {
  startIndex: number;
  endIndex: number;
  startRow: number;
  endRow: number;
  totalRows: number;
  columns: number;
  totalItems: number;
  topSpacerHeight: number;
  bottomSpacerHeight: number;
}

function sanitizePositiveSize(value: number, fallback: number): number {
  if (!Number.isFinite(value)) {
    return fallback;
  }
  return Math.max(1, value);
}

/**
 * Binary Indexed Tree (Fenwick tree) supporting O(log n) updates, prefix sums, and offset lookups
 * for variable-height virtualized lists and grids.
 */
export class FenwickOffsetTree {
  private sizes: Float64Array;
  private tree: Float64Array;
  private itemCount: number;
  private fallbackSize: number;
  private cachedTotal: number;

  constructor(count = 0, defaultSize = 48) {
    this.sizes = new Float64Array(0);
    this.tree = new Float64Array(1);
    this.itemCount = 0;
    this.fallbackSize = sanitizePositiveSize(defaultSize, 48);
    this.cachedTotal = 0;
    this.reset(count, this.fallbackSize);
  }

  get length(): number {
    return this.itemCount;
  }

  /** Rebuilds the tree in O(n) time from `count`, `defaultSize`, and optional per-index overrides. */
  reset(count: number, defaultSize: number, overrides?: ReadonlyMap<number, number>): void {
    const safeCount = Math.max(0, Math.floor(Number.isFinite(count) ? count : 0));
    const safeDefault = sanitizePositiveSize(defaultSize, 48);
    this.itemCount = safeCount;
    this.fallbackSize = safeDefault;
    this.sizes = new Float64Array(safeCount);
    this.tree = new Float64Array(safeCount + 1);

    let total = 0;
    for (let idx = 0; idx < safeCount; idx += 1) {
      const override = overrides?.get(idx);
      const size =
        override !== undefined ? sanitizePositiveSize(override, safeDefault) : safeDefault;
      this.sizes[idx] = size;
      total += size;
      this.tree[idx + 1] = size;
    }
    this.cachedTotal = total;

    for (let pos = 1; pos <= safeCount; pos += 1) {
      const parent = pos + (pos & -pos);
      if (parent <= safeCount) {
        this.tree[parent] = (this.tree[parent] ?? 0) + (this.tree[pos] ?? 0);
      }
    }
  }

  /** Returns the height/size recorded for `index` in O(1). */
  sizeOf(index: number): number {
    const idx = Math.floor(index);
    if (idx < 0 || idx >= this.itemCount) {
      return this.fallbackSize;
    }
    return this.sizes[idx] ?? this.fallbackSize;
  }

  /** Updates the height/size at `index` in O(log n); returns `true` when it changed. */
  setSize(index: number, size: number): boolean {
    const idx = Math.floor(index);
    if (!Number.isFinite(index) || idx < 0 || idx >= this.itemCount) {
      return false;
    }
    const next = sanitizePositiveSize(size, this.fallbackSize);
    const prev = this.sizes[idx] ?? this.fallbackSize;
    if (Math.abs(next - prev) < 1e-6) {
      return false;
    }
    const delta = next - prev;
    this.sizes[idx] = next;
    this.cachedTotal += delta;
    for (let pos = idx + 1; pos <= this.itemCount; pos += pos & -pos) {
      this.tree[pos] = (this.tree[pos] ?? 0) + delta;
    }
    return true;
  }

  /** Returns the exclusive prefix sum covering `[0, index)` in O(log n). */
  offsetOf(index: number): number {
    if (this.itemCount === 0 || !Number.isFinite(index)) {
      return 0;
    }
    const clamped = Math.max(0, Math.min(this.itemCount, Math.floor(index)));
    let sum = 0;
    for (let pos = clamped; pos > 0; pos -= pos & -pos) {
      sum += this.tree[pos] ?? 0;
    }
    return sum;
  }

  /** Returns total size of all items in O(1). */
  totalSize(): number {
    return this.cachedTotal;
  }

  /** Returns the first 0-based index covering `offset` via O(log n) binary lifting. */
  indexAtOffset(offset: number): number {
    if (this.itemCount === 0 || !Number.isFinite(offset) || offset <= 0) {
      return 0;
    }
    if (offset >= this.cachedTotal) {
      return this.itemCount - 1;
    }

    let maxBit = 1;
    while (maxBit << 1 <= this.itemCount) {
      maxBit <<= 1;
    }

    let pos = 0;
    let acc = 0;
    for (let bit = maxBit; bit > 0; bit >>= 1) {
      const next = pos + bit;
      if (next <= this.itemCount) {
        const candidate = acc + (this.tree[next] ?? 0);
        if (candidate <= offset) {
          pos = next;
          acc = candidate;
        }
      }
    }
    return Math.min(this.itemCount - 1, pos);
  }
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
  const safeItemHeight = sanitizePositiveSize(itemHeight, 1);
  const safeViewport = Math.max(
    safeItemHeight,
    Number.isFinite(viewportHeight) ? viewportHeight : safeItemHeight,
  );
  const safeOverscan = Math.max(0, Math.floor(overscan));
  const maxScrollTop = Math.max(0, safeTotal * safeItemHeight - safeViewport);
  const clampedScrollTop = Math.min(
    Math.max(0, Number.isFinite(scrollTop) ? scrollTop : 0),
    maxScrollTop,
  );

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
 * Computes the visible item slice and exact top/bottom spacer heights from a variable-height `FenwickOffsetTree`.
 */
export function computeVariableVirtualSliceRange(
  tree: FenwickOffsetTree,
  scrollTop: number,
  viewportHeight: number,
  overscan = 4,
): VirtualSliceRange {
  const safeTotal = tree.length;
  if (safeTotal === 0) {
    return {
      startIndex: 0,
      endIndex: 0,
      totalItems: 0,
      topSpacerHeight: 0,
      bottomSpacerHeight: 0,
    };
  }

  const totalHeight = tree.totalSize();
  const safeViewport = sanitizePositiveSize(viewportHeight, 1);
  const safeOverscan = Math.max(0, Math.floor(overscan));
  const maxScrollTop = Math.max(0, totalHeight - safeViewport);
  const clampedScrollTop = Math.min(
    Math.max(0, Number.isFinite(scrollTop) ? scrollTop : 0),
    maxScrollTop,
  );

  const firstVisible = tree.indexAtOffset(clampedScrollTop);
  const viewportBottom = clampedScrollTop + safeViewport;
  const endVisible =
    viewportBottom >= totalHeight
      ? safeTotal
      : Math.min(
          safeTotal,
          tree.indexAtOffset(Math.max(clampedScrollTop, viewportBottom - 1e-6)) + 1,
        );

  const startIndex = Math.max(0, firstVisible - safeOverscan);
  const endIndex = Math.min(safeTotal, endVisible + safeOverscan);

  return {
    startIndex,
    endIndex,
    totalItems: safeTotal,
    topSpacerHeight: Math.round(tree.offsetOf(startIndex)),
    bottomSpacerHeight: Math.round(Math.max(0, totalHeight - tree.offsetOf(endIndex))),
  };
}

function findRowAtVariableOffset(
  rowHeights: FenwickOffsetTree,
  totalRows: number,
  rowGap: number,
  offset: number,
): number {
  if (offset <= 0) {
    return 0;
  }
  let lo = 0;
  let hi = totalRows - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    const rowStart = rowHeights.offsetOf(mid) + mid * rowGap;
    if (rowStart <= offset) {
      lo = mid;
    } else {
      hi = mid - 1;
    }
  }
  return lo;
}

/**
 * Computes the visible 2D item range, row span, responsive column count, and spacer heights for `<ng-virtual-grid>`.
 */
export function computeVirtualGridSliceRange(
  options: VirtualGridSliceOptions,
): VirtualGridSliceRange {
  const safeTotal = Math.max(
    0,
    Math.floor(Number.isFinite(options.totalItems) ? options.totalItems : 0),
  );
  const safeColGap = Math.max(0, Number.isFinite(options.columnGap) ? options.columnGap! : 0);
  const safeRowGap = Math.max(0, Number.isFinite(options.rowGap) ? options.rowGap! : 0);
  const safeWidth = sanitizePositiveSize(options.viewportWidth, 320);
  const safeMinColWidth = sanitizePositiveSize(options.minColumnWidth ?? 200, 200);
  const safeRowHeight = sanitizePositiveSize(options.rowHeight, 48);
  const safeViewportHeight = sanitizePositiveSize(options.viewportHeight, safeRowHeight);
  const safeOverscan = Math.max(0, Math.floor(options.overscanRows ?? 2));

  const columns =
    typeof options.columns === 'number' && Number.isFinite(options.columns) && options.columns >= 1
      ? Math.floor(options.columns)
      : Math.max(1, Math.floor((safeWidth + safeColGap) / (safeMinColWidth + safeColGap)));

  if (safeTotal === 0) {
    return {
      startIndex: 0,
      endIndex: 0,
      startRow: 0,
      endRow: 0,
      totalRows: 0,
      columns,
      totalItems: 0,
      topSpacerHeight: 0,
      bottomSpacerHeight: 0,
    };
  }

  const totalRows = Math.ceil(safeTotal / columns);
  const rowHeights =
    options.rowHeights && options.rowHeights.length === totalRows ? options.rowHeights : null;
  const gapTotal = Math.max(0, totalRows - 1) * safeRowGap;
  const totalHeight = rowHeights
    ? rowHeights.totalSize() + gapTotal
    : totalRows * safeRowHeight + gapTotal;

  const maxScrollTop = Math.max(0, totalHeight - safeViewportHeight);
  const clampedScrollTop = Math.min(
    Math.max(0, Number.isFinite(options.scrollTop) ? options.scrollTop : 0),
    maxScrollTop,
  );

  let firstVisibleRow: number;
  let endVisibleRow: number;
  if (rowHeights) {
    firstVisibleRow = findRowAtVariableOffset(rowHeights, totalRows, safeRowGap, clampedScrollTop);
    const viewportBottom = clampedScrollTop + safeViewportHeight;
    endVisibleRow =
      viewportBottom >= totalHeight
        ? totalRows
        : Math.min(
            totalRows,
            findRowAtVariableOffset(
              rowHeights,
              totalRows,
              safeRowGap,
              Math.max(clampedScrollTop, viewportBottom - 1e-6),
            ) + 1,
          );
  } else {
    const stride = safeRowHeight + safeRowGap;
    firstVisibleRow = Math.min(totalRows - 1, Math.floor(clampedScrollTop / stride));
    const lastOffset = Math.max(clampedScrollTop, clampedScrollTop + safeViewportHeight - 1e-6);
    endVisibleRow = Math.min(totalRows, Math.floor(lastOffset / stride) + 1);
  }

  const startRow = Math.max(0, firstVisibleRow - safeOverscan);
  const endRow = Math.min(totalRows, endVisibleRow + safeOverscan);
  const startIndex = startRow * columns;
  const endIndex = Math.min(safeTotal, endRow * columns);

  const topSpacerHeight = rowHeights
    ? Math.round(rowHeights.offsetOf(startRow) + startRow * safeRowGap)
    : Math.round(startRow * (safeRowHeight + safeRowGap));

  const renderedEndOffset = rowHeights
    ? rowHeights.offsetOf(endRow) + Math.max(0, endRow - 1) * safeRowGap
    : endRow * safeRowHeight + Math.max(0, endRow - 1) * safeRowGap;
  const bottomSpacerHeight = Math.round(Math.max(0, totalHeight - renderedEndOffset));

  return {
    startIndex,
    endIndex,
    startRow,
    endRow,
    totalRows,
    columns,
    totalItems: safeTotal,
    topSpacerHeight,
    bottomSpacerHeight,
  };
}

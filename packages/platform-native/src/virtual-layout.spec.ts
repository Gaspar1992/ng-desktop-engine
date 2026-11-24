import { Component, signal, viewChild } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import {
  computeVariableVirtualSliceRange,
  computeVirtualGridSliceRange,
  ensureNgNativeBridge,
  FenwickOffsetTree,
  NATIVE_UI_DIRECTIVES,
  NativeRendererFactory2,
  NativeVirtualGridComponent,
  NativeVirtualListComponent,
  provideNativePlatform,
  resolveNativeTag,
  type VirtualGridSliceRange,
  type VirtualSliceRange,
} from './index';

describe('FenwickOffsetTree & variable-height / 2D grid virtualization', () => {
  it('maintains O(log n) prefix sums, point updates, and binary-lifted offset lookups', () => {
    const empty = new FenwickOffsetTree(0, 40);
    expect(empty.length).toBe(0);
    expect(empty.totalSize()).toBe(0);
    expect(empty.offsetOf(5)).toBe(0);
    expect(empty.indexAtOffset(100)).toBe(0);
    expect(empty.setSize(0, 80)).toBe(false);

    const tree = new FenwickOffsetTree(5, 40);
    expect(tree.length).toBe(5);
    expect(tree.totalSize()).toBe(200);
    expect(tree.offsetOf(0)).toBe(0);
    expect(tree.offsetOf(3)).toBe(120);
    expect(tree.indexAtOffset(0)).toBe(0);
    expect(tree.indexAtOffset(39.5)).toBe(0);
    expect(tree.indexAtOffset(40)).toBe(1);
    expect(tree.indexAtOffset(159)).toBe(3);
    expect(tree.indexAtOffset(999)).toBe(4);

    // Update index 1 from 40 -> 100 and index 3 from 40 -> 60 (clamping invalid heights to >= 1)
    expect(tree.setSize(1, 100)).toBe(true);
    expect(tree.setSize(1, 100)).toBe(false);
    expect(tree.setSize(3, 60)).toBe(true);
    expect(tree.setSize(4, -5)).toBe(true);
    expect(tree.sizeOf(4)).toBe(1);

    // Sizes: [40, 100, 40, 60, 1] -> total = 241
    // Offsets: [0, 40, 140, 180, 240]
    expect(tree.totalSize()).toBe(241);
    expect(tree.offsetOf(2)).toBe(140);
    expect(tree.offsetOf(4)).toBe(240);
    expect(tree.indexAtOffset(40)).toBe(1);
    expect(tree.indexAtOffset(139)).toBe(1);
    expect(tree.indexAtOffset(140)).toBe(2);
    expect(tree.indexAtOffset(180)).toBe(3);
    expect(tree.indexAtOffset(240)).toBe(4);
  });

  it('computes accurate variable-height virtual slice ranges', () => {
    const tree = new FenwickOffsetTree(100, 40);
    for (let i = 0; i < 100; i += 2) {
      tree.setSize(i, 80); // alternating 80, 40 -> 60px avg
    }
    const emptyTree = new FenwickOffsetTree(0, 40);
    expect(computeVariableVirtualSliceRange(emptyTree, 0, 200, 2)).toEqual({
      startIndex: 0,
      endIndex: 0,
      totalItems: 0,
      topSpacerHeight: 0,
      bottomSpacerHeight: 0,
    });

    const top = computeVariableVirtualSliceRange(tree, 0, 160, 1);
    // [0..160]: idx 0 (80), idx 1 (40 -> 120), idx 2 (80 -> 200) -> visible 0..3 + overscan 1 -> 0..4
    expect(top.startIndex).toBe(0);
    expect(top.endIndex).toBe(4);
    expect(top.topSpacerHeight).toBe(0);
    expect(top.bottomSpacerHeight).toBe(tree.totalSize() - tree.offsetOf(4));

    const mid = computeVariableVirtualSliceRange(tree, 600, 160, 1);
    expect(mid.startIndex).toBe(9);
    expect(mid.endIndex).toBe(14);
    expect(mid.topSpacerHeight).toBe(tree.offsetOf(9));
    expect(mid.bottomSpacerHeight).toBe(tree.totalSize() - tree.offsetOf(14));
  });

  it('computes 2D virtual grid ranges across responsive columns and variable row heights', () => {
    const responsive = computeVirtualGridSliceRange({
      totalItems: 100,
      scrollTop: 0,
      viewportHeight: 400,
      viewportWidth: 660,
      rowHeight: 150,
      minColumnWidth: 200,
      rowGap: 12,
      columnGap: 12,
      overscanRows: 1,
    });
    // (660 + 12) / (200 + 12) = 3 columns
    expect(responsive.columns).toBe(3);
    expect(responsive.totalRows).toBe(34);
    expect(responsive.startRow).toBe(0);
    expect(responsive.endRow).toBe(4);
    expect(responsive.startIndex).toBe(0);
    expect(responsive.endIndex).toBe(12);
    expect(responsive.topSpacerHeight).toBe(0);

    const rowTree = new FenwickOffsetTree(20, 100);
    rowTree.setSize(0, 180);
    rowTree.setSize(2, 160);
    const variableGrid = computeVirtualGridSliceRange({
      totalItems: 80,
      scrollTop: 292,
      viewportHeight: 220,
      viewportWidth: 850,
      rowHeight: 100,
      columns: 4,
      rowGap: 12,
      columnGap: 12,
      overscanRows: 0,
      rowHeights: rowTree,
    });
    // Row 0: [0..180), Row 1: [192..292), Row 2: [304..464), Row 3: [476..576)
    expect(variableGrid.columns).toBe(4);
    expect(variableGrid.startRow).toBe(1);
    expect(variableGrid.endRow).toBe(4);
    expect(variableGrid.startIndex).toBe(4);
    expect(variableGrid.endIndex).toBe(16);
    expect(variableGrid.topSpacerHeight).toBe(192);
  });

  it('supports variable/measured row heights and scrollToIndex on <ng-virtual-list>', () => {
    interface VarRow {
      id: number;
      tall: boolean;
    }

    @Component({
      standalone: true,
      imports: [NATIVE_UI_DIRECTIVES, NativeVirtualListComponent],
      template: `
        <ng-window title="Variable List">
          <ng-virtual-list
            [items]="rows()"
            [itemHeight]="40"
            [itemHeightResolver]="resolveHeight"
            [viewportHeight]="200"
            [overscan]="1"
            (rangeChange)="lastRange.set($event)"
            (scrollBottom)="bottomCount.set(bottomCount() + 1)"
          >
            <ng-template let-item let-idx="index">
              <ng-label [text]="idx + ':' + item.id"></ng-label>
            </ng-template>
          </ng-virtual-list>
        </ng-window>
      `,
    })
    class VariableListHost {
      readonly vList = viewChild.required(NativeVirtualListComponent);
      readonly rows = signal<VarRow[]>(
        Array.from({ length: 200 }, (_, i) => ({ id: i, tall: i % 3 === 0 })),
      );
      readonly resolveHeight = (_idx: number, row: VarRow): number => (row.tall ? 90 : 30);
      readonly lastRange = signal<VirtualSliceRange | null>(null);
      readonly bottomCount = signal(0);
    }

    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [VariableListHost],
      providers: [provideNativePlatform()],
    });
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();

    const fixture = TestBed.createComponent(VariableListHost);
    fixture.detectChanges();
    const initCmds = bridge.__drainQueue().split('\n');
    const scrollId = initCmds.find((l) => l.endsWith(' ng-scroll'))!.split(' ')[1]!;

    const vList = fixture.componentInstance.vList() as NativeVirtualListComponent<VarRow>;
    // idx 0=90, 1=30, 2=30, 3=90 -> offsetOf(3) = 150
    expect(vList.offsetForIndex(3)).toBe(150);

    vList.measureItemHeight(1, 70);
    fixture.detectChanges();
    expect(vList.offsetForIndex(3)).toBe(190);

    vList.scrollToIndex(30);
    fixture.detectChanges();
    const scrolledCmds = bridge.__drainQueue();
    expect(scrolledCmds).toContain('scroll-to 0,');
    expect(fixture.componentInstance.lastRange()!.startIndex).toBeGreaterThanOrEqual(28);

    bridge.__dispatchEvent(scrollId, 'scroll-bottom', {});
    expect(fixture.componentInstance.bottomCount()).toBe(1);
    fixture.destroy();
  });

  it('virtualizes 1,000 items in 2D <ng-virtual-grid>, renders [ngVirtualEmpty], and binds grid attrs', () => {
    interface GridItem {
      id: number;
      title: string;
    }

    @Component({
      standalone: true,
      imports: [NATIVE_UI_DIRECTIVES, NativeVirtualGridComponent],
      template: `
        <ng-window title="Virtual Grid">
          <ng-virtual-grid
            gridId="poke_virtual_grid"
            gridClass="pokemon-grid"
            [items]="items()"
            [rowHeight]="120"
            [minColumnWidth]="200"
            [viewportHeight]="260"
            [viewportWidth]="660"
            [rowGap]="10"
            [columnGap]="12"
            [overscanRows]="1"
            (rangeChange)="lastRange.set($event)"
          >
            <ng-template let-item>
              <ng-box [col]="item.id % 3" [row]="0">
                <ng-label [text]="item.title"></ng-label>
              </ng-box>
            </ng-template>
            <ng-box ngVirtualEmpty id="empty_grid_slot">
              <ng-label text="No items"></ng-label>
            </ng-box>
          </ng-virtual-grid>
        </ng-window>
      `,
    })
    class VirtualGridHost {
      readonly items = signal<GridItem[]>(
        Array.from({ length: 1000 }, (_, i) => ({ id: i, title: `Mon ${i}` })),
      );
      readonly lastRange = signal<VirtualGridSliceRange | null>(null);
    }

    expect(resolveNativeTag('ng-virtual-grid')).toBeNull();

    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [VirtualGridHost],
      providers: [provideNativePlatform()],
    });
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();

    const fixture = TestBed.createComponent(VirtualGridHost);
    fixture.detectChanges();
    const initCmds = bridge.__drainQueue().split('\n');

    expect(initCmds.some((l) => l.includes('ng-virtual-grid'))).toBe(false);
    const gridLine = initCmds.find((l) => l.endsWith(' ng-grid'));
    const scrollLine = initCmds.find((l) => l.endsWith(' ng-scroll'));
    expect(gridLine).toBeDefined();
    expect(scrollLine).toBeDefined();

    const gridId = gridLine!.split(' ')[1]!;
    const scrollId = scrollLine!.split(' ')[1]!;
    const factory = TestBed.inject(NativeRendererFactory2);
    expect(factory.getNode(gridId)?.attributes.get('columns')).toBe('3');
    expect(initCmds.some((l) => l.startsWith('attr ') && l.endsWith(' col 0'))).toBe(true);
    expect(initCmds.some((l) => l.startsWith('attr ') && l.endsWith(' row 0'))).toBe(true);

    // Total items = 1,000 across 334 rows -> only initial viewport (3 rows * 3 cols = 9 items) attached inside ng-grid
    const renderedItemTexts = initCmds.filter((l) => l.startsWith('text ') && l.includes('Mon '));
    expect(renderedItemTexts.length).toBe(9);
    expect(fixture.componentInstance.lastRange()).toMatchObject({
      columns: 3,
      startIndex: 0,
      endIndex: 9,
      totalRows: 334,
      totalItems: 1000,
    });

    // Scroll down + trigger responsive viewport resize via clientWidth=860 -> 4 columns
    bridge.__dispatchEvent(scrollId, 'scroll', {
      value: '{"scrollTop":1300,"clientWidth":860,"clientHeight":260}',
    });
    fixture.detectChanges();
    const scrolledSlice = fixture.componentInstance.lastRange()!;
    expect(scrolledSlice.columns).toBe(4);
    expect(scrolledSlice.startRow).toBe(9);
    expect(scrolledSlice.startIndex).toBe(36);

    // Switch to empty -> grid destroys and projected [ngVirtualEmpty] slot inserts into container
    fixture.componentInstance.items.set([]);
    fixture.detectChanges();
    const emptyCmds = bridge.__drainQueue();
    expect(emptyCmds).toContain(`destroy ${gridId}`);
    expect(emptyCmds).toContain('insert_before ');

    fixture.destroy();
  });
});

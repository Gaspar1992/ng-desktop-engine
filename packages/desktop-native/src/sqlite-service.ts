import { computed, DestroyRef, inject, Injectable, Signal, signal } from '@angular/core';
import { ensureNgNativeBridge } from '@ng-desktop/platform';

export interface NativeLiveQueryRef<T> {
  readonly rows: Signal<T[]>;
  readonly count: Signal<number>;
  refresh(): T[];
  destroy(): void;
}

/**
 * Reactive native SQLite service backed by Rust `libsqlite3.so.0` FFI (`NativeSqliteDatabase`)
 * with Signal-based `liveQuery()` subscriptions that refresh automatically after mutations.
 */
@Injectable({ providedIn: 'root' })
export class NativeSqliteService {
  private readonly bridge = ensureNgNativeBridge();
  private readonly liveQueryRefreshers = new Set<() => unknown>();

  readonly mutationVersion = signal(0);

  execute(sql: string): boolean {
    const ok = this.bridge.sqliteExecute(sql);
    if (ok) {
      this.mutationVersion.update((v) => v + 1);
      for (const refresh of this.liveQueryRefreshers) {
        refresh();
      }
    }
    return ok;
  }

  query<T = Record<string, string>>(sql: string, mapRow?: (row: Record<string, string>) => T): T[] {
    const rawRows = this.bridge.sqliteQuery(sql);
    return mapRow ? rawRows.map(mapRow) : (rawRows as unknown as T[]);
  }

  liveQuery<T = Record<string, string>>(
    sqlOrFn: string | (() => string),
    mapRow?: (row: Record<string, string>) => T,
  ): NativeLiveQueryRef<T> {
    const rowsSignal = signal<T[]>([]);
    const refresh = (): T[] => {
      const sql = typeof sqlOrFn === 'function' ? sqlOrFn() : sqlOrFn;
      const next = this.query(sql, mapRow);
      rowsSignal.set(next);
      return next;
    };

    refresh();
    this.liveQueryRefreshers.add(refresh);

    const destroy = (): void => {
      this.liveQueryRefreshers.delete(refresh);
    };
    let destroyRef: DestroyRef | null = null;
    try {
      destroyRef = inject(DestroyRef, { optional: true });
    } catch {
      destroyRef = null;
    }
    destroyRef?.onDestroy(destroy);

    return {
      rows: rowsSignal.asReadonly(),
      count: computed(() => rowsSignal().length),
      refresh,
      destroy,
    };
  }
}

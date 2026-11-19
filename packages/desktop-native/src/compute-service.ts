import { Injectable } from '@angular/core';
import { ensureNgNativeBridge, type NativeComputeMatch } from '@ng-native/platform';

export interface NativeScoredItem<T> {
  item: T;
  index: number;
  score: number;
}

/**
 * Injectable Angular service delegating CPU-intensive fuzzy search, scoring, and FNV-1a hashing
 * to the multithreaded Rust `NativeComputePool` without blocking the UI thread.
 */
@Injectable({ providedIn: 'root' })
export class NativeComputeService {
  private readonly bridge = ensureNgNativeBridge();

  fuzzyFilterStrings(items: readonly string[], query: string, limit = 50): NativeComputeMatch[] {
    return this.bridge.computeFuzzyFilter(items, query, limit);
  }

  fuzzyFilter<T>(
    items: readonly T[],
    selector: (item: T) => string,
    query: string,
    limit = 50,
  ): NativeScoredItem<T>[] {
    const keys = items.map((item) => selector(item));
    const matches = this.bridge.computeFuzzyFilter(keys, query, limit);
    return matches.map((m) => ({
      item: items[m.index],
      index: m.index,
      score: m.score,
    }));
  }

  hash(input: string): string {
    return this.bridge.computeHash(input);
  }
}

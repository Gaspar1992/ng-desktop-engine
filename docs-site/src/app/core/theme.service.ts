import { DOCUMENT } from '@angular/common';
import { computed, DestroyRef, inject, Injectable, signal } from '@angular/core';

export type DocsThemePreference = 'light' | 'dark' | 'system';
export type DocsEffectiveTheme = 'light' | 'dark';

export const DOCS_THEME_STORAGE_KEY = 'ng-desktop-docs-theme';

function readStoredOverride(): 'light' | 'dark' | null {
  try {
    const raw = globalThis.localStorage?.getItem(DOCS_THEME_STORAGE_KEY);
    return raw === 'light' || raw === 'dark' ? raw : null;
  } catch {
    return null;
  }
}

function writeStoredOverride(value: 'light' | 'dark' | null): void {
  try {
    if (!globalThis.localStorage) {
      return;
    }
    if (value === null) {
      globalThis.localStorage.removeItem(DOCS_THEME_STORAGE_KEY);
    } else {
      globalThis.localStorage.setItem(DOCS_THEME_STORAGE_KEY, value);
    }
  } catch {
    // Ignore storage quota or sandbox restrictions.
  }
}

function readSystemPrefersDark(): boolean {
  try {
    return Boolean(globalThis.matchMedia?.('(prefers-color-scheme: dark)').matches);
  } catch {
    return false;
  }
}

/**
 * Coordinates the documentation portal's system-first dark/light theme using
 * `<meta name="color-scheme">` paired with CSS `:has()` + `light-dark()`.
 * Only divergent choices are persisted in `localStorage`, and divergence is
 * never re-evaluated retroactively when OS preferences change.
 */
@Injectable({ providedIn: 'root' })
export class DocsThemeService {
  private readonly document = inject(DOCUMENT);
  private readonly destroyRef = inject(DestroyRef);

  private readonly systemDark = signal<boolean>(readSystemPrefersDark());
  private readonly storedOverride = signal<'light' | 'dark' | null>(readStoredOverride());

  readonly preference = computed<DocsThemePreference>(() => this.storedOverride() ?? 'system');

  readonly effectiveTheme = computed<DocsEffectiveTheme>(
    () => this.storedOverride() ?? (this.systemDark() ? 'dark' : 'light'),
  );

  readonly hasExplicitOverride = computed<boolean>(() => this.storedOverride() !== null);

  constructor() {
    this.syncMetaTag(this.storedOverride());

    const mediaQuery = globalThis.matchMedia?.('(prefers-color-scheme: dark)');
    if (mediaQuery) {
      const handleMediaChange = (event: MediaQueryListEvent): void => {
        this.systemDark.set(event.matches);
      };
      mediaQuery.addEventListener?.('change', handleMediaChange);
      this.destroyRef.onDestroy(() => {
        mediaQuery.removeEventListener?.('change', handleMediaChange);
      });
    }

    const handleStorage = (event: StorageEvent): void => {
      if (event.key !== null && event.key !== DOCS_THEME_STORAGE_KEY) {
        return;
      }
      const next = event.newValue === 'light' || event.newValue === 'dark' ? event.newValue : null;
      this.storedOverride.set(next);
      this.syncMetaTag(next);
    };
    globalThis.addEventListener?.('storage', handleStorage);
    this.destroyRef.onDestroy(() => {
      globalThis.removeEventListener?.('storage', handleStorage);
    });
  }

  toggle(): void {
    const nextTheme: DocsEffectiveTheme = this.effectiveTheme() === 'dark' ? 'light' : 'dark';
    const systemTheme: DocsEffectiveTheme = this.systemDark() ? 'dark' : 'light';
    const nextOverride = nextTheme === systemTheme ? null : nextTheme;

    this.storedOverride.set(nextOverride);
    writeStoredOverride(nextOverride);
    this.syncMetaTag(nextOverride);
  }

  private syncMetaTag(override: 'light' | 'dark' | null): void {
    const meta = this.document.querySelector('meta[name="color-scheme"]');
    if (meta) {
      meta.setAttribute('content', override ?? 'light dark');
    }
  }
}

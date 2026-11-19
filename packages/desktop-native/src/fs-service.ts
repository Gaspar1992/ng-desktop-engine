import { Injectable, Signal, signal, WritableSignal } from '@angular/core';
import { ensureNgNativeBridge, type NativeXdgDirKind } from '@ng-native/platform';

export interface NativeWatchedFileRef<T> {
  readonly path: string;
  readonly value: Signal<T>;
  reload(): void;
}

interface WatchedEntry {
  path: string;
  reload: () => void;
}

/**
 * Injectable Angular service for XDG Base Directory resolution (`config`, `data`, `cache`),
 * atomic file writes, and reactive file watching backed by OS kernel notifications (`inotify` / `FSEvents` / `ReadDirectoryChangesW`).
 */
@Injectable({ providedIn: 'root' })
export class NativeFileSystemService {
  private readonly bridge = ensureNgNativeBridge();
  private readonly watchedEntries = new Map<string, WatchedEntry>();

  resolveXdgPath(kind: NativeXdgDirKind, relPath: string): string {
    return this.bridge.fsResolvePath(kind, relPath);
  }

  readText(path: string): string | null {
    return this.bridge.fsReadText(path);
  }

  writeText(path: string, content: string): boolean {
    const ok = this.bridge.fsWriteText(path, content);
    if (ok) {
      this.refreshWatchedFiles();
    }
    return ok;
  }

  readJson<T>(path: string, fallback: T): T {
    const raw = this.readText(path);
    if (raw === null || raw.trim().length === 0) {
      return fallback;
    }
    try {
      return JSON.parse(raw) as T;
    } catch {
      return fallback;
    }
  }

  writeJson(path: string, data: unknown): boolean {
    return this.writeText(path, JSON.stringify(data, null, 2));
  }

  watchFile(path: string, initialContent = ''): NativeWatchedFileRef<string> {
    this.bridge.fsWatchFile(path);
    const sig: WritableSignal<string> = signal(this.readText(path) ?? initialContent);
    const reload = (): void => {
      const latest = this.readText(path);
      if (latest !== null) {
        sig.set(latest);
      }
    };
    this.watchedEntries.set(path, { path, reload });
    return {
      path,
      value: sig.asReadonly(),
      reload,
    };
  }

  watchJson<T>(path: string, fallback: T): NativeWatchedFileRef<T> {
    this.bridge.fsWatchFile(path);
    const sig: WritableSignal<T> = signal(this.readJson(path, fallback));
    const reload = (): void => {
      sig.set(this.readJson(path, fallback));
    };
    this.watchedEntries.set(path, { path, reload });
    return {
      path,
      value: sig.asReadonly(),
      reload,
    };
  }

  refreshWatchedFiles(): string[] {
    const changed = this.bridge.fsPollChangedFiles();
    for (const path of changed) {
      this.watchedEntries.get(path)?.reload();
    }
    return changed;
  }
}

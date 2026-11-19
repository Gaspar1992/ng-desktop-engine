export type NativeXdgDirKind = 'config' | 'data' | 'cache';

export function resolveDefaultAppId(): string {
  const raw = (globalThis as { __NG_NATIVE_APP_ID__?: unknown }).__NG_NATIVE_APP_ID__;
  return typeof raw === 'string' && raw.trim().length > 0 ? raw.trim() : 'ng-desktop-app';
}

export interface NativeComputeMatch {
  index: number;
  score: number;
  value: string;
}

function scoreCandidate(candidate: string, needleLower: string): number | null {
  if (needleLower.length === 0) {
    return 1;
  }
  const candLower = candidate.toLowerCase();
  if (candLower === needleLower) {
    return 1000;
  }
  if (candLower.startsWith(needleLower)) {
    return 600 + Math.max(0, 80 - candidate.length);
  }
  const pos = candLower.indexOf(needleLower);
  if (pos !== -1) {
    return 350 + Math.max(0, 60 - pos * 2);
  }

  let needleIdx = 0;
  let consecutiveBonus = 0;
  let prevMatched = false;
  for (let i = 0; i < candLower.length && needleIdx < needleLower.length; i++) {
    if (candLower[i] === needleLower[needleIdx]) {
      needleIdx += 1;
      if (prevMatched) {
        consecutiveBonus += 15;
      }
      prevMatched = true;
    } else {
      prevMatched = false;
    }
  }
  return needleIdx === needleLower.length ? 100 + consecutiveBonus : null;
}

export function computeFuzzyFilterFallback(
  items: readonly string[],
  query: string,
  limit = 50,
): NativeComputeMatch[] {
  if (items.length === 0 || limit <= 0) {
    return [];
  }
  const needleLower = query.trim().toLowerCase();
  const matches: NativeComputeMatch[] = [];
  for (let index = 0; index < items.length; index++) {
    const value = items[index];
    const score = scoreCandidate(value, needleLower);
    if (score !== null) {
      matches.push({ index, score, value });
    }
  }
  matches.sort((a, b) => (b.score !== a.score ? b.score - a.score : a.index - b.index));
  return matches.slice(0, limit);
}

export function computeHashFallback(input: string): string {
  let hi = 0x84222325;
  let lo = 0xcbf29ce4;
  for (let i = 0; i < input.length; i++) {
    const code = input.charCodeAt(i);
    lo = (lo ^ code) >>> 0;
    hi = (Math.imul(hi, 0x01000193) ^ (lo >>> 16)) >>> 0;
    lo = Math.imul(lo, 0x01000193) >>> 0;
  }
  return `${hi.toString(16).padStart(8, '0')}${lo.toString(16).padStart(8, '0')}`;
}

export class InMemoryFsStore {
  private readonly files = new Map<string, string>();
  private readonly watched = new Set<string>();
  private readonly changed = new Set<string>();

  resolvePath(kind: NativeXdgDirKind, relPath: string, appId = resolveDefaultAppId()): string {
    const clean = relPath.trim().replace(/^[/\\]+/, '');
    if (clean.length === 0 || clean.split(/[/\\]/).includes('..')) {
      throw new Error(`Invalid or unsafe XDG relative path: '${relPath}'`);
    }
    const base =
      kind === 'data'
        ? `/home/user/.local/share/${appId}`
        : kind === 'cache'
          ? `/home/user/.cache/${appId}`
          : `/home/user/.config/${appId}`;
    return `${base}/${clean}`;
  }

  readText(path: string): string | null {
    return this.files.get(path.trim()) ?? null;
  }

  writeText(path: string, content: string): boolean {
    const clean = path.trim();
    if (clean.length === 0) {
      return false;
    }
    this.files.set(clean, content);
    if (this.watched.has(clean)) {
      this.changed.add(clean);
    }
    return true;
  }

  watchFile(path: string): boolean {
    const clean = path.trim();
    if (clean.length === 0) {
      return false;
    }
    this.watched.add(clean);
    return true;
  }

  pollChangedFiles(): string[] {
    const out = [...this.changed];
    this.changed.clear();
    return out;
  }
}

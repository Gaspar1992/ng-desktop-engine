export interface NativeDomRect {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
  readonly top: number;
  readonly left: number;
  readonly right: number;
  readonly bottom: number;
}

export interface NativeElementEvent {
  readonly type: string;
  readonly target?: NativeElementNode;
  readonly detail?: unknown;
}

export type NativeEventListener = (event: NativeElementEvent) => void;

let activeNativeElement: NativeElementNode | null = null;

/** Returns the currently focused `NativeElementNode`, mirroring `document.activeElement`. */
export function getNativeActiveElement(): NativeElementNode | null {
  return activeNativeElement;
}

/** Programmatically updates the active `NativeElementNode` for headless ARIA focus tracking. */
export function setNativeActiveElement(node: NativeElementNode | null): void {
  activeNativeElement = node;
}

/**
 * Lightweight scene-graph proxy node used by `NativeRenderer2` and headless ARIA/CDK utilities.
 * Supports synchronous tree traversal, WAI-ARIA attribute reflection, focus state, `classList`,
 * and DOM-compatible event dispatch without crossing FFI.
 */
export class NativeElementNode {
  readonly id: string;
  readonly tag: string;
  readonly isComment: boolean;
  parent: NativeElementNode | null = null;
  readonly children: NativeElementNode[] = [];
  readonly attributes = new Map<string, string>();
  readonly classes = new Set<string>();
  readonly styles = new Map<string, string>();
  private readonly listeners = new Map<string, Set<NativeEventListener>>();
  textContent = '';

  readonly classList: {
    add: (...tokens: string[]) => void;
    remove: (...tokens: string[]) => void;
    toggle: (token: string, force?: boolean) => boolean;
    contains: (token: string) => boolean;
  };

  constructor(id: string, tag: string, isComment = false) {
    this.id = id;
    this.tag = tag;
    this.isComment = isComment;
    this.classList = {
      add: (...tokens: string[]) => {
        for (const token of tokens) {
          if (token.trim()) {
            this.classes.add(token.trim());
          }
        }
      },
      remove: (...tokens: string[]) => {
        for (const token of tokens) {
          this.classes.delete(token);
        }
      },
      toggle: (token: string, force?: boolean) => {
        const shouldAdd = force ?? !this.classes.has(token);
        if (shouldAdd) {
          this.classes.add(token);
        } else {
          this.classes.delete(token);
        }
        return shouldAdd;
      },
      contains: (token: string) => this.classes.has(token),
    };
  }

  get tagName(): string {
    return this.tag.toUpperCase();
  }

  get nodeName(): string {
    return this.isComment ? '#comment' : this.tag.toUpperCase();
  }

  get parentNode(): NativeElementNode | null {
    return this.parent;
  }

  get firstChild(): NativeElementNode | null {
    return this.children[0] ?? null;
  }

  get lastChild(): NativeElementNode | null {
    return this.children.length > 0 ? (this.children[this.children.length - 1] ?? null) : null;
  }

  get previousSibling(): NativeElementNode | null {
    if (!this.parent) {
      return null;
    }
    const idx = this.parent.children.indexOf(this);
    if (idx <= 0) {
      return null;
    }
    return this.parent.children[idx - 1] ?? null;
  }

  get nextSibling(): NativeElementNode | null {
    if (!this.parent) {
      return null;
    }
    const idx = this.parent.children.indexOf(this);
    if (idx === -1 || idx + 1 >= this.parent.children.length) {
      return null;
    }
    return this.parent.children[idx + 1] ?? null;
  }

  get childNodes(): readonly NativeElementNode[] {
    return this.children;
  }

  get tabIndex(): number {
    const raw = this.attributes.get('tabindex');
    if (raw === undefined) {
      return -1;
    }
    const parsed = Number.parseInt(raw, 10);
    return Number.isNaN(parsed) ? -1 : parsed;
  }

  set tabIndex(value: number) {
    this.attributes.set('tabindex', String(value));
  }

  get role(): string | null {
    return this.attributes.get('role') ?? null;
  }

  set role(value: string | null) {
    if (value === null) {
      this.attributes.delete('role');
    } else {
      this.attributes.set('role', value);
    }
  }

  getAttribute(name: string): string | null {
    return this.attributes.get(name) ?? null;
  }

  setAttribute(name: string, value: string): void {
    this.attributes.set(name, String(value));
  }

  removeAttribute(name: string): void {
    this.attributes.delete(name);
  }

  hasAttribute(name: string): boolean {
    return this.attributes.has(name);
  }

  focus(): void {
    const previous = getNativeActiveElement();
    if (previous && previous !== this) {
      previous.blur();
    }
    setNativeActiveElement(this);
    this.dispatchEvent({ type: 'focus', target: this });
  }

  blur(): void {
    if (getNativeActiveElement() === this) {
      setNativeActiveElement(null);
    }
    this.dispatchEvent({ type: 'blur', target: this });
  }

  contains(other: NativeElementNode | null): boolean {
    let cursor: NativeElementNode | null = other;
    while (cursor) {
      if (cursor === this) {
        return true;
      }
      cursor = cursor.parent;
    }
    return false;
  }

  addEventListener(type: string, listener: NativeEventListener): void {
    let bucket = this.listeners.get(type);
    if (!bucket) {
      bucket = new Set<NativeEventListener>();
      this.listeners.set(type, bucket);
    }
    bucket.add(listener);
  }

  removeEventListener(type: string, listener: NativeEventListener): void {
    this.listeners.get(type)?.delete(listener);
  }

  dispatchEvent(event: NativeElementEvent | string): boolean {
    const normalized: NativeElementEvent =
      typeof event === 'string'
        ? { type: event, target: this }
        : { type: event.type, target: event.target ?? this, detail: event.detail };
    const bucket = this.listeners.get(normalized.type);
    if (bucket) {
      for (const listener of bucket) {
        listener(normalized);
      }
    }
    return true;
  }

  getBoundingClientRect(): NativeDomRect {
    const rawW = Number(this.attributes.get('width') ?? 0);
    const rawH = Number(this.attributes.get('height') ?? 0);
    const width = Number.isFinite(rawW) ? rawW : 0;
    const height = Number.isFinite(rawH) ? rawH : 0;
    return {
      x: 0,
      y: 0,
      width,
      height,
      top: 0,
      left: 0,
      right: width,
      bottom: height,
    };
  }
}

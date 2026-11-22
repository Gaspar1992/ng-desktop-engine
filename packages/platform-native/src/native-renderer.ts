import {
  Injectable,
  Renderer2,
  RendererFactory2,
  RendererStyleFlags2,
  RendererType2,
} from '@angular/core';
import { ensureNgNativeBridge, NgNativeBridge } from './bridge';
import { mapAlignKeyword, registerParsedStyles } from './css-style-parser';
import { NativeElementNode } from './native-element';

const VIRTUAL_ANGULAR_TAGS = new Set(['ng-container', 'ng-template', 'ng-content', 'ng-component']);

const TEXT_BEARING_TAGS = new Set([
  'ng-label',
  'gtk-label',
  'ng-button',
  'gtk-button',
  'ng-window',
  'adw-window',
  'ng-dialog',
  'gtk-dialog',
  'adw-dialog',
  'ng-header-bar',
  'adw-header-bar',
  'gtk-header-bar',
  'ng-check-button',
  'gtk-check-button',
  'ng-expander',
  'gtk-expander',
  'ng-entry',
  'gtk-entry',
  'ng-search-entry',
  'gtk-search-entry',
  'ng-password-entry',
  'gtk-password-entry',
  'ng-text-view',
  'gtk-text-view',
]);

const TEXT_INPUT_TAGS = new Set([
  'ng-entry',
  'gtk-entry',
  'ng-search-entry',
  'gtk-search-entry',
  'ng-password-entry',
  'gtk-password-entry',
  'ng-text-view',
  'gtk-text-view',
]);

/**
 * 1:1 mapping from standard HTML elements to native desktop widget primitives.
 * Containers map to `ng-box` (1D Flexbox); text elements map to `ng-label`.
 * Angular component selectors (e.g. `app-*`, `router-outlet`, `ng-container`) are omitted
 * so they continue operating as virtual hosts.
 */
export const HTML_TO_NATIVE_TAG: Readonly<Record<string, string>> = {
  div: 'ng-box',
  section: 'ng-box',
  main: 'ng-box',
  article: 'ng-box',
  aside: 'ng-box',
  nav: 'ng-box',
  header: 'ng-box',
  footer: 'ng-box',
  form: 'ng-box',
  ul: 'ng-box',
  ol: 'ng-box',
  li: 'ng-box',
  figure: 'ng-box',
  figcaption: 'ng-box',
  p: 'ng-label',
  span: 'ng-label',
  label: 'ng-label',
  h1: 'ng-label',
  h2: 'ng-label',
  h3: 'ng-label',
  h4: 'ng-label',
  h5: 'ng-label',
  h6: 'ng-label',
  strong: 'ng-label',
  em: 'ng-label',
  small: 'ng-label',
  code: 'ng-label',
  pre: 'ng-label',
  a: 'ng-label',
  button: 'ng-button',
  img: 'ng-image',
  hr: 'ng-separator',
  input: 'ng-entry',
  textarea: 'ng-text-view',
  select: 'ng-select',
  progress: 'ng-progress-bar',
  details: 'ng-expander',
  dialog: 'ng-dialog',
};

/**
 * Resolves an Angular template element tag into a native desktop widget tag (`ng-*`, `gtk-*`,
 * `adw-*`), returning `null` for virtual Angular hosts (`ng-container`, `router-outlet`, `app-*`).
 */
export function resolveNativeTag(tag: string): string | null {
  const lower = tag.trim().toLowerCase();
  if (VIRTUAL_ANGULAR_TAGS.has(lower)) {
    return null;
  }
  if (lower.startsWith('ng-') || lower.startsWith('adw-') || lower.startsWith('gtk-')) {
    return lower;
  }
  return HTML_TO_NATIVE_TAG[lower] ?? null;
}

function isImageTag(tag: string): boolean {
  return tag === 'ng-image' || tag === 'adw-image' || tag === 'ng-picture' || tag === 'gtk-picture';
}

function parseHtmlBooleanFlag(value: unknown): boolean {
  if (value === undefined || value === null || value === false || value === 0) {
    return false;
  }
  if (typeof value === 'string') {
    const lower = value.trim().toLowerCase();
    return lower !== 'false' && lower !== '0' && lower !== 'null' && lower !== 'undefined';
  }
  return Boolean(value);
}

function isInternalAngularAttr(name: string): boolean {
  return name.startsWith('_ngcontent-') || name.startsWith('_nghost-') || name === 'ng-version';
}

function mapNativeEventNames(eventName: string): string[] {
  switch (eventName) {
    case 'click':
      return ['clicked'];
    case 'dblclick':
    case 'doubleclick':
      return ['dblclick'];
    case 'contextmenu':
    case 'rightclick':
      return ['contextmenu'];
    case 'mousedown':
    case 'pointerdown':
      return ['mousedown'];
    case 'mouseup':
    case 'pointerup':
      return ['mouseup'];
    case 'mouseenter':
    case 'mouseover':
    case 'pointerenter':
      return ['mouseenter'];
    case 'mouseleave':
    case 'mouseout':
    case 'pointerleave':
      return ['mouseleave'];
    case 'mousemove':
    case 'pointermove':
      return ['mousemove'];
    case 'focus':
    case 'focusin':
    case 'windowFocus':
      return ['focus'];
    case 'blur':
    case 'focusout':
    case 'windowBlur':
      return ['blur'];
    case 'keydown':
      return ['keydown'];
    case 'keyup':
      return ['keyup'];
    case 'input':
      return ['input'];
    case 'change':
    case 'valueChange':
      return ['changed', 'value-changed', 'toggled'];
    case 'submit':
    case 'search':
    case 'activate':
      return ['activate'];
    case 'cancel':
    case 'escape':
    case 'stopSearch':
      return ['cancel'];
    case 'scroll':
      return ['scroll'];
    case 'scrollEdge':
    case 'scroll-edge':
      return ['scroll-edge', 'edge-reached'];
    case 'scrollBottom':
    case 'scroll-bottom':
      return ['scroll-bottom'];
    case 'scrollTop':
    case 'scroll-top':
      return ['scroll-top'];
    case 'wheel':
    case 'mousewheel':
      return ['wheel'];
    case 'resize':
    case 'windowResize':
    case 'window-resize':
      return ['resize'];
    case 'maximize':
    case 'windowMaximize':
      return ['maximize'];
    case 'closeRequest':
    case 'close-request':
    case 'beforeunload':
      return ['close-request'];
    case 'dragenter':
      return ['dragenter'];
    case 'dragleave':
      return ['dragleave'];
    case 'drop':
      return ['drop', 'files-dropped'];
    default:
      return [eventName];
  }
}

function lazyControllerTokenFor(eventName: string): string | null {
  switch (eventName) {
    case 'click':
    case 'dblclick':
    case 'doubleclick':
    case 'mousedown':
    case 'mouseup':
    case 'pointerdown':
    case 'pointerup':
      return 'click';
    case 'contextmenu':
    case 'rightclick':
      return 'contextmenu';
    case 'mouseenter':
    case 'mouseleave':
    case 'mousemove':
    case 'mouseover':
    case 'mouseout':
    case 'pointerenter':
    case 'pointerleave':
    case 'pointermove':
      return 'motion';
    case 'focus':
    case 'blur':
    case 'focusin':
    case 'focusout':
      return 'focus';
    case 'keydown':
    case 'keyup':
      return 'keyboard';
    case 'wheel':
    case 'mousewheel':
      return 'wheel';
    case 'closeRequest':
    case 'close-request':
    case 'beforeunload':
      return 'close-request';
    default:
      return null;
  }
}

function syncTargetFromPayload(
  target: NativeElementNode | undefined,
  parsed: Record<string, unknown>,
): void {
  if (!target) {
    return;
  }
  if (parsed['value'] !== undefined) {
    target.value = String(parsed['value']);
  }
  if (typeof parsed['checked'] === 'boolean') {
    target.checked = parsed['checked'];
  } else if (typeof parsed['active'] === 'boolean') {
    target.checked = parsed['active'];
  }
}

function normalizeNativeEventPayload(
  eventName: string,
  rawPayload: unknown,
  target?: NativeElementNode,
): unknown {
  const tryParseJson = (raw: unknown): Record<string, unknown> | null => {
    if (typeof raw !== 'string') return null;
    const trimmed = raw.trim();
    if (!trimmed.startsWith('{') || !trimmed.endsWith('}')) return null;
    try {
      const parsed = JSON.parse(trimmed) as unknown;
      return parsed && typeof parsed === 'object' && !Array.isArray(parsed)
        ? (parsed as Record<string, unknown>)
        : null;
    } catch {
      return null;
    }
  };

  let normalized: Record<string, unknown>;
  if (typeof rawPayload === 'string') {
    const parsed = tryParseJson(rawPayload);
    normalized = parsed ? { type: eventName, ...parsed } : { type: eventName, value: rawPayload };
  } else if (rawPayload && typeof rawPayload === 'object' && !Array.isArray(rawPayload)) {
    const obj = rawPayload as Record<string, unknown>;
    const parsed = tryParseJson(obj['value']);
    normalized = parsed ? { type: eventName, ...obj, ...parsed } : { type: eventName, ...obj };
  } else {
    normalized = { type: eventName };
  }

  syncTargetFromPayload(target, normalized);
  if (target && normalized['target'] === undefined) {
    normalized['target'] = target;
  }
  return normalized;
}

/**
 * Custom Angular `Renderer2` implementation that translates Ivy template instructions
 * (`createElement`, `createComment`, `insertBefore`, `removeChild`, `setAttribute`, `listen`)
 * into `globalThis.__NG_NATIVE__` scene graph commands.
 */
interface NativeRendererHooks {
  onRealElementCreated?: (tag: string, id: string) => void;
  getPrimaryWindowId?: () => string | null;
}

interface VirtualListenerRegistration {
  nativeEvents: string[];
  eventName: string;
  callback: (event: unknown) => boolean | void;
  attachedRealIds: Set<string>;
  activeUnsubs: Set<() => void>;
  disposed: boolean;
}

export class NativeRenderer2 extends Renderer2 {
  readonly data: { [key: string]: unknown } = Object.create(null);
  override destroyNode: ((node: unknown) => void) | null;

  private readonly bridge: NgNativeBridge;
  private readonly nodesById: Map<string, NativeElementNode>;
  private readonly nextId: () => string;
  private readonly hooks?: NativeRendererHooks;
  private readonly materializedTextLabels = new Map<NativeElementNode, string>();
  private readonly imageLoadSeq = new WeakMap<NativeElementNode, number>();
  private readonly enabledEventTokens = new Map<string, Set<string>>();
  private readonly pendingVirtualListeners = new WeakMap<
    NativeElementNode,
    Set<VirtualListenerRegistration>
  >();

  constructor(
    bridge: NgNativeBridge,
    nodesById: Map<string, NativeElementNode>,
    nextId: () => string,
    hooks?: NativeRendererHooks,
  ) {
    super();
    this.bridge = bridge;
    this.nodesById = nodesById;
    this.nextId = nextId;
    this.hooks = hooks;
    this.destroyNode = (node: unknown) => {
      if (node instanceof NativeElementNode) {
        this.destroySubtreeNodes(node, true);
      }
    };
  }

  private destroySubtreeNodes(node: NativeElementNode, emitBridgeDestroy: boolean): void {
    for (const child of node.children) {
      this.destroySubtreeNodes(child, emitBridgeDestroy && node.isVirtual);
    }
    this.nodesById.delete(node.id);
    this.imageLoadSeq.delete(node);
    this.enabledEventTokens.delete(node.id);
    const pending = this.pendingVirtualListeners.get(node);
    if (pending) {
      for (const reg of pending) {
        reg.disposed = true;
        for (const unlisten of reg.activeUnsubs) {
          unlisten();
        }
        reg.activeUnsubs.clear();
        reg.attachedRealIds.clear();
      }
      this.pendingVirtualListeners.delete(node);
    }
    const materializedId = this.materializedTextLabels.get(node);
    if (materializedId) {
      this.materializedTextLabels.delete(node);
      if (emitBridgeDestroy) {
        this.bridge.destroyNode(materializedId);
      }
    }
    if (!node.isVirtual && emitBridgeDestroy) {
      this.bridge.destroyNode(node.id);
    }
  }

  override destroy(): void {
    // Renderer lifecycle cleanup
  }

  override createElement(name: string, _namespace?: string | null): NativeElementNode {
    const id = this.nextId();
    const trimmed = name.trim();
    const resolvedTag = resolveNativeTag(trimmed);
    const isNative = resolvedTag !== null;
    const node = new NativeElementNode(id, resolvedTag ?? trimmed, false, !isNative, false);
    this.nodesById.set(id, node);
    if (resolvedTag !== null) {
      this.hooks?.onRealElementCreated?.(resolvedTag, id);
      this.bridge.createElement(id, resolvedTag);
      if (resolvedTag === 'ng-window' || resolvedTag === 'adw-window') {
        this.attachGlobalWindowIcon(id);
      }
    }
    return node;
  }

  override createComment(value: string): NativeElementNode {
    const id = this.nextId();
    const cleanComment = value.trim() || 'anchor';
    const node = new NativeElementNode(id, '#comment', true, false, false);
    node.textContent = cleanComment;
    this.nodesById.set(id, node);
    this.bridge.createComment(id, cleanComment);
    return node;
  }

  override createText(value: string): NativeElementNode {
    const id = this.nextId();
    const node = new NativeElementNode(id, '#text', false, true, true);
    node.textContent = value;
    this.nodesById.set(id, node);
    return node;
  }

  override appendChild(parent: NativeElementNode, newChild: NativeElementNode): void {
    if (!parent) {
      return;
    }
    if (newChild.parent) {
      this.removeChild(newChild.parent, newChild);
    }
    newChild.parent = parent;
    parent.children.push(newChild);
    this.projectInsertion(parent, newChild, null, true);
  }

  override insertBefore(
    parent: NativeElementNode,
    newChild: NativeElementNode,
    refChild: NativeElementNode | null,
    _isMove?: boolean,
  ): void {
    if (!parent) {
      return;
    }
    const prevParent = newChild.parent;
    if (prevParent) {
      const existingIdx = prevParent.children.indexOf(newChild);
      if (existingIdx !== -1) {
        prevParent.children.splice(existingIdx, 1);
      }
      const prevReal = this.findRealAncestor(prevParent);
      const nextReal = this.findRealAncestor(parent);
      if (prevParent !== parent && prevReal && prevReal !== nextReal && !newChild.isText) {
        for (const realId of this.collectRealRootIds(newChild)) {
          this.bridge.removeChild(prevReal.id, realId);
        }
      }
    }
    newChild.parent = parent;

    if (refChild) {
      const refIdx = parent.children.indexOf(refChild);
      if (refIdx !== -1) {
        parent.children.splice(refIdx, 0, newChild);
        this.projectInsertion(parent, newChild, refChild, false);
        return;
      }
    }

    parent.children.push(newChild);
    this.projectInsertion(parent, newChild, null, false);
  }

  override removeChild(
    parent: NativeElementNode | null,
    oldChild: NativeElementNode,
    _isHostElement?: boolean,
  ): void {
    const targetParent = parent ?? oldChild.parent;
    if (!targetParent) {
      return;
    }
    const realParent = this.findRealAncestor(targetParent);
    const materializedId = this.materializedTextLabels.get(oldChild);
    const realIds = oldChild.isText ? [] : this.collectRealRootIds(oldChild);

    const idx = targetParent.children.indexOf(oldChild);
    if (idx !== -1) {
      targetParent.children.splice(idx, 1);
    }
    oldChild.parent = null;

    if (!realParent) {
      return;
    }
    if (oldChild.isText) {
      if (materializedId) {
        this.bridge.removeChild(realParent.id, materializedId);
      } else if (TEXT_BEARING_TAGS.has(realParent.tag)) {
        this.syncTextBearingParent(realParent);
      }
      return;
    }
    for (const realId of realIds) {
      this.bridge.removeChild(realParent.id, realId);
    }
  }

  override selectRootElement(
    selectorOrNode: string | NativeElementNode,
    _preserveContent?: boolean,
  ): NativeElementNode {
    if (selectorOrNode instanceof NativeElementNode) {
      return selectorOrNode;
    }
    const trimmed = selectorOrNode.trim();
    const normalizedId = trimmed.replace(/^[#.]/, '') || 'ng_root';
    const existing = this.nodesById.get(normalizedId);
    if (existing) {
      return existing;
    }
    const isRealWindow =
      trimmed.startsWith('#') || trimmed === 'ng-window' || trimmed === 'adw-window';
    if (isRealWindow) {
      const root = new NativeElementNode(normalizedId, 'ng-window', false, false, false);
      this.nodesById.set(normalizedId, root);
      this.hooks?.onRealElementCreated?.('ng-window', normalizedId);
      this.bridge.createElement(normalizedId, 'ng-window');
      this.attachGlobalWindowIcon(normalizedId);
      return root;
    }
    const virtualRoot = new NativeElementNode(normalizedId, trimmed, false, true, false);
    this.nodesById.set(normalizedId, virtualRoot);
    return virtualRoot;
  }

  override parentNode(node: NativeElementNode): NativeElementNode | null {
    return node.parent;
  }

  override nextSibling(node: NativeElementNode): NativeElementNode | null {
    return node.nextSibling;
  }

  override setAttribute(
    el: NativeElementNode,
    name: string,
    value: string,
    _namespace?: string | null,
  ): void {
    if (isInternalAngularAttr(name)) {
      return;
    }
    el.attributes.set(name, value);
    if (name === 'class' || name === 'className') {
      this.syncClassList(el, value);
      return;
    }
    if (el.isVirtual) {
      return;
    }
    if (name === 'disabled') {
      const nextSensitive = parseHtmlBooleanFlag(value) ? 'false' : 'true';
      el.attributes.set('sensitive', nextSensitive);
      this.bridge.setAttribute(el.id, 'sensitive', nextSensitive);
      return;
    }
    if (isImageTag(el.tag) && (name === 'src' || name === 'ngSrc')) {
      this.handleImageSource(el, value);
      return;
    }
    if (
      name === 'title' &&
      (el.tag === 'ng-window' ||
        el.tag === 'adw-window' ||
        el.tag === 'ng-dialog' ||
        el.tag === 'gtk-dialog' ||
        el.tag === 'adw-dialog' ||
        el.tag === 'ng-header-bar' ||
        el.tag === 'adw-header-bar')
    ) {
      this.bridge.setText(el.id, value);
    } else {
      this.bridge.setAttribute(el.id, name, value);
    }
  }

  override removeAttribute(el: NativeElementNode, name: string, _namespace?: string | null): void {
    el.attributes.delete(name);
    if (isInternalAngularAttr(name) || el.isVirtual) {
      return;
    }
    if (name === 'class' || name === 'className') {
      const classesToRemove: string[] = [];
      for (const cls of el.classes) {
        classesToRemove.push(cls);
      }
      for (const cls of classesToRemove) {
        this.removeClass(el, cls);
      }
      return;
    }
    if (name === 'disabled') {
      el.attributes.set('sensitive', 'true');
      this.bridge.setAttribute(el.id, 'sensitive', 'true');
      return;
    }
    this.bridge.setAttribute(el.id, name, '');
  }

  override addClass(el: NativeElementNode, name: string): void {
    el.classes.add(name);
    if (!el.isVirtual) {
      this.bridge.addClass(el.id, name);
    }
  }

  override removeClass(el: NativeElementNode, name: string): void {
    el.classes.delete(name);
    if (!el.isVirtual) {
      this.bridge.removeClass(el.id, name);
    }
  }

  override setStyle(
    el: NativeElementNode,
    style: string,
    value: unknown,
    _flags?: RendererStyleFlags2,
  ): void {
    if (value === null || value === undefined || value === false) {
      this.removeStyle(el, style, _flags);
      return;
    }
    const strVal = String(value);
    el.styles.set(style, strVal);
    if (el.isVirtual) {
      return;
    }
    const key = style.trim().toLowerCase();
    const trimmed = strVal.trim();
    if (key === 'flex' || key === 'flex-grow') {
      const grow = Number.parseFloat(trimmed);
      if (Number.isFinite(grow) && grow >= 1) {
        this.bridge.setAttribute(el.id, 'hexpand', 'true');
      }
      return;
    }
    if (key === 'flex-direction') {
      this.bridge.setAttribute(
        el.id,
        'orientation',
        trimmed.toLowerCase().startsWith('row') ? 'horizontal' : 'vertical',
      );
      return;
    }
    if (key === 'gap' || key === 'row-gap' || key === 'column-gap') {
      this.bridge.setAttribute(
        el.id,
        key === 'gap' ? 'spacing' : key.replace('-gap', '-spacing'),
        trimmed,
      );
      return;
    }
    if (key === 'width' && trimmed === '100%') {
      this.bridge.setAttribute(el.id, 'hexpand', 'true');
      return;
    }
    if (key === 'height' && trimmed === '100%') {
      this.bridge.setAttribute(el.id, 'vexpand', 'true');
      return;
    }
    if (key === 'justify-self') {
      const aligned = mapAlignKeyword(trimmed);
      if (aligned) this.bridge.setAttribute(el.id, 'halign', aligned);
      return;
    }
    if (key === 'align-self') {
      const aligned = mapAlignKeyword(trimmed);
      if (aligned) this.bridge.setAttribute(el.id, 'valign', aligned);
      return;
    }
    if (key === 'margin-left') {
      this.bridge.setAttribute(el.id, 'margin-start', trimmed);
      return;
    }
    if (key === 'margin-right') {
      this.bridge.setAttribute(el.id, 'margin-end', trimmed);
      return;
    }
    this.bridge.setAttribute(el.id, style, strVal);
  }

  override removeStyle(el: NativeElementNode, style: string, _flags?: RendererStyleFlags2): void {
    el.styles.delete(style);
    if (!el.isVirtual) {
      this.bridge.setAttribute(el.id, style, '');
    }
  }

  override setProperty(el: NativeElementNode, name: string, value: unknown): void {
    if (isInternalAngularAttr(name)) {
      return;
    }
    const strVal = String(value ?? '');
    if (name === 'className') {
      this.syncClassList(el, strVal);
      return;
    }
    if (el.isVirtual) {
      return;
    }
    if (name === 'disabled') {
      const nextSensitive = parseHtmlBooleanFlag(value) ? 'false' : 'true';
      el.attributes.set('sensitive', nextSensitive);
      this.bridge.setAttribute(el.id, 'sensitive', nextSensitive);
      return;
    }
    if (name === 'hidden') {
      const nextVisible = parseHtmlBooleanFlag(value) ? 'false' : 'true';
      el.attributes.set('visible', nextVisible);
      this.bridge.setAttribute(el.id, 'visible', nextVisible);
      return;
    }
    if (name === 'checked') {
      const checked = parseHtmlBooleanFlag(value);
      el.checked = checked;
      this.bridge.setAttribute(el.id, 'active', checked ? 'true' : 'false');
      return;
    }
    if (name === 'value' && TEXT_INPUT_TAGS.has(el.tag)) {
      el.value = strVal;
      this.bridge.setAttribute(el.id, 'value', strVal);
      return;
    }
    if (isImageTag(el.tag) && (name === 'src' || name === 'ngSrc')) {
      el.attributes.set(name, strVal);
      this.handleImageSource(el, strVal);
      return;
    }
    if (
      name === 'textContent' ||
      name === 'innerText' ||
      name === 'label' ||
      (name === 'text' && TEXT_BEARING_TAGS.has(el.tag)) ||
      (name === 'title' &&
        (el.tag === 'ng-window' ||
          el.tag === 'adw-window' ||
          el.tag === 'ng-dialog' ||
          el.tag === 'gtk-dialog' ||
          el.tag === 'adw-dialog' ||
          el.tag === 'ng-header-bar' ||
          el.tag === 'adw-header-bar'))
    ) {
      el.attributes.set(name, strVal);
      el.textContent = strVal;
      this.bridge.setText(el.id, strVal);
    } else {
      el.attributes.set(name, strVal);
      this.bridge.setAttribute(el.id, name, strVal);
    }
  }

  override setValue(node: NativeElementNode, value: string): void {
    node.textContent = value;
    if (node.isText) {
      const realParent = this.findRealAncestor(node.parent);
      if (!realParent) {
        return;
      }
      if (TEXT_BEARING_TAGS.has(realParent.tag)) {
        this.syncTextBearingParent(realParent);
      } else {
        const existingMat = this.materializedTextLabels.get(node);
        if (existingMat) {
          this.bridge.setText(existingMat, value);
        } else if (value.trim().length > 0 && node.parent) {
          this.materializeTextIntoContainer(node.parent, realParent, node, null, false);
        }
      }
      return;
    }
    this.bridge.setText(node.id, value);
  }

  private ensureLazyEventController(nodeId: string, tag: string, eventName: string): void {
    const token = lazyControllerTokenFor(eventName);
    if (!token) {
      return;
    }
    const lowerTag = tag.trim().toLowerCase();
    if ((lowerTag === 'ng-button' || lowerTag === 'gtk-button') && eventName === 'click') {
      return;
    }
    let active = this.enabledEventTokens.get(nodeId);
    if (!active) {
      active = new Set();
      this.enabledEventTokens.set(nodeId, active);
    }
    if (active.has(token)) {
      return;
    }
    active.add(token);
    this.bridge.setAttribute(nodeId, 'event-mask', [...active].join(','));
  }

  override listen(
    target: 'window' | 'document' | 'body' | NativeElementNode,
    eventName: string,
    callback: (event: unknown) => boolean | void,
  ): () => void {
    const nativeEvents = mapNativeEventNames(eventName);
    if (typeof target === 'string') {
      const nodeId =
        target === 'window' || target === 'document' || target === 'body'
          ? (this.hooks?.getPrimaryWindowId?.() ?? 'win')
          : target;
      const isWindowGlobal = target === 'window' || target === 'document' || target === 'body';
      if (
        !isWindowGlobal ||
        (eventName !== 'focus' &&
          eventName !== 'blur' &&
          eventName !== 'windowFocus' &&
          eventName !== 'windowBlur')
      ) {
        this.ensureLazyEventController(nodeId, 'ng-window', eventName);
      }
      const unsubs = nativeEvents.map((nativeEvent) =>
        this.bridge.listen(nodeId, nativeEvent, (evt) => {
          callback(normalizeNativeEventPayload(eventName, evt));
        }),
      );
      return () => {
        for (const unlisten of unsubs) {
          unlisten();
        }
      };
    }

    if (target.isVirtual) {
      const reg: VirtualListenerRegistration = {
        nativeEvents,
        eventName,
        callback,
        attachedRealIds: new Set(),
        activeUnsubs: new Set(),
        disposed: false,
      };
      let pendingSet = this.pendingVirtualListeners.get(target);
      if (!pendingSet) {
        pendingSet = new Set();
        this.pendingVirtualListeners.set(target, pendingSet);
      }
      pendingSet.add(reg);

      const currentRootIds = this.collectRealRootIds(target);
      if (currentRootIds.length > 0) {
        this.attachRegistrationToRealIds(reg, currentRootIds);
      }

      return () => {
        reg.disposed = true;
        pendingSet.delete(reg);
        for (const unlisten of reg.activeUnsubs) {
          unlisten();
        }
        reg.activeUnsubs.clear();
        reg.attachedRealIds.clear();
      };
    }

    this.ensureLazyEventController(target.id, target.tag, eventName);
    const unsubs = nativeEvents.map((nativeEvent) =>
      this.bridge.listen(target.id, nativeEvent, (evt) => {
        callback(normalizeNativeEventPayload(eventName, evt, target));
      }),
    );
    return () => {
      for (const unlisten of unsubs) {
        unlisten();
      }
    };
  }

  private attachRegistrationToRealIds(reg: VirtualListenerRegistration, realIds: string[]): void {
    if (reg.disposed) {
      return;
    }
    for (const realId of realIds) {
      if (reg.attachedRealIds.has(realId)) {
        continue;
      }
      reg.attachedRealIds.add(realId);
      const realNode = this.nodesById.get(realId);
      const realTag = realNode?.tag ?? '';
      this.ensureLazyEventController(realId, realTag, reg.eventName);
      for (const nativeEvent of reg.nativeEvents) {
        const unlisten = this.bridge.listen(realId, nativeEvent, (evt) => {
          reg.callback(normalizeNativeEventPayload(reg.eventName, evt, realNode));
        });
        reg.activeUnsubs.add(unlisten);
      }
    }
  }

  private attachVirtualListenersForAncestors(
    startParent: NativeElementNode | null,
    realIds: string[],
  ): void {
    let ancestor = startParent;
    while (ancestor) {
      if (ancestor.isVirtual) {
        const regs = this.pendingVirtualListeners.get(ancestor);
        if (regs) {
          for (const reg of regs) {
            this.attachRegistrationToRealIds(reg, realIds);
          }
        }
      }
      ancestor = ancestor.parent;
    }
  }

  private attachGlobalWindowIcon(windowId: string): void {
    const iconMeta = (globalThis as { __NG_DESKTOP_ICON__?: { appId?: string; relPath?: string } })
      .__NG_DESKTOP_ICON__;
    if (iconMeta?.appId && iconMeta.relPath) {
      this.bridge.setWindowIcon(windowId, iconMeta.appId, iconMeta.relPath);
    }
  }

  private syncClassList(el: NativeElementNode, rawClasses: string): void {
    const nextClasses = new Set(
      rawClasses
        .split(/\s+/)
        .map((c) => c.trim())
        .filter(Boolean),
    );
    const toRemove: string[] = [];
    for (const existing of el.classes) {
      if (!nextClasses.has(existing)) {
        toRemove.push(existing);
      }
    }
    for (const existing of toRemove) {
      this.removeClass(el, existing);
    }
    for (const added of nextClasses) {
      if (!el.classes.has(added)) {
        this.addClass(el, added);
      }
    }
  }

  private handleImageSource(el: NativeElementNode, src: string): void {
    const seq = (this.imageLoadSeq.get(el) ?? 0) + 1;
    this.imageLoadSeq.set(el, seq);
    if (src.startsWith('http://') || src.startsWith('https://')) {
      if (typeof this.bridge.downloadImageAsync === 'function') {
        void this.bridge.downloadImageAsync(src).then((res) => {
          if (
            this.imageLoadSeq.get(el) === seq &&
            res?.ok &&
            res.path &&
            this.nodesById.has(el.id)
          ) {
            this.bridge.setText(el.id, res.path);
          }
        });
      } else {
        this.bridge.setText(el.id, this.bridge.resolveImageAsset(src));
      }
    } else {
      this.bridge.setText(el.id, src);
    }
  }

  private findRealAncestor(node: NativeElementNode | null): NativeElementNode | null {
    let cur = node;
    while (cur) {
      if (!cur.isVirtual && !cur.isText) {
        return cur;
      }
      cur = cur.parent;
    }
    return null;
  }

  private collectRealRootIds(node: NativeElementNode, out: string[] = []): string[] {
    if (!node.isVirtual && !node.isText) {
      out.push(node.id);
      return out;
    }
    if (node.isText) {
      const matId = this.materializedTextLabels.get(node);
      if (matId) {
        out.push(matId);
      }
      return out;
    }
    for (const child of node.children) {
      this.collectRealRootIds(child, out);
    }
    return out;
  }

  private findFirstRealId(node: NativeElementNode): string | null {
    if (!node.isVirtual && !node.isText) {
      return node.id;
    }
    if (node.isText) {
      return this.materializedTextLabels.get(node) ?? null;
    }
    for (const child of node.children) {
      const found = this.findFirstRealId(child);
      if (found) {
        return found;
      }
    }
    return null;
  }

  private findNextRealSiblingId(
    targetNode: NativeElementNode,
    realParent: NativeElementNode,
  ): string | null {
    let cur: NativeElementNode | null = targetNode;
    while (cur && cur !== realParent) {
      const parent: NativeElementNode | null = cur.parent;
      if (!parent) {
        break;
      }
      const siblings = parent.children;
      const idx = siblings.indexOf(cur);
      if (idx !== -1) {
        for (let i = idx + 1; i < siblings.length; i += 1) {
          const sibling = siblings[i];
          if (sibling) {
            const firstId = this.findFirstRealId(sibling);
            if (firstId) {
              return firstId;
            }
          }
        }
      }
      cur = parent;
    }
    return null;
  }

  private collectTextContent(node: NativeElementNode): string {
    let out = '';
    for (const child of node.children) {
      if (child.isText) {
        out += child.textContent;
      } else if (child.isVirtual) {
        out += this.collectTextContent(child);
      }
    }
    return out;
  }

  private syncTextBearingParent(realParent: NativeElementNode): void {
    const text = this.collectTextContent(realParent);
    this.bridge.setText(realParent.id, text);
  }

  private materializeTextIntoContainer(
    logicalParent: NativeElementNode,
    realParent: NativeElementNode,
    textNode: NativeElementNode,
    refChild: NativeElementNode | null,
    isAppend: boolean,
  ): void {
    let labelId = this.materializedTextLabels.get(textNode);
    const createdNow = !labelId;
    if (!labelId) {
      labelId = this.nextId();
      this.materializedTextLabels.set(textNode, labelId);
      this.bridge.createElement(labelId, 'ng-label');
    }
    this.bridge.setText(labelId, textNode.textContent);
    const nextRealId =
      refChild || logicalParent !== realParent
        ? this.findNextRealSiblingId(textNode, realParent)
        : null;
    if (nextRealId) {
      this.bridge.insertBefore(realParent.id, labelId, nextRealId);
    } else if (isAppend) {
      this.bridge.appendChild(realParent.id, labelId);
    } else {
      this.bridge.insertBefore(realParent.id, labelId);
    }
    if (createdNow) {
      this.attachVirtualListenersForAncestors(logicalParent, [labelId]);
    }
  }

  private projectInsertion(
    parent: NativeElementNode,
    newChild: NativeElementNode,
    refChild: NativeElementNode | null,
    isAppend: boolean,
  ): void {
    let realRootIds: string[] = [];
    if (!newChild.isText) {
      realRootIds = this.collectRealRootIds(newChild);
      if (realRootIds.length > 0) {
        this.attachVirtualListenersForAncestors(parent, realRootIds);
      }
    }

    const realParent = this.findRealAncestor(parent);
    if (!realParent) {
      return;
    }
    if (newChild.isText) {
      if (TEXT_BEARING_TAGS.has(realParent.tag)) {
        this.syncTextBearingParent(realParent);
      } else if (newChild.textContent.trim().length > 0) {
        this.materializeTextIntoContainer(parent, realParent, newChild, refChild, isAppend);
      }
      return;
    }

    if (realRootIds.length === 0) {
      return;
    }
    const nextRealId =
      refChild || parent !== realParent ? this.findNextRealSiblingId(newChild, realParent) : null;
    for (const realId of realRootIds) {
      if (nextRealId) {
        this.bridge.insertBefore(realParent.id, realId, nextRealId);
      } else if (isAppend) {
        this.bridge.appendChild(realParent.id, realId);
      } else {
        this.bridge.insertBefore(realParent.id, realId);
      }
    }
  }
}

@Injectable({ providedIn: 'root' })
export class NativeRendererFactory2 implements RendererFactory2 {
  private readonly bridge: NgNativeBridge;
  private readonly nodesById = new Map<string, NativeElementNode>();
  private readonly defaultRenderer: NativeRenderer2;
  private readonly registeredTypeIds = new Set<string>();
  private nodeCounter = 1;
  private primaryWindowId: string | null = null;
  private primaryScrollId: string | null = null;

  constructor() {
    this.bridge = ensureNgNativeBridge();
    this.defaultRenderer = new NativeRenderer2(
      this.bridge,
      this.nodesById,
      () => `ng_node_${this.nodeCounter++}`,
      {
        onRealElementCreated: (tag, id) => {
          if (tag === 'ng-window' || tag === 'adw-window') {
            if (!this.primaryWindowId || !this.nodesById.has(this.primaryWindowId)) {
              this.primaryWindowId = id;
            }
          } else if (tag === 'ng-scroll' || tag === 'adw-scroll') {
            if (!this.primaryScrollId || !this.nodesById.has(this.primaryScrollId)) {
              this.primaryScrollId = id;
            }
          }
        },
        getPrimaryWindowId: () => this.getPrimaryWindowId(),
      },
    );
  }

  createRenderer(_hostElement: unknown, type: RendererType2 | null): Renderer2 {
    if (type?.styles?.length) {
      const typeKey = type.id || '__anonymous__';
      if (!this.registeredTypeIds.has(typeKey)) {
        this.registeredTypeIds.add(typeKey);
        registerParsedStyles(this.bridge, type.styles);
      }
    }
    return this.defaultRenderer;
  }

  begin(): void {
    this.bridge.beginBatch?.();
  }

  end(): void {
    this.bridge.endBatch?.();
  }

  getPrimaryWindowId(): string | null {
    if (this.primaryWindowId && this.nodesById.has(this.primaryWindowId)) {
      return this.primaryWindowId;
    }
    for (const node of this.nodesById.values()) {
      if (!node.isVirtual && (node.tag === 'ng-window' || node.tag === 'adw-window')) {
        this.primaryWindowId = node.id;
        return node.id;
      }
    }
    this.primaryWindowId = null;
    return null;
  }

  getPrimaryScrollId(): string | null {
    if (this.primaryScrollId && this.nodesById.has(this.primaryScrollId)) {
      return this.primaryScrollId;
    }
    for (const node of this.nodesById.values()) {
      if (!node.isVirtual && (node.tag === 'ng-scroll' || node.tag === 'adw-scroll')) {
        this.primaryScrollId = node.id;
        return node.id;
      }
    }
    this.primaryScrollId = null;
    return null;
  }

  getNode(id: string): NativeElementNode | undefined {
    return this.nodesById.get(id);
  }
}

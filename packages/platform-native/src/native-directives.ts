import { Directive, ElementRef, inject, Input, Renderer2 } from '@angular/core';

export type NativeAttrInput = string | number | boolean | null | undefined;
export type NativeBoolInput = boolean | string | number | null | undefined;
export type NativeNumberInput = number | string | null | undefined;

interface AttributeStorage {
  attributes?: Map<string, string> | { getNamedItem?: (name: string) => { value: string } | null };
}

function getElementAttribute(el: unknown, name: string): string | undefined {
  if (!el || typeof el !== 'object') {
    return undefined;
  }
  const attrs = (el as AttributeStorage).attributes;
  if (attrs instanceof Map) {
    return attrs.get(name);
  }
  if (attrs && typeof attrs.getNamedItem === 'function') {
    return attrs.getNamedItem(name)?.value;
  }
  return undefined;
}

function setCachedAttribute(el: unknown, name: string, value: string): void {
  if (!el || typeof el !== 'object') {
    return;
  }
  const attrs = (el as AttributeStorage).attributes;
  if (attrs instanceof Map) {
    attrs.set(name, value);
  }
}

function hasElementAttribute(el: unknown, name: string): boolean {
  if (!el || typeof el !== 'object') {
    return false;
  }
  const attrs = (el as AttributeStorage).attributes;
  if (attrs instanceof Map) {
    return attrs.has(name);
  }
  if (attrs && typeof attrs.getNamedItem === 'function') {
    return attrs.getNamedItem(name) !== null;
  }
  return false;
}

function syncNativeAttr(
  renderer: Renderer2,
  el: unknown,
  name: string,
  value: NativeAttrInput,
): void {
  if (value === undefined) {
    return;
  }
  if (value === null) {
    if (hasElementAttribute(el, name)) {
      renderer.removeAttribute(el, name);
    }
    return;
  }
  const strVal = String(value);
  if (getElementAttribute(el, name) === strVal) {
    return;
  }
  renderer.setAttribute(el, name, strVal);
}

function syncNativeProp(
  renderer: Renderer2,
  el: unknown,
  name: string,
  value: NativeAttrInput,
): void {
  if (value === undefined) {
    return;
  }
  const strVal = value === null ? '' : String(value);
  if (getElementAttribute(el, name) === strVal) {
    return;
  }
  setCachedAttribute(el, name, strVal);
  renderer.setProperty(el, name, strVal);
}

function isTruthyDisabledInput(value: NativeBoolInput): boolean {
  if (value === undefined || value === null || value === false || value === 0) {
    return false;
  }
  if (typeof value === 'string') {
    const lower = value.trim().toLowerCase();
    return lower !== 'false' && lower !== '0' && lower !== 'null' && lower !== 'undefined';
  }
  return Boolean(value);
}

/**
 * Common layout, visibility, accessibility, and interaction directive covering all native desktop
 * widget elements (`<ng-*>`, `<gtk-*>`, `<adw-*>`).
 */
@Directive({
  selector:
    'ng-window,adw-window,ng-dialog,gtk-dialog,adw-dialog,ng-header-bar,adw-header-bar,gtk-header-bar,ng-scroll,gtk-scrolled-window,ng-box,gtk-box,ng-grid,gtk-grid,ng-separator,gtk-separator,ng-button,gtk-button,ng-label,gtk-label,ng-entry,gtk-entry,ng-switch,gtk-switch,ng-checkbox,ng-check,ng-check-button,gtk-check-button,ng-slider,ng-scale,gtk-scale,ng-select,ng-dropdown,gtk-dropdown,gtk-drop-down,ng-image,adw-image,gtk-picture,ng-picture,ng-level-bar,gtk-level-bar,ng-flow,ng-flow-box,gtk-flow-box,ng-spinner,gtk-spinner,ng-progress-bar,gtk-progress-bar,ng-paned,gtk-paned,ng-stack,gtk-stack,adw-navigation-view,ng-expander,gtk-expander,ng-text-view,gtk-text-view,ng-password-entry,gtk-password-entry,ng-spin-button,gtk-spin-button,ng-search-entry,gtk-search-entry,ng-clamp,adw-clamp,adw-entry-row,adw-password-entry-row,adw-switch-row,adw-combo-row,adw-spin-row',
  standalone: true,
})
export class NativeElementDirective {
  private readonly renderer = inject(Renderer2);
  private readonly el = inject<ElementRef<unknown>>(ElementRef).nativeElement;

  @Input() set hexpand(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'hexpand', val);
  }
  @Input() set vexpand(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'vexpand', val);
  }
  @Input() set halign(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'halign', val);
  }
  @Input() set valign(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'valign', val);
  }
  @Input() set width(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'width', val);
  }
  @Input() set height(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'height', val);
  }
  @Input('min-width') set minWidth(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'min-width', val);
  }
  @Input('min-height') set minHeight(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'min-height', val);
  }
  @Input() set margin(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'margin', val);
  }
  @Input('margin-top') set marginTop(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'margin-top', val);
  }
  @Input('margin-bottom') set marginBottom(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'margin-bottom', val);
  }
  @Input('margin-start') set marginStart(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'margin-start', val);
  }
  @Input('margin-end') set marginEnd(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'margin-end', val);
  }
  @Input() set visible(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'visible', val);
  }
  @Input() set sensitive(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'sensitive', val);
  }
  @Input() set disabled(val: NativeBoolInput) {
    if (val === undefined) {
      return;
    }
    const nextSensitive = isTruthyDisabledInput(val) ? 'false' : 'true';
    if (getElementAttribute(this.el, 'sensitive') === nextSensitive) {
      return;
    }
    this.renderer.setProperty(this.el, 'disabled', val);
  }
  @Input() set tooltip(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'tooltip', val);
  }
  @Input('event-mask') set eventMask(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'event-mask', val);
  }
  @Input('drop-target') set dropTarget(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'drop-target', val);
  }
}

/**
 * Typed directive for top-level native windows, dialogs, and header bars (`<ng-window>`,
 * `<ng-dialog>`, `<ng-header-bar>`).
 */
@Directive({
  selector:
    'ng-window,adw-window,ng-dialog,gtk-dialog,adw-dialog,ng-header-bar,adw-header-bar,gtk-header-bar',
  standalone: true,
})
export class NativeWindowDirective {
  private readonly renderer = inject(Renderer2);
  private readonly el = inject<ElementRef<unknown>>(ElementRef).nativeElement;

  @Input() set title(val: NativeAttrInput) {
    syncNativeProp(this.renderer, this.el, 'title', val);
  }
  @Input() set subtitle(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'subtitle', val);
  }
  @Input('show-title-buttons') set showTitleButtons(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'show-title-buttons', val);
  }
  @Input() set icon(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'icon', val);
  }
  @Input() set resizable(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'resizable', val);
  }
  @Input() set decorated(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'decorated', val);
  }
  @Input() set modal(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'modal', val);
  }
  @Input('transient-for') set transientForAttr(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'transient-for', val);
  }
  @Input() set transientFor(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'transient-for', val);
  }
  @Input() set preventClose(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'prevent-close', val);
  }
  @Input('prevent-close') set preventCloseAttr(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'prevent-close', val);
  }
}

/**
 * Typed directive for native layout containers, grids, flow boxes, split panes, stacks, clamps,
 * scroll viewports, separators, and expanders.
 */
@Directive({
  selector:
    'ng-box,gtk-box,ng-grid,gtk-grid,ng-flow,ng-flow-box,gtk-flow-box,ng-paned,gtk-paned,ng-scroll,gtk-scrolled-window,ng-stack,gtk-stack,adw-navigation-view,ng-expander,gtk-expander,ng-clamp,adw-clamp,ng-separator,gtk-separator',
  standalone: true,
})
export class NativeContainerDirective {
  private readonly renderer = inject(Renderer2);
  private readonly el = inject<ElementRef<unknown>>(ElementRef).nativeElement;

  @Input() set orientation(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'orientation', val);
  }
  @Input() set spacing(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'spacing', val);
  }
  @Input() set gap(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'gap', val);
  }
  @Input('row-spacing') set rowSpacing(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'row-spacing', val);
  }
  @Input('column-spacing') set columnSpacing(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'column-spacing', val);
  }
  @Input('min-children') set minChildren(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'min-children', val);
  }
  @Input('max-children') set maxChildren(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'max-children', val);
  }
  @Input('min-per-row') set minPerRow(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'min-per-row', val);
  }
  @Input('max-per-row') set maxPerRow(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'max-per-row', val);
  }
  @Input() set homogeneous(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'homogeneous', val);
  }
  @Input() set position(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'position', val);
  }
  @Input('visible-child') set visibleChild(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'visible-child', val);
  }
  @Input('active-page') set activePage(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'active-page', val);
  }
  @Input() set transition(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'transition', val);
  }
  @Input('transition-type') set transitionType(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'transition-type', val);
  }
  @Input() set label(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'label', val);
  }
  @Input() set title(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'title', val);
  }
  @Input() set expanded(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'expanded', val);
  }
  @Input() set open(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'open', val);
  }
  @Input('max-size') set maxSize(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'max-size', val);
  }
  @Input('maximum-size') set maximumSize(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'maximum-size', val);
  }
  @Input('scroll-to') set scrollTo(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'scroll-to', val);
  }
  @Input('scroll-anchor') set scrollAnchor(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'scroll-anchor', val);
  }
}

/**
 * Typed directive for native labels and action buttons (`<ng-label>`, `<ng-button>`).
 */
@Directive({
  selector: 'ng-button,gtk-button,ng-label,gtk-label',
  standalone: true,
})
export class NativeTextWidgetDirective {
  private readonly renderer = inject(Renderer2);
  private readonly el = inject<ElementRef<unknown>>(ElementRef).nativeElement;

  @Input() set label(val: NativeAttrInput) {
    syncNativeProp(this.renderer, this.el, 'label', val);
  }
  @Input() set text(val: NativeAttrInput) {
    syncNativeProp(this.renderer, this.el, 'text', val);
  }
  @Input() set wrap(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'wrap', val);
  }
  @Input() set selectable(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'selectable', val);
  }
  @Input() set ellipsize(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'ellipsize', val);
  }
}

/**
 * Typed directive for native images, level bars, progress bars, and spinners.
 */
@Directive({
  selector:
    'ng-image,adw-image,gtk-picture,ng-picture,ng-level-bar,gtk-level-bar,ng-progress-bar,gtk-progress-bar,ng-spinner,gtk-spinner',
  standalone: true,
})
export class NativeMediaAndMeterDirective {
  private readonly renderer = inject(Renderer2);
  private readonly el = inject<ElementRef<unknown>>(ElementRef).nativeElement;

  @Input() set src(val: NativeAttrInput) {
    syncNativeProp(this.renderer, this.el, 'src', val);
  }
  @Input() set ngSrc(val: NativeAttrInput) {
    syncNativeProp(this.renderer, this.el, 'ngSrc', val);
  }
  @Input() set alt(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'alt', val);
  }
  @Input() set value(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'value', val);
  }
  @Input() set min(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'min', val);
  }
  @Input() set max(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'max', val);
  }
  @Input() set indeterminate(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'indeterminate', val);
  }
  @Input() set active(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'active', val);
  }
  @Input() set spinning(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'spinning', val);
  }
}

/**
 * Typed directive for native text inputs, switches, checkboxes, sliders, dropdowns, and spin buttons.
 */
@Directive({
  selector:
    'ng-entry,gtk-entry,ng-search-entry,gtk-search-entry,ng-password-entry,gtk-password-entry,ng-text-view,gtk-text-view,ng-switch,gtk-switch,ng-checkbox,ng-check,ng-check-button,gtk-check-button,ng-slider,ng-scale,gtk-scale,ng-spin-button,gtk-spin-button,ng-select,ng-dropdown,gtk-dropdown,gtk-drop-down,adw-entry-row,adw-password-entry-row,adw-switch-row,adw-combo-row,adw-spin-row',
  standalone: true,
})
export class NativeInputWidgetDirective {
  private readonly renderer = inject(Renderer2);
  private readonly el = inject<ElementRef<unknown>>(ElementRef).nativeElement;

  @Input() set placeholder(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'placeholder', val);
  }
  @Input() set value(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'value', val);
  }
  @Input() set text(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'text', val);
  }
  @Input() set active(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'active', val);
  }
  @Input() set checked(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'checked', val);
  }
  @Input() set label(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'label', val);
  }
  @Input() set min(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'min', val);
  }
  @Input() set max(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'max', val);
  }
  @Input() set step(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'step', val);
  }
  @Input() set options(val: NativeAttrInput) {
    syncNativeAttr(this.renderer, this.el, 'options', val);
  }
  @Input() set selected(val: NativeNumberInput) {
    syncNativeAttr(this.renderer, this.el, 'selected', val);
  }
  @Input() set editable(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'editable', val);
  }
  @Input() set monospace(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'monospace', val);
  }
  @Input('show-peek-icon') set showPeekIcon(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'show-peek-icon', val);
  }
  @Input() set peek(val: NativeBoolInput) {
    syncNativeAttr(this.renderer, this.el, 'peek', val);
  }
}

/**
 * Convenience directive bundle enabling typed bindings on `<ng-*>`, `<gtk-*>`, and `<adw-*>` tags
 * in standalone Angular components without `CUSTOM_ELEMENTS_SCHEMA`.
 */
export const NATIVE_UI_DIRECTIVES = [
  NativeElementDirective,
  NativeWindowDirective,
  NativeContainerDirective,
  NativeTextWidgetDirective,
  NativeMediaAndMeterDirective,
  NativeInputWidgetDirective,
] as const;

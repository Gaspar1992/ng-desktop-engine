import {
  computed,
  Directive,
  effect,
  ElementRef,
  inject,
  input,
  Renderer2,
  Signal,
} from '@angular/core';

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

type SignalBinding = readonly [attr: string, source: Signal<NativeAttrInput>, asProp?: boolean];

/**
 * Synchronizes Signal `input()` values to the native renderer without clobbering attributes
 * managed by static template attributes, text interpolation, or `[formField]` directives.
 */
function bindSignalInputs(bindings: readonly SignalBinding[]): void {
  const renderer = inject(Renderer2);
  const el = inject<ElementRef<unknown>>(ElementRef).nativeElement;
  const boundKeys = new Set<string>();

  effect(() => {
    for (const [name, source, asProp] of bindings) {
      const value = source();
      if (value === undefined) {
        if (boundKeys.has(name)) {
          if (asProp) {
            syncNativeProp(renderer, el, name, null);
          } else {
            syncNativeAttr(renderer, el, name, null);
          }
        }
        continue;
      }
      boundKeys.add(name);
      if (asProp) {
        syncNativeProp(renderer, el, name, value);
      } else {
        syncNativeAttr(renderer, el, name, value);
      }
    }
  });
}

/**
 * Common layout, visibility, accessibility, and interaction directive covering all native desktop
 * widget elements (`<ng-*>`, `<gtk-*>`, `<adw-*>`).
 */
@Directive({
  selector:
    'ng-window,adw-window,ng-dialog,gtk-dialog,adw-dialog,ng-header-bar,adw-header-bar,gtk-header-bar,ng-scroll,gtk-scrolled-window,ng-box,gtk-box,ng-grid,gtk-grid,ng-separator,gtk-separator,ng-button,gtk-button,ng-label,gtk-label,ng-entry,gtk-entry,ng-switch,gtk-switch,ng-checkbox,ng-check,ng-check-button,gtk-check-button,ng-slider,ng-scale,gtk-scale,ng-select,ng-dropdown,gtk-dropdown,gtk-drop-down,ng-image,adw-image,gtk-picture,ng-picture,ng-level-bar,gtk-level-bar,ng-flow,ng-flow-box,gtk-flow-box,ng-spinner,gtk-spinner,ng-progress-bar,gtk-progress-bar,ng-paned,gtk-paned,ng-stack,gtk-stack,adw-navigation-view,ng-expander,gtk-expander,ng-text-view,gtk-text-view,ng-password-entry,gtk-password-entry,ng-spin-button,gtk-spin-button,ng-search-entry,gtk-search-entry,ng-clamp,adw-clamp,ng-list,ng-table,gtk-list-box,gtk-list-view,ng-popover,gtk-popover,ng-menu,adw-entry-row,adw-password-entry-row,adw-switch-row,adw-combo-row,adw-spin-row',
})
export class NativeElementDirective {
  readonly hexpand = input<NativeBoolInput>();
  readonly vexpand = input<NativeBoolInput>();
  readonly halign = input<NativeAttrInput>();
  readonly valign = input<NativeAttrInput>();
  readonly width = input<NativeNumberInput>();
  readonly height = input<NativeNumberInput>();
  readonly minWidth = input<NativeNumberInput>(undefined, { alias: 'min-width' });
  readonly minHeight = input<NativeNumberInput>(undefined, { alias: 'min-height' });
  readonly margin = input<NativeNumberInput>();
  readonly marginTop = input<NativeNumberInput>(undefined, { alias: 'margin-top' });
  readonly marginBottom = input<NativeNumberInput>(undefined, { alias: 'margin-bottom' });
  readonly marginStart = input<NativeNumberInput>(undefined, { alias: 'margin-start' });
  readonly marginEnd = input<NativeNumberInput>(undefined, { alias: 'margin-end' });
  readonly visible = input<NativeBoolInput>();
  readonly sensitive = input<NativeBoolInput>();
  readonly disabled = input<NativeBoolInput>();
  readonly tooltip = input<NativeAttrInput>();
  readonly eventMask = input<NativeAttrInput>(undefined, { alias: 'event-mask' });
  readonly dropTarget = input<NativeBoolInput>(undefined, { alias: 'drop-target' });
  readonly col = input<NativeNumberInput>();
  readonly row = input<NativeNumberInput>();

  private readonly resolvedSensitive = computed<NativeBoolInput>(() => {
    const disabled = this.disabled();
    if (disabled !== undefined) {
      return isTruthyDisabledInput(disabled) ? 'false' : 'true';
    }
    return this.sensitive();
  });

  constructor() {
    bindSignalInputs([
      ['hexpand', this.hexpand],
      ['vexpand', this.vexpand],
      ['halign', this.halign],
      ['valign', this.valign],
      ['width', this.width],
      ['height', this.height],
      ['min-width', this.minWidth],
      ['min-height', this.minHeight],
      ['margin', this.margin],
      ['margin-top', this.marginTop],
      ['margin-bottom', this.marginBottom],
      ['margin-start', this.marginStart],
      ['margin-end', this.marginEnd],
      ['visible', this.visible],
      ['sensitive', this.resolvedSensitive],
      ['tooltip', this.tooltip],
      ['event-mask', this.eventMask],
      ['drop-target', this.dropTarget],
      ['col', this.col],
      ['row', this.row],
    ]);
  }
}

/**
 * Typed directive for top-level native windows, dialogs, and header bars (`<ng-window>`,
 * `<ng-dialog>`, `<ng-header-bar>`).
 */
@Directive({
  selector:
    'ng-window,adw-window,ng-dialog,gtk-dialog,adw-dialog,ng-header-bar,adw-header-bar,gtk-header-bar',
})
export class NativeWindowDirective {
  readonly title = input<NativeAttrInput>();
  readonly subtitle = input<NativeAttrInput>();
  readonly showTitleButtons = input<NativeBoolInput>(undefined, { alias: 'show-title-buttons' });
  readonly icon = input<NativeAttrInput>();
  readonly resizable = input<NativeBoolInput>();
  readonly decorated = input<NativeBoolInput>();
  readonly modal = input<NativeBoolInput>();
  readonly transientForAttr = input<NativeAttrInput>(undefined, { alias: 'transient-for' });
  readonly transientFor = input<NativeAttrInput>();
  readonly preventClose = input<NativeBoolInput>();
  readonly preventCloseAttr = input<NativeBoolInput>(undefined, { alias: 'prevent-close' });
  readonly backdrop = input<NativeAttrInput>();
  readonly theme = input<NativeAttrInput>();

  private readonly resolvedTransientFor = computed(
    () => this.transientFor() ?? this.transientForAttr(),
  );
  private readonly resolvedPreventClose = computed(
    () => this.preventClose() ?? this.preventCloseAttr(),
  );

  constructor() {
    bindSignalInputs([
      ['title', this.title, true],
      ['subtitle', this.subtitle],
      ['show-title-buttons', this.showTitleButtons],
      ['icon', this.icon],
      ['resizable', this.resizable],
      ['decorated', this.decorated],
      ['modal', this.modal],
      ['transient-for', this.resolvedTransientFor],
      ['prevent-close', this.resolvedPreventClose],
      ['backdrop', this.backdrop],
      ['theme', this.theme],
    ]);
  }
}

/**
 * Typed directive for native layout containers, grids, flow boxes, split panes, stacks, clamps,
 * scroll viewports, separators, and expanders.
 */
@Directive({
  selector:
    'ng-box,gtk-box,ng-grid,gtk-grid,ng-flow,ng-flow-box,gtk-flow-box,ng-paned,gtk-paned,ng-scroll,gtk-scrolled-window,ng-stack,gtk-stack,adw-navigation-view,ng-expander,gtk-expander,ng-clamp,adw-clamp,ng-separator,gtk-separator',
})
export class NativeContainerDirective {
  readonly orientation = input<NativeAttrInput>();
  readonly spacing = input<NativeNumberInput>();
  readonly gap = input<NativeNumberInput>();
  readonly rowSpacing = input<NativeNumberInput>(undefined, { alias: 'row-spacing' });
  readonly columnSpacing = input<NativeNumberInput>(undefined, { alias: 'column-spacing' });
  readonly minChildren = input<NativeNumberInput>(undefined, { alias: 'min-children' });
  readonly maxChildren = input<NativeNumberInput>(undefined, { alias: 'max-children' });
  readonly minPerRow = input<NativeNumberInput>(undefined, { alias: 'min-per-row' });
  readonly maxPerRow = input<NativeNumberInput>(undefined, { alias: 'max-per-row' });
  readonly columns = input<NativeNumberInput>();
  readonly homogeneous = input<NativeBoolInput>();
  readonly position = input<NativeNumberInput>();
  readonly visibleChild = input<NativeAttrInput>(undefined, { alias: 'visible-child' });
  readonly activePage = input<NativeAttrInput>(undefined, { alias: 'active-page' });
  readonly transition = input<NativeAttrInput>();
  readonly transitionType = input<NativeAttrInput>(undefined, { alias: 'transition-type' });
  readonly label = input<NativeAttrInput>();
  readonly title = input<NativeAttrInput>();
  readonly expanded = input<NativeBoolInput>();
  readonly open = input<NativeBoolInput>();
  readonly maxSize = input<NativeNumberInput>(undefined, { alias: 'max-size' });
  readonly maximumSize = input<NativeNumberInput>(undefined, { alias: 'maximum-size' });
  readonly scrollTo = input<NativeAttrInput>(undefined, { alias: 'scroll-to' });
  readonly scrollAnchor = input<NativeAttrInput>(undefined, { alias: 'scroll-anchor' });

  constructor() {
    bindSignalInputs([
      ['orientation', this.orientation],
      ['spacing', this.spacing],
      ['gap', this.gap],
      ['row-spacing', this.rowSpacing],
      ['column-spacing', this.columnSpacing],
      ['min-children', this.minChildren],
      ['max-children', this.maxChildren],
      ['min-per-row', this.minPerRow],
      ['max-per-row', this.maxPerRow],
      ['columns', this.columns],
      ['homogeneous', this.homogeneous],
      ['position', this.position],
      ['visible-child', this.visibleChild],
      ['active-page', this.activePage],
      ['transition', this.transition],
      ['transition-type', this.transitionType],
      ['label', this.label],
      ['title', this.title],
      ['expanded', this.expanded],
      ['open', this.open],
      ['max-size', this.maxSize],
      ['maximum-size', this.maximumSize],
      ['scroll-to', this.scrollTo],
      ['scroll-anchor', this.scrollAnchor],
    ]);
  }
}

/**
 * Typed directive for native labels and action buttons (`<ng-label>`, `<ng-button>`).
 */
@Directive({
  selector: 'ng-button,gtk-button,ng-label,gtk-label',
})
export class NativeTextWidgetDirective {
  readonly label = input<NativeAttrInput>();
  readonly text = input<NativeAttrInput>();
  readonly wrap = input<NativeBoolInput>();
  readonly selectable = input<NativeBoolInput>();
  readonly ellipsize = input<NativeAttrInput>();

  constructor() {
    bindSignalInputs([
      ['label', this.label, true],
      ['text', this.text, true],
      ['wrap', this.wrap],
      ['selectable', this.selectable],
      ['ellipsize', this.ellipsize],
    ]);
  }
}

/**
 * Typed directive for native images, level bars, progress bars, and spinners.
 */
@Directive({
  selector:
    'ng-image,adw-image,gtk-picture,ng-picture,ng-level-bar,gtk-level-bar,ng-progress-bar,gtk-progress-bar,ng-spinner,gtk-spinner',
})
export class NativeMediaAndMeterDirective {
  readonly src = input<NativeAttrInput>();
  readonly ngSrc = input<NativeAttrInput>();
  readonly alt = input<NativeAttrInput>();
  readonly value = input<NativeNumberInput>();
  readonly min = input<NativeNumberInput>();
  readonly max = input<NativeNumberInput>();
  readonly indeterminate = input<NativeBoolInput>();
  readonly active = input<NativeBoolInput>();
  readonly spinning = input<NativeBoolInput>();

  constructor() {
    bindSignalInputs([
      ['src', this.src, true],
      ['ngSrc', this.ngSrc, true],
      ['alt', this.alt],
      ['value', this.value],
      ['min', this.min],
      ['max', this.max],
      ['indeterminate', this.indeterminate],
      ['active', this.active],
      ['spinning', this.spinning],
    ]);
  }
}

/**
 * Typed directive for native text inputs, switches, checkboxes, sliders, dropdowns, and spin buttons.
 */
@Directive({
  selector:
    'ng-entry,gtk-entry,ng-search-entry,gtk-search-entry,ng-password-entry,gtk-password-entry,ng-text-view,gtk-text-view,ng-switch,gtk-switch,ng-checkbox,ng-check,ng-check-button,gtk-check-button,ng-slider,ng-scale,gtk-scale,ng-spin-button,gtk-spin-button,ng-select,ng-dropdown,gtk-dropdown,gtk-drop-down,adw-entry-row,adw-password-entry-row,adw-switch-row,adw-combo-row,adw-spin-row',
})
export class NativeInputWidgetDirective {
  readonly placeholder = input<NativeAttrInput>();
  readonly value = input<NativeAttrInput>();
  readonly text = input<NativeAttrInput>();
  readonly active = input<NativeBoolInput>();
  readonly checked = input<NativeBoolInput>();
  readonly label = input<NativeAttrInput>();
  readonly options = input<NativeAttrInput>();
  readonly selected = input<NativeNumberInput>();
  readonly editable = input<NativeBoolInput>();
  readonly monospace = input<NativeBoolInput>();
  readonly showPeekIcon = input<NativeBoolInput>(undefined, { alias: 'show-peek-icon' });
  readonly peek = input<NativeBoolInput>();

  constructor() {
    bindSignalInputs([
      ['placeholder', this.placeholder],
      ['value', this.value],
      ['text', this.text],
      ['active', this.active],
      ['checked', this.checked],
      ['label', this.label],
      ['options', this.options],
      ['selected', this.selected],
      ['editable', this.editable],
      ['monospace', this.monospace],
      ['show-peek-icon', this.showPeekIcon],
      ['peek', this.peek],
    ]);
  }
}

/**
 * Typed directive for native virtualized lists and tables (`<ng-list>`, `<ng-table>`).
 */
@Directive({
  selector: 'ng-list,ng-table,gtk-list-box,gtk-list-view',
})
export class NativeListDirective {
  readonly selectionModeAttr = input<NativeAttrInput>(undefined, { alias: 'selection-mode' });
  readonly selectionMode = input<NativeAttrInput>();
  readonly showSeparatorsAttr = input<NativeBoolInput>(undefined, { alias: 'show-separators' });
  readonly showSeparators = input<NativeBoolInput>();
  readonly separators = input<NativeBoolInput>();

  private readonly resolvedSelectionMode = computed(
    () => this.selectionMode() ?? this.selectionModeAttr(),
  );
  private readonly resolvedShowSeparators = computed(
    () => this.showSeparators() ?? this.showSeparatorsAttr(),
  );

  constructor() {
    bindSignalInputs([
      ['selection-mode', this.resolvedSelectionMode],
      ['show-separators', this.resolvedShowSeparators],
      ['separators', this.separators],
    ]);
  }
}

/**
 * Typed directive for native anchored popovers and context menus (`<ng-popover>`, `<ng-menu>`).
 */
@Directive({
  selector: 'ng-popover,gtk-popover,ng-menu',
})
export class NativePopoverDirective {
  readonly open = input<NativeBoolInput>();
  readonly position = input<NativeAttrInput>();
  readonly placement = input<NativeAttrInput>();
  readonly autohide = input<NativeBoolInput>();
  readonly hasArrowAttr = input<NativeBoolInput>(undefined, { alias: 'has-arrow' });
  readonly hasArrow = input<NativeBoolInput>();
  readonly anchor = input<NativeAttrInput>();

  private readonly resolvedHasArrow = computed(() => this.hasArrow() ?? this.hasArrowAttr());

  constructor() {
    bindSignalInputs([
      ['open', this.open],
      ['position', this.position],
      ['placement', this.placement],
      ['autohide', this.autohide],
      ['has-arrow', this.resolvedHasArrow],
      ['anchor', this.anchor],
    ]);
  }
}

/**
 * Typed directive for native numeric range & stepper inputs (`<ng-slider>`, `<ng-spin-button>`),
 * isolating `min`/`max`/`step` away from boolean toggles for Signal Forms type safety.
 */
@Directive({
  selector: 'ng-slider,ng-scale,gtk-scale,ng-spin-button,gtk-spin-button,adw-spin-row',
})
export class NativeNumericRangeDirective {
  readonly min = input<NativeNumberInput>();
  readonly max = input<NativeNumberInput>();
  readonly step = input<NativeNumberInput>();

  constructor() {
    bindSignalInputs([
      ['min', this.min],
      ['max', this.max],
      ['step', this.step],
    ]);
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
  NativeNumericRangeDirective,
  NativeListDirective,
  NativePopoverDirective,
] as const;

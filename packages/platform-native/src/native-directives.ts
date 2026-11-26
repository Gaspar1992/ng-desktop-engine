import { computed, Directive, input } from '@angular/core';

export type NativeAttrInput = string | number | boolean | null | undefined;
export type NativeBoolInput = boolean | string | number | null | undefined;
export type NativeNumberInput = number | string | null | undefined;

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
    'ng-window,adw-window,ng-dialog,gtk-dialog,adw-dialog,ng-header-bar,adw-header-bar,gtk-header-bar,ng-scroll,gtk-scrolled-window,ng-box,gtk-box,ng-grid,gtk-grid,ng-separator,gtk-separator,ng-button,gtk-button,ng-label,gtk-label,ng-entry,gtk-entry,ng-switch,gtk-switch,ng-checkbox,ng-check,ng-check-button,gtk-check-button,ng-slider,ng-scale,gtk-scale,ng-select,ng-dropdown,gtk-dropdown,gtk-drop-down,ng-image,adw-image,gtk-picture,ng-picture,ng-level-bar,gtk-level-bar,ng-flow,ng-flow-box,gtk-flow-box,ng-spinner,gtk-spinner,ng-progress-bar,gtk-progress-bar,ng-paned,gtk-paned,ng-stack,gtk-stack,adw-navigation-view,ng-expander,gtk-expander,ng-text-view,gtk-text-view,ng-password-entry,gtk-password-entry,ng-spin-button,gtk-spin-button,ng-search-entry,gtk-search-entry,ng-clamp,adw-clamp,ng-list,ng-table,gtk-list-box,gtk-list-view,ng-popover,gtk-popover,ng-menu,adw-entry-row,adw-password-entry-row,adw-switch-row,adw-combo-row,adw-spin-row',
  host: {
    '[attr.hexpand]': 'hexpand()',
    '[attr.vexpand]': 'vexpand()',
    '[attr.halign]': 'halign()',
    '[attr.valign]': 'valign()',
    '[attr.width]': 'width()',
    '[attr.height]': 'height()',
    '[attr.min-width]': 'minWidth()',
    '[attr.min-height]': 'minHeight()',
    '[attr.margin]': 'margin()',
    '[attr.margin-top]': 'marginTop()',
    '[attr.margin-bottom]': 'marginBottom()',
    '[attr.margin-start]': 'marginStart()',
    '[attr.margin-end]': 'marginEnd()',
    '[attr.visible]': 'visible()',
    '[attr.sensitive]': 'resolvedSensitive()',
    '[attr.tooltip]': 'tooltip()',
    '[attr.event-mask]': 'eventMask()',
    '[attr.drop-target]': 'dropTarget()',
    '[attr.col]': 'col()',
    '[attr.row]': 'row()',
  },
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

  protected readonly resolvedSensitive = computed<NativeBoolInput>(() => {
    const disabled = this.disabled();
    if (disabled !== undefined) {
      return isTruthyDisabledInput(disabled) ? 'false' : 'true';
    }
    return this.sensitive();
  });
}

/**
 * Typed directive for top-level native windows, dialogs, and header bars (`<ng-window>`,
 * `<ng-dialog>`, `<ng-header-bar>`).
 */
@Directive({
  selector:
    'ng-window,adw-window,ng-dialog,gtk-dialog,adw-dialog,ng-header-bar,adw-header-bar,gtk-header-bar',
  host: {
    '[attr.title]': 'title()',
    '[attr.subtitle]': 'subtitle()',
    '[attr.show-title-buttons]': 'showTitleButtons()',
    '[attr.icon]': 'icon()',
    '[attr.resizable]': 'resizable()',
    '[attr.decorated]': 'decorated()',
    '[attr.modal]': 'modal()',
    '[attr.transient-for]': 'transientFor() ?? transientForAttr()',
    '[attr.prevent-close]': 'preventClose() ?? preventCloseAttr()',
    '[attr.backdrop]': 'backdrop()',
    '[attr.theme]': 'theme()',
  },
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
}

/**
 * Typed directive for native layout containers, grids, flow boxes, split panes, stacks, clamps,
 * scroll viewports, separators, and expanders.
 */
@Directive({
  selector:
    'ng-box,gtk-box,ng-grid,gtk-grid,ng-flow,ng-flow-box,gtk-flow-box,ng-paned,gtk-paned,ng-scroll,gtk-scrolled-window,ng-stack,gtk-stack,adw-navigation-view,ng-expander,gtk-expander,ng-clamp,adw-clamp,ng-separator,gtk-separator',
  host: {
    '[attr.orientation]': 'orientation()',
    '[attr.spacing]': 'spacing()',
    '[attr.gap]': 'gap()',
    '[attr.row-spacing]': 'rowSpacing()',
    '[attr.column-spacing]': 'columnSpacing()',
    '[attr.min-children]': 'minChildren()',
    '[attr.max-children]': 'maxChildren()',
    '[attr.min-per-row]': 'minPerRow()',
    '[attr.max-per-row]': 'maxPerRow()',
    '[attr.columns]': 'columns()',
    '[attr.homogeneous]': 'homogeneous()',
    '[attr.position]': 'position()',
    '[attr.visible-child]': 'visibleChild()',
    '[attr.active-page]': 'activePage()',
    '[attr.transition]': 'transition()',
    '[attr.transition-type]': 'transitionType()',
    '[attr.label]': 'label()',
    '[attr.title]': 'title()',
    '[attr.expanded]': 'expanded()',
    '[attr.open]': 'open()',
    '[attr.max-size]': 'maxSize()',
    '[attr.maximum-size]': 'maximumSize()',
    '[attr.scroll-to]': 'scrollTo()',
    '[attr.scroll-anchor]': 'scrollAnchor()',
  },
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
}

/**
 * Typed directive for native labels and action buttons (`<ng-label>`, `<ng-button>`).
 */
@Directive({
  selector: 'ng-button,gtk-button,ng-label,gtk-label',
  host: {
    '[attr.label]': 'label()',
    '[attr.text]': 'text()',
    '[attr.wrap]': 'wrap()',
    '[attr.selectable]': 'selectable()',
    '[attr.ellipsize]': 'ellipsize()',
  },
})
export class NativeTextWidgetDirective {
  readonly label = input<NativeAttrInput>();
  readonly text = input<NativeAttrInput>();
  readonly wrap = input<NativeBoolInput>();
  readonly selectable = input<NativeBoolInput>();
  readonly ellipsize = input<NativeAttrInput>();
}

/**
 * Typed directive for native images, level bars, progress bars, and spinners.
 */
@Directive({
  selector:
    'ng-image,adw-image,gtk-picture,ng-picture,ng-level-bar,gtk-level-bar,ng-progress-bar,gtk-progress-bar,ng-spinner,gtk-spinner',
  host: {
    '[attr.src]': 'src()',
    '[attr.ngSrc]': 'ngSrc()',
    '[attr.alt]': 'alt()',
    '[attr.value]': 'value()',
    '[attr.min]': 'min()',
    '[attr.max]': 'max()',
    '[attr.indeterminate]': 'indeterminate()',
    '[attr.active]': 'active()',
    '[attr.spinning]': 'spinning()',
  },
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
}

/**
 * Typed directive for native text inputs, switches, checkboxes, sliders, dropdowns, and spin buttons.
 */
@Directive({
  selector:
    'ng-entry,gtk-entry,ng-search-entry,gtk-search-entry,ng-password-entry,gtk-password-entry,ng-text-view,gtk-text-view,ng-switch,gtk-switch,ng-checkbox,ng-check,ng-check-button,gtk-check-button,ng-slider,ng-scale,gtk-scale,ng-spin-button,gtk-spin-button,ng-select,ng-dropdown,gtk-dropdown,gtk-drop-down,adw-entry-row,adw-password-entry-row,adw-switch-row,adw-combo-row,adw-spin-row',
  host: {
    '[attr.placeholder]': 'placeholder()',
    '[attr.value]': 'value()',
    '[attr.text]': 'text()',
    '[attr.active]': 'active()',
    '[attr.checked]': 'checked()',
    '[attr.label]': 'label()',
    '[attr.options]': 'options()',
    '[attr.selected]': 'selected()',
    '[attr.editable]': 'editable()',
    '[attr.monospace]': 'monospace()',
    '[attr.show-peek-icon]': 'showPeekIcon()',
    '[attr.peek]': 'peek()',
  },
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
}

/**
 * Typed directive for native virtualized lists and tables (`<ng-list>`, `<ng-table>`).
 */
@Directive({
  selector: 'ng-list,ng-table,gtk-list-box,gtk-list-view',
  host: {
    '[attr.selection-mode]': 'selectionMode() ?? selectionModeAttr()',
    '[attr.show-separators]': 'showSeparators() ?? showSeparatorsAttr()',
    '[attr.separators]': 'separators()',
  },
})
export class NativeListDirective {
  readonly selectionModeAttr = input<NativeAttrInput>(undefined, { alias: 'selection-mode' });
  readonly selectionMode = input<NativeAttrInput>();
  readonly showSeparatorsAttr = input<NativeBoolInput>(undefined, { alias: 'show-separators' });
  readonly showSeparators = input<NativeBoolInput>();
  readonly separators = input<NativeBoolInput>();
}

/**
 * Typed directive for native anchored popovers and context menus (`<ng-popover>`, `<ng-menu>`).
 */
@Directive({
  selector: 'ng-popover,gtk-popover,ng-menu',
  host: {
    '[attr.open]': 'open()',
    '[attr.position]': 'position()',
    '[attr.placement]': 'placement()',
    '[attr.autohide]': 'autohide()',
    '[attr.has-arrow]': 'hasArrow() ?? hasArrowAttr()',
    '[attr.anchor]': 'anchor()',
  },
})
export class NativePopoverDirective {
  readonly open = input<NativeBoolInput>();
  readonly position = input<NativeAttrInput>();
  readonly placement = input<NativeAttrInput>();
  readonly autohide = input<NativeBoolInput>();
  readonly hasArrowAttr = input<NativeBoolInput>(undefined, { alias: 'has-arrow' });
  readonly hasArrow = input<NativeBoolInput>();
  readonly anchor = input<NativeAttrInput>();
}

/**
 * Typed directive for native numeric range & stepper inputs (`<ng-slider>`, `<ng-spin-button>`),
 * isolating `min`/`max`/`step` away from boolean toggles for Signal Forms type safety.
 */
@Directive({
  selector: 'ng-slider,ng-scale,gtk-scale,ng-spin-button,gtk-spin-button,adw-spin-row',
  host: {
    '[attr.min]': 'min()',
    '[attr.max]': 'max()',
    '[attr.step]': 'step()',
  },
})
export class NativeNumericRangeDirective {
  readonly min = input<NativeNumberInput>();
  readonly max = input<NativeNumberInput>();
  readonly step = input<NativeNumberInput>();
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

import {
  Directive,
  effect,
  ElementRef,
  inject,
  input,
  InputSignal,
  model,
  OnDestroy,
  output,
  Renderer2,
  Signal,
} from '@angular/core';
import {
  FormCheckboxControl,
  FormField,
  FormValueControl,
  type ValidationError,
} from '@angular/forms/signals';
import { NativeElementNode } from '@ng-desktop/platform';
import { SELECT_NUMBER_TAGS } from './value-accessors';

interface NativeWidgetEvent {
  type?: string;
  targetId?: string;
  value?: string | number | boolean;
  checked?: boolean;
  detail?: unknown;
}

interface NativeSignalUiStateSignals {
  readonly disabled: Signal<boolean>;
  readonly readonly: Signal<boolean>;
  readonly hidden: Signal<boolean>;
  readonly required: Signal<boolean>;
  readonly invalid: Signal<boolean>;
  readonly touched: Signal<boolean>;
  readonly dirty: Signal<boolean>;
  readonly name: Signal<string>;
}

function syncBooleanStateAttr(
  renderer: Renderer2,
  el: NativeElementNode,
  attrName: string,
  active: boolean,
  activeValue: string,
  inactiveValue: string,
  stateFlag: { applied: boolean },
): void {
  const current = el.attributes.get(attrName);
  if (active) {
    stateFlag.applied = true;
    if (current !== activeValue) {
      renderer.setAttribute(el, attrName, activeValue);
    }
    return;
  }
  if ((stateFlag.applied || current === activeValue) && current !== inactiveValue) {
    renderer.setAttribute(el, attrName, inactiveValue);
  }
}

function syncStateClass(
  renderer: Renderer2,
  el: NativeElementNode,
  className: string,
  active: boolean,
): void {
  if (active && !el.classes.has(className)) {
    renderer.addClass(el, className);
  } else if (!active && el.classes.has(className)) {
    renderer.removeClass(el, className);
  }
}

/**
 * Attaches reactive `effect()` synchronizers projecting Signal Forms `FormUiControl` status
 * signals onto native widget attributes, ARIA flags, and CSS classes without redundant commands.
 */
function bindNativeSignalUiEffects(
  renderer: Renderer2,
  el: NativeElementNode,
  ui: NativeSignalUiStateSignals,
): void {
  const disabledFlag = { applied: false };
  const readonlyFlag = { applied: false };
  const hiddenFlag = { applied: false };
  const requiredFlag = { applied: false };
  const invalidFlag = { applied: false };

  effect(() => {
    syncBooleanStateAttr(renderer, el, 'sensitive', ui.disabled(), 'false', 'true', disabledFlag);
    syncBooleanStateAttr(renderer, el, 'aria-disabled', ui.disabled(), 'true', 'false', {
      applied: disabledFlag.applied,
    });
  });

  effect(() => {
    const isReadonly = ui.readonly();
    syncBooleanStateAttr(renderer, el, 'editable', isReadonly, 'false', 'true', readonlyFlag);
    syncBooleanStateAttr(renderer, el, 'aria-readonly', isReadonly, 'true', 'false', {
      applied: readonlyFlag.applied,
    });
  });

  effect(() => {
    syncBooleanStateAttr(renderer, el, 'visible', ui.hidden(), 'false', 'true', hiddenFlag);
  });

  effect(() => {
    syncBooleanStateAttr(
      renderer,
      el,
      'aria-required',
      ui.required(),
      'true',
      'false',
      requiredFlag,
    );
  });

  effect(() => {
    const isInvalid = ui.invalid();
    syncStateClass(renderer, el, 'ng-invalid', isInvalid);
    syncBooleanStateAttr(renderer, el, 'aria-invalid', isInvalid, 'true', 'false', invalidFlag);
  });

  effect(() => {
    syncStateClass(renderer, el, 'ng-touched', ui.touched());
  });

  effect(() => {
    syncStateClass(renderer, el, 'ng-dirty', ui.dirty());
  });

  effect(() => {
    const fieldName = ui.name();
    if (fieldName && el.attributes.get('name') !== fieldName) {
      renderer.setAttribute(el, 'name', fieldName);
    }
  });
}

function syncDefinedNumericAttr(
  renderer: Renderer2,
  el: NativeElementNode,
  attrName: string,
  val: number | undefined,
): void {
  if (val === undefined || !Number.isFinite(val)) {
    return;
  }
  const next = String(val);
  if (el.attributes.get(attrName) !== next) {
    renderer.setAttribute(el, attrName, next);
  }
}

/**
 * First-class Angular Signal Forms `FormValueControl<string>` binding `[formField]` onto native
 * text widgets (`<ng-entry>`, `<ng-search-entry>`, `<ng-password-entry>`, `<ng-text-view>`, `<input>`, `<textarea>`).
 */
@Directive({
  selector:
    'ng-entry[formField],gtk-entry[formField],ng-search-entry[formField],gtk-search-entry[formField],ng-password-entry[formField],gtk-password-entry[formField],ng-text-view[formField],gtk-text-view[formField],adw-entry-row[formField],adw-password-entry-row[formField],input[formField]:not([type=checkbox]):not([type=radio]):not([type=range]):not([type=number]),textarea[formField]',
  standalone: true,
})
export class NativeTextFieldControl implements FormValueControl<string>, OnDestroy {
  private readonly renderer = inject(Renderer2);
  private readonly elementRef = inject<ElementRef<NativeElementNode>>(ElementRef);
  private readonly unlistenFns: Array<() => void> = [];

  readonly value = model<string>('');
  readonly disabled = input<boolean>(false);
  readonly readonly = input<boolean>(false);
  readonly hidden = input<boolean>(false);
  readonly required = input<boolean>(false);
  readonly invalid = input<boolean>(false);
  readonly touched = input<boolean>(false);
  readonly dirty = input<boolean>(false);
  readonly name = input<string>('');
  readonly minLength: InputSignal<number | undefined> = input<number | undefined>(undefined);
  readonly maxLength: InputSignal<number | undefined> = input<number | undefined>(undefined);
  readonly errors = input<readonly ValidationError.WithOptionalFieldTree[]>([]);
  readonly touch = output<void>();

  constructor() {
    const el = this.elementRef.nativeElement;

    bindNativeSignalUiEffects(this.renderer, el, this);

    effect(() => {
      const normalized = this.value() ?? '';
      if (el.attributes.get('value') !== normalized || el.textContent !== normalized) {
        this.renderer.setValue(el, normalized);
        this.renderer.setAttribute(el, 'value', normalized);
      }
    });

    effect(() => {
      syncDefinedNumericAttr(this.renderer, el, 'max-length', this.maxLength());
    });

    this.unlistenFns.push(
      this.renderer.listen(el, 'input', (event: unknown) => {
        const payload = event as NativeWidgetEvent | undefined;
        const nextText =
          typeof payload?.value === 'string' ? payload.value : String(payload?.value ?? '');
        el.textContent = nextText;
        el.attributes.set('value', nextText);
        this.value.set(nextText);
      }),
      this.renderer.listen(el, 'blur', () => {
        this.touch.emit();
      }),
    );
  }

  focus(): void {
    this.elementRef.nativeElement.focus?.();
  }

  ngOnDestroy(): void {
    for (const unlisten of this.unlistenFns) {
      unlisten();
    }
  }
}

/**
 * First-class Angular Signal Forms `FormCheckboxControl` binding `[formField]` onto native boolean
 * widgets (`<ng-switch>`, `<ng-checkbox>`, `<ng-check>`, `<ng-check-button>`, `<gtk-switch>`, `<gtk-check-button>`).
 */
@Directive({
  selector:
    'ng-switch[formField],gtk-switch[formField],ng-checkbox[formField],ng-check[formField],ng-check-button[formField],gtk-check-button[formField],adw-switch-row[formField]',
  standalone: true,
})
export class NativeBooleanFieldControl implements FormCheckboxControl, OnDestroy {
  private readonly renderer = inject(Renderer2);
  private readonly elementRef = inject<ElementRef<NativeElementNode>>(ElementRef);
  private readonly unlistenFns: Array<() => void> = [];

  readonly checked = model<boolean>(false);
  readonly disabled = input<boolean>(false);
  readonly readonly = input<boolean>(false);
  readonly hidden = input<boolean>(false);
  readonly required = input<boolean>(false);
  readonly invalid = input<boolean>(false);
  readonly touched = input<boolean>(false);
  readonly dirty = input<boolean>(false);
  readonly name = input<string>('');
  readonly errors = input<readonly ValidationError.WithOptionalFieldTree[]>([]);
  readonly touch = output<void>();

  constructor() {
    const el = this.elementRef.nativeElement;

    bindNativeSignalUiEffects(this.renderer, el, this);

    effect(() => {
      const nextBool = Boolean(this.checked());
      const nextAttr = nextBool ? 'true' : 'false';
      if (el.attributes.get('active') !== nextAttr) {
        this.renderer.setAttribute(el, 'active', nextAttr);
      }
      el.checked = nextBool;
    });

    this.unlistenFns.push(
      this.renderer.listen(el, 'toggled', (event: unknown) => {
        const payload = event as NativeWidgetEvent | undefined;
        const active =
          typeof payload?.checked === 'boolean' ? payload.checked : Boolean(payload?.value);
        el.checked = active;
        this.checked.set(active);
        this.touch.emit();
      }),
      this.renderer.listen(el, 'blur', () => {
        this.touch.emit();
      }),
    );
  }

  focus(): void {
    this.elementRef.nativeElement.focus?.();
  }

  ngOnDestroy(): void {
    for (const unlisten of this.unlistenFns) {
      unlisten();
    }
  }
}

/**
 * First-class Angular Signal Forms `FormValueControl<number>` binding `[formField]` onto native
 * numeric/selection widgets (`<ng-slider>`, `<ng-spin-button>`, `<ng-select>`, `<select>`, `<input type="number|range">`).
 */
@Directive({
  selector:
    'ng-slider[formField],ng-scale[formField],gtk-scale[formField],ng-select[formField],ng-dropdown[formField],gtk-dropdown[formField],gtk-drop-down[formField],ng-spin-button[formField],gtk-spin-button[formField],adw-combo-row[formField],adw-spin-row[formField],select[formField],input[type=number][formField],input[type=range][formField]',
  standalone: true,
})
export class NativeNumberFieldControl implements FormValueControl<number>, OnDestroy {
  private readonly renderer = inject(Renderer2);
  private readonly elementRef = inject<ElementRef<NativeElementNode>>(ElementRef);
  private readonly unlistenFns: Array<() => void> = [];

  readonly value = model<number>(0);
  readonly disabled = input<boolean>(false);
  readonly readonly = input<boolean>(false);
  readonly hidden = input<boolean>(false);
  readonly required = input<boolean>(false);
  readonly invalid = input<boolean>(false);
  readonly touched = input<boolean>(false);
  readonly dirty = input<boolean>(false);
  readonly name = input<string>('');
  readonly min = input<number | undefined>(undefined);
  readonly max = input<number | undefined>(undefined);
  readonly errors = input<readonly ValidationError.WithOptionalFieldTree[]>([]);
  readonly touch = output<void>();

  constructor() {
    const el = this.elementRef.nativeElement;
    const attrKey = SELECT_NUMBER_TAGS.has(el.tag) || el.tag === 'select' ? 'selected' : 'value';

    bindNativeSignalUiEffects(this.renderer, el, this);

    effect(() => {
      const nextStr = String(this.value() ?? 0);
      if (el.attributes.get(attrKey) !== nextStr) {
        this.renderer.setAttribute(el, attrKey, nextStr);
      }
    });

    effect(() => {
      syncDefinedNumericAttr(this.renderer, el, 'min', this.min());
    });

    effect(() => {
      syncDefinedNumericAttr(this.renderer, el, 'max', this.max());
    });

    this.unlistenFns.push(
      this.renderer.listen(el, 'value-changed', (event: unknown) => {
        const payload = event as NativeWidgetEvent | undefined;
        const numValue = Number(payload?.value ?? 0);
        el.attributes.set(attrKey, String(numValue));
        this.value.set(numValue);
        this.touch.emit();
      }),
      this.renderer.listen(el, 'blur', () => {
        this.touch.emit();
      }),
    );
  }

  focus(): void {
    this.elementRef.nativeElement.focus?.();
  }

  ngOnDestroy(): void {
    for (const unlisten of this.unlistenFns) {
      unlisten();
    }
  }
}

/**
 * Convenience standalone directive bundle enabling Angular Signal Forms (`[formField]`) on all
 * native text, boolean, numeric, and mapped standard HTML widgets.
 */
export const NATIVE_SIGNAL_FORM_DIRECTIVES = [
  FormField,
  NativeTextFieldControl,
  NativeBooleanFieldControl,
  NativeNumberFieldControl,
] as const;

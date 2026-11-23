import { Directive, ElementRef, forwardRef, inject, OnDestroy, Renderer2 } from '@angular/core';
import { ControlValueAccessor, NG_VALUE_ACCESSOR } from '@angular/forms';
import { NativeElementNode } from '@ng-desktop/platform';

interface NativeWidgetEvent {
  type?: string;
  targetId?: string;
  value?: string | number | boolean;
  checked?: boolean;
  detail?: unknown;
}

export const SELECT_NUMBER_TAGS = new Set([
  'ng-select',
  'ng-dropdown',
  'gtk-dropdown',
  'gtk-drop-down',
  'adw-combo-row',
]);

/**
 * Angular `ControlValueAccessor` for `<ng-entry>` / `<gtk-entry>` / `<adw-entry-row>` text input widgets.
 * Bridges `GtkEditable::changed` (`input`) native signals and `gtk_editable_set_text`.
 */
@Directive({
  selector:
    'ng-entry[formControlName],ng-entry[formControl],ng-entry[ngModel],gtk-entry[formControlName],gtk-entry[formControl],gtk-entry[ngModel],ng-search-entry[formControlName],ng-search-entry[formControl],ng-search-entry[ngModel],gtk-search-entry[formControlName],gtk-search-entry[formControl],gtk-search-entry[ngModel],ng-password-entry[formControlName],ng-password-entry[formControl],ng-password-entry[ngModel],gtk-password-entry[formControlName],gtk-password-entry[formControl],gtk-password-entry[ngModel],ng-text-view[formControlName],ng-text-view[formControl],ng-text-view[ngModel],gtk-text-view[formControlName],gtk-text-view[formControl],gtk-text-view[ngModel],adw-entry-row[formControlName],adw-entry-row[formControl],adw-entry-row[ngModel],adw-password-entry-row[formControlName],adw-password-entry-row[formControl],adw-password-entry-row[ngModel]',
  standalone: true,
  providers: [
    {
      provide: NG_VALUE_ACCESSOR,
      useExisting: forwardRef(() => NativeTextValueAccessor),
      multi: true,
    },
  ],
})
export class NativeTextValueAccessor implements ControlValueAccessor, OnDestroy {
  private readonly renderer = inject(Renderer2);
  private readonly elementRef = inject<ElementRef<NativeElementNode>>(ElementRef);
  private onChange: (value: string) => void = () => {};
  private onTouched: () => void = () => {};
  private readonly unlistenFns: Array<() => void> = [];

  constructor() {
    const el = this.elementRef.nativeElement;
    this.unlistenFns.push(
      this.renderer.listen(el, 'input', (event: unknown) => {
        const payload = event as NativeWidgetEvent | undefined;
        const nextText =
          typeof payload?.value === 'string' ? payload.value : String(payload?.value ?? '');
        el.textContent = nextText;
        el.attributes.set('value', nextText);
        this.onChange(nextText);
      }),
      this.renderer.listen(el, 'blur', () => {
        this.onTouched();
      }),
    );
  }

  writeValue(value: unknown): void {
    const normalized = value === null || value === undefined ? '' : String(value);
    const el = this.elementRef.nativeElement;
    this.renderer.setValue(el, normalized);
    this.renderer.setAttribute(el, 'value', normalized);
  }

  registerOnChange(fn: (value: string) => void): void {
    this.onChange = fn;
  }

  registerOnTouched(fn: () => void): void {
    this.onTouched = fn;
  }

  setDisabledState(isDisabled: boolean): void {
    this.renderer.setAttribute(
      this.elementRef.nativeElement,
      'sensitive',
      isDisabled ? 'false' : 'true',
    );
  }

  ngOnDestroy(): void {
    for (const unlisten of this.unlistenFns) {
      unlisten();
    }
  }
}

/**
 * Angular `ControlValueAccessor` for boolean native widgets:
 * `<ng-switch>`, `<ng-checkbox>`, `<ng-check>`, `<gtk-switch>`, `<gtk-check-button>`, `<adw-switch-row>`.
 */
@Directive({
  selector:
    'ng-switch[formControlName],ng-switch[formControl],ng-switch[ngModel],ng-checkbox[formControlName],ng-checkbox[formControl],ng-checkbox[ngModel],ng-check[formControlName],ng-check[formControl],ng-check[ngModel],ng-check-button[formControlName],ng-check-button[formControl],ng-check-button[ngModel],gtk-switch[formControlName],gtk-switch[formControl],gtk-switch[ngModel],gtk-check-button[formControlName],gtk-check-button[formControl],gtk-check-button[ngModel],adw-switch-row[formControlName],adw-switch-row[formControl],adw-switch-row[ngModel]',
  standalone: true,
  providers: [
    {
      provide: NG_VALUE_ACCESSOR,
      useExisting: forwardRef(() => NativeBooleanValueAccessor),
      multi: true,
    },
  ],
})
export class NativeBooleanValueAccessor implements ControlValueAccessor, OnDestroy {
  private readonly renderer = inject(Renderer2);
  private readonly elementRef = inject<ElementRef<NativeElementNode>>(ElementRef);
  private onChange: (value: boolean) => void = () => {};
  private onTouched: () => void = () => {};
  private readonly unlistenFns: Array<() => void> = [];

  constructor() {
    const el = this.elementRef.nativeElement;
    this.unlistenFns.push(
      this.renderer.listen(el, 'toggled', (event: unknown) => {
        const payload = event as NativeWidgetEvent | undefined;
        const active =
          typeof payload?.checked === 'boolean' ? payload.checked : Boolean(payload?.value);
        el.attributes.set('active', active ? 'true' : 'false');
        this.onChange(active);
        this.onTouched();
      }),
    );
  }

  writeValue(value: unknown): void {
    const active = Boolean(value);
    this.renderer.setAttribute(this.elementRef.nativeElement, 'active', active ? 'true' : 'false');
  }

  registerOnChange(fn: (value: boolean) => void): void {
    this.onChange = fn;
  }

  registerOnTouched(fn: () => void): void {
    this.onTouched = fn;
  }

  setDisabledState(isDisabled: boolean): void {
    this.renderer.setAttribute(
      this.elementRef.nativeElement,
      'sensitive',
      isDisabled ? 'false' : 'true',
    );
  }

  ngOnDestroy(): void {
    for (const unlisten of this.unlistenFns) {
      unlisten();
    }
  }
}

/**
 * Angular `ControlValueAccessor` for numeric/index native widgets:
 * `<ng-slider>`, `<ng-scale>`, `<gtk-scale>`, `<ng-select>`, `<ng-dropdown>`, `<gtk-dropdown>`,
 * `<gtk-drop-down>`, `<ng-spin-button>`, `<gtk-spin-button>`, `<adw-combo-row>`, `<adw-spin-row>`.
 */
@Directive({
  selector:
    'ng-slider[formControlName],ng-slider[formControl],ng-slider[ngModel],ng-scale[formControlName],ng-scale[formControl],ng-scale[ngModel],gtk-scale[formControlName],gtk-scale[formControl],gtk-scale[ngModel],ng-select[formControlName],ng-select[formControl],ng-select[ngModel],ng-dropdown[formControlName],ng-dropdown[formControl],ng-dropdown[ngModel],gtk-dropdown[formControlName],gtk-dropdown[formControl],gtk-dropdown[ngModel],gtk-drop-down[formControlName],gtk-drop-down[formControl],gtk-drop-down[ngModel],ng-spin-button[formControlName],ng-spin-button[formControl],ng-spin-button[ngModel],gtk-spin-button[formControlName],gtk-spin-button[formControl],gtk-spin-button[ngModel],adw-combo-row[formControlName],adw-combo-row[formControl],adw-combo-row[ngModel],adw-spin-row[formControlName],adw-spin-row[formControl],adw-spin-row[ngModel]',
  standalone: true,
  providers: [
    {
      provide: NG_VALUE_ACCESSOR,
      useExisting: forwardRef(() => NativeNumberValueAccessor),
      multi: true,
    },
  ],
})
export class NativeNumberValueAccessor implements ControlValueAccessor, OnDestroy {
  private readonly renderer = inject(Renderer2);
  private readonly elementRef = inject<ElementRef<NativeElementNode>>(ElementRef);
  private onChange: (value: number) => void = () => {};
  private onTouched: () => void = () => {};
  private readonly unlistenFns: Array<() => void> = [];

  constructor() {
    const el = this.elementRef.nativeElement;
    this.unlistenFns.push(
      this.renderer.listen(el, 'value-changed', (event: unknown) => {
        const payload = event as NativeWidgetEvent | undefined;
        const numValue = Number(payload?.value ?? 0);
        const attrKey = SELECT_NUMBER_TAGS.has(el.tag) ? 'selected' : 'value';
        el.attributes.set(attrKey, String(numValue));
        this.onChange(numValue);
        this.onTouched();
      }),
    );
  }

  writeValue(value: unknown): void {
    const numValue = Number(value ?? 0);
    const el = this.elementRef.nativeElement;
    const attrKey = SELECT_NUMBER_TAGS.has(el.tag) ? 'selected' : 'value';
    this.renderer.setAttribute(el, attrKey, String(numValue));
  }

  registerOnChange(fn: (value: number) => void): void {
    this.onChange = fn;
  }

  registerOnTouched(fn: () => void): void {
    this.onTouched = fn;
  }

  setDisabledState(isDisabled: boolean): void {
    this.renderer.setAttribute(
      this.elementRef.nativeElement,
      'sensitive',
      isDisabled ? 'false' : 'true',
    );
  }

  ngOnDestroy(): void {
    for (const unlisten of this.unlistenFns) {
      unlisten();
    }
  }
}

export const NATIVE_FORM_DIRECTIVES = [
  NativeTextValueAccessor,
  NativeBooleanValueAccessor,
  NativeNumberValueAccessor,
] as const;

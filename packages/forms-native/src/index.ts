import { FormField } from '@angular/forms/signals';
import {
  NativeBooleanFieldControl,
  NativeNumberFieldControl,
  NativeTextFieldControl,
} from './signal-forms';
import {
  NativeBooleanValueAccessor,
  NativeNumberValueAccessor,
  NativeTextValueAccessor,
} from './value-accessors';

export {
  NativeBooleanValueAccessor,
  NativeNumberValueAccessor,
  NativeTextValueAccessor,
} from './value-accessors';

export {
  NATIVE_SIGNAL_FORM_DIRECTIVES,
  NativeBooleanFieldControl,
  NativeNumberFieldControl,
  NativeTextFieldControl,
} from './signal-forms';

export {
  disabled,
  email,
  form,
  FormField,
  FormRoot,
  hidden,
  max,
  maxLength,
  min,
  minLength,
  pattern,
  provideSignalFormsConfig,
  readonly,
  required,
  schema,
  submit,
  validate,
  type FieldState,
  type FieldTree,
  type FormCheckboxControl,
  type FormUiControl,
  type FormValueControl,
  type ValidationError,
} from '@angular/forms/signals';

/**
 * Unified standalone directive bundle supporting both Reactive/Template forms (`[formControl]`,
 * `[formControlName]`, `[(ngModel)]`) and Angular Signal Forms (`[formField]`).
 */
export const NATIVE_FORM_DIRECTIVES = [
  NativeTextValueAccessor,
  NativeBooleanValueAccessor,
  NativeNumberValueAccessor,
  FormField,
  NativeTextFieldControl,
  NativeBooleanFieldControl,
  NativeNumberFieldControl,
] as const;

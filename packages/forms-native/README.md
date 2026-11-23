# `@ng-desktop/forms`

First-class Angular Signal Forms (`[formField]` via `FormValueControl` / `FormCheckboxControl`) and `ControlValueAccessor` directives (`ReactiveFormsModule`, `FormsModule`) wired directly to native GTK4 / WinUI 3 / AppKit desktop widgets.

## Install

```bash
pnpm add @ng-desktop/forms @ng-desktop/platform
```

Peer dependencies: `@angular/core >= 22.0.0`, `@angular/forms >= 22.0.0`, `@ng-desktop/platform`.

## Quick Usage — Angular Signal Forms (`[formField]`)

```ts
import { Component, signal } from '@angular/core';
import { form, max, min, NATIVE_SIGNAL_FORM_DIRECTIVES, required } from '@ng-desktop/forms';
import { NATIVE_UI_DIRECTIVES } from '@ng-desktop/platform';

@Component({
  selector: 'app-preferences',
  standalone: true,
  imports: [NATIVE_UI_DIRECTIVES, NATIVE_SIGNAL_FORM_DIRECTIVES],
  template: `
    <ng-box orientation="vertical" spacing="10px">
      <ng-entry placeholder="Display name" [formField]="settingsForm.name" />
      <ng-switch [formField]="settingsForm.enabled" />
      <ng-slider [formField]="settingsForm.volume" />
    </ng-box>
  `,
})
export class PreferencesComponent {
  readonly model = signal({ name: 'Ada', enabled: true, volume: 75 });

  readonly settingsForm = form(this.model, (path) => {
    required(path.name);
    min(path.volume, 0);
    max(path.volume, 100);
  });
}
```

Standard HTML form elements (`<input [formField]>`, `<textarea [formField]>`, `<select [formField]>`) also work out of the box without `CUSTOM_ELEMENTS_SCHEMA`. Schema states (`disabled`, `readonly`, `hidden`, `min`/`max`, `maxLength`, `invalid`, `touched`, `dirty`, `focusBoundControl()`) propagate automatically to native attributes and CSS classes (`ng-invalid`, `ng-touched`, `ng-dirty`).

## Quick Usage — Reactive & Template Forms

`NATIVE_FORM_DIRECTIVES` includes both the Signal Forms controls and the classic `ControlValueAccessor` directives:

```ts
import { Component } from '@angular/core';
import { FormControl, FormGroup, ReactiveFormsModule } from '@angular/forms';
import { NATIVE_FORM_DIRECTIVES } from '@ng-desktop/forms';
import { NATIVE_UI_DIRECTIVES } from '@ng-desktop/platform';

@Component({
  selector: 'app-legacy-preferences',
  standalone: true,
  imports: [ReactiveFormsModule, NATIVE_FORM_DIRECTIVES, NATIVE_UI_DIRECTIVES],
  template: `
    <ng-box [formGroup]="form" orientation="vertical" spacing="10px">
      <ng-entry placeholder="Display name" formControlName="name" />
      <ng-switch formControlName="enabled" />
      <ng-slider min="0" max="100" formControlName="volume" />
    </ng-box>
  `,
})
export class LegacyPreferencesComponent {
  readonly form = new FormGroup({
    name: new FormControl('', { nonNullable: true }),
    enabled: new FormControl(true, { nonNullable: true }),
    volume: new FormControl(75, { nonNullable: true }),
  });
}
```

## Key Exports

- **Bundles:** `NATIVE_SIGNAL_FORM_DIRECTIVES` (Signal Forms `[formField]`), `NATIVE_FORM_DIRECTIVES` (all Signal Forms + CVA directives)
- **Signal Forms Controls:** `NativeTextFieldControl` (`FormValueControl<string>`), `NativeBooleanFieldControl` (`FormCheckboxControl`), `NativeNumberFieldControl` (`FormValueControl<number>`)
- **Re-exported Signal Forms APIs:** `form`, `schema`, `FormField`, `FormRoot`, `required`, `min`, `max`, `minLength`, `maxLength`, `pattern`, `email`, `disabled`, `readonly`, `hidden`, `debounce`, `submit`, `validate`, `provideSignalFormsConfig`
- **Classic CVAs:** `NativeTextValueAccessor`, `NativeBooleanValueAccessor`, `NativeNumberValueAccessor`

## Documentation

- [Getting Started with External Angular 22+ & Nx Apps](../../docs/GETTING_STARTED.md)
- [Architectural Roadmap & OS Maturity Matrix](../../ROADMAP.md)

# `@ng-desktop/forms`

Angular `ControlValueAccessor` directives bridging `ReactiveFormsModule` (`FormControl`, `FormGroup`, `Validators`), `FormsModule` (`[(ngModel)]`), and Signal Forms directly to native desktop widgets.

## Install

```bash
pnpm add @ng-desktop/forms @ng-desktop/platform
```

Peer dependencies: `@angular/core >= 22.0.0`, `@angular/forms >= 22.0.0`, `@ng-desktop/platform`.

## Quick Usage

```ts
import { Component } from '@angular/core';
import { FormControl, FormGroup, ReactiveFormsModule } from '@angular/forms';
import { NATIVE_FORM_DIRECTIVES } from '@ng-desktop/forms';
import { NATIVE_UI_DIRECTIVES } from '@ng-desktop/platform';

@Component({
  selector: 'app-preferences',
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
export class PreferencesComponent {
  readonly form = new FormGroup({
    name: new FormControl('', { nonNullable: true }),
    enabled: new FormControl(true, { nonNullable: true }),
    volume: new FormControl(75, { nonNullable: true }),
  });
}
```

## Key Exports

- `NATIVE_FORM_DIRECTIVES`
- `NativeTextValueAccessor` (`<ng-entry>`, `<ng-search-entry>`, `<ng-password-entry>`, `<ng-text-view>`)
- `NativeBooleanValueAccessor` (`<ng-switch>`, `<ng-checkbox>`)
- `NativeNumberValueAccessor` (`<ng-slider>`, `<ng-spin-button>`, `<ng-select>`)

## Documentation

- [Getting Started with External Angular 22+ & Nx Apps](../../docs/GETTING_STARTED.md)
- [Architectural Roadmap & OS Maturity Matrix](../../ROADMAP.md)

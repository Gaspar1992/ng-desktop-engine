import { Component, input } from '@angular/core';
import { NATIVE_UI_DIRECTIVES } from '@ng-desktop/platform';

@Component({
  selector: 'app-stat-meter',
  imports: [NATIVE_UI_DIRECTIVES],
  templateUrl: './stat-meter.html',
  styleUrl: './stat-meter.css',
})
export class StatMeter {
  readonly label = input.required<string>();
  readonly value = input.required<number>();
}

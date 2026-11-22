import { Component, CUSTOM_ELEMENTS_SCHEMA, input } from '@angular/core';

@Component({
  selector: 'app-stat-meter',
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  templateUrl: './stat-meter.html',
  styleUrl: './stat-meter.css',
})
export class StatMeter {
  readonly label = input.required<string>();
  readonly value = input.required<number>();
}

import { Component, input } from '@angular/core';
import { NATIVE_UI_DIRECTIVES } from '@ng-desktop/platform';

@Component({
  selector: 'app-type-badge',
  imports: [NATIVE_UI_DIRECTIVES],
  templateUrl: './type-badge.html',
  styleUrl: './type-badge.css',
})
export class TypeBadge {
  readonly type = input<string>('');
  readonly badgeClass = input<string>('badge-normal');
}

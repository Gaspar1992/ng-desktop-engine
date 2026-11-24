import { ChangeDetectionStrategy, Component, input } from '@angular/core';
import { NATIVE_UI_DIRECTIVES } from '@ng-desktop/platform';

@Component({
  selector: 'app-pokemon-card',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [NATIVE_UI_DIRECTIVES],
  templateUrl: './pokemon-card.html',
  styleUrl: './pokemon-card.css',
})
export class PokemonCard {
  readonly name = input<string>('');
  readonly number = input<string>('');
  readonly sprite = input<string>('');
}

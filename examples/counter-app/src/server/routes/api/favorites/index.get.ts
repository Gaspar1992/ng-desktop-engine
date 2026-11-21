import { defineNativeEventHandler } from '@ng-desktop/server';
import { ensurePokedexSchema, type FavoritePokemonRow } from '../../../pokedex-store';

export default defineNativeEventHandler((event) => {
  ensurePokedexSchema(event.context.db);
  const items = event.context.db.query<FavoritePokemonRow>(
    'SELECT id, name, note FROM favorites ORDER BY id ASC;',
  );
  return { items };
});

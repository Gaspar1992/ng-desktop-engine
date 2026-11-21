import { createError, defineNativeEventHandler, getRouterParam } from '@ng-desktop/server';
import { ensurePokedexSchema } from '../../../pokedex-store';

export default defineNativeEventHandler((event) => {
  ensurePokedexSchema(event.context.db);
  const rawId = getRouterParam(event, 'id');
  const id = Number.parseInt(String(rawId ?? ''), 10);
  if (!Number.isInteger(id) || id < 1 || id > 1025) {
    throw createError({
      statusCode: 400,
      message: `ID inválido '${String(rawId ?? '')}'`,
    });
  }

  event.context.db.execute('DELETE FROM favorites WHERE id = ?;', [id]);
  return { removed: true, id };
});

import {
  createError,
  defineNativeEventHandler,
  getRouterParam,
  readBody,
} from '@ng-desktop/server';
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

  const body = readBody<{ note?: string }>(event);
  const note = String(body.note ?? '');
  event.context.db.execute('INSERT OR REPLACE INTO field_notes (id, note) VALUES (?, ?);', [
    id,
    note,
  ]);
  return { saved: true, id, note };
});

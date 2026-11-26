import {
  createError,
  defineNativeEventHandler,
  readBody,
  setResponseStatus,
} from '@ng-desktop/server';
import { ensurePokedexSchema } from '../../../pokedex-store';

interface SaveFavoriteBody {
  id?: number | string;
  name?: string;
  note?: string;
}

export default defineNativeEventHandler((event) => {
  ensurePokedexSchema(event.context.db);
  const body = readBody<SaveFavoriteBody>(event) ?? {};
  const id = Number.parseInt(String(body.id ?? ''), 10);
  const name = String(body.name ?? '').trim();
  const note = String(body.note ?? '');

  if (!Number.isInteger(id) || id < 1 || id > 1025 || name.length === 0) {
    throw createError({
      statusCode: 400,
      message: 'Se requiere id (1..1025) y name para guardar favorito',
    });
  }

  event.context.db.execute('INSERT OR REPLACE INTO favorites (id, name, note) VALUES (?, ?, ?);', [
    id,
    name,
    note,
  ]);
  setResponseStatus(event, 201, 'Created');
  return {
    saved: true,
    item: { id, name, note },
  };
});

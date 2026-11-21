import { createError, defineNativeEventHandler, getRouterParam } from '@ng-desktop/server';
import {
  ensurePokedexSchema,
  normalizePokemonPayload,
  type PokemonDetailRecord,
} from '../../../pokedex-store';

export default defineNativeEventHandler(async (event) => {
  ensurePokedexSchema(event.context.db);
  const rawId = getRouterParam(event, 'id');
  const id = Number.parseInt(String(rawId ?? ''), 10);
  if (!Number.isInteger(id) || id < 1 || id > 1025) {
    throw createError({
      statusCode: 400,
      message: `ID de Pokémon inválido '${String(rawId ?? '')}'`,
    });
  }

  const favRows = event.context.db.query<{ id: number; note: string }>(
    'SELECT id, note FROM favorites WHERE id = ?;',
    [id],
  );
  const noteRows = event.context.db.query<{ note: string }>(
    'SELECT note FROM field_notes WHERE id = ?;',
    [id],
  );
  const isFavorite = favRows.length > 0;
  const note = noteRows[0]?.note ?? favRows[0]?.note ?? '';

  const cachedRows = event.context.db.query<{ payload: string }>(
    'SELECT payload FROM pokemon_cache WHERE id = ?;',
    [id],
  );
  const cachedPayload = cachedRows[0]?.payload;
  if (typeof cachedPayload === 'string' && cachedPayload.length > 2) {
    try {
      const pokemon = JSON.parse(cachedPayload) as PokemonDetailRecord;
      return { pokemon, isFavorite, note, cached: true };
    } catch {
      // Refresh corrupt entry
    }
  }

  const pokemonRes = await event.context.http.get(`https://pokeapi.co/api/v2/pokemon/${id}`);
  if (pokemonRes.status < 200 || pokemonRes.status >= 300) {
    throw createError({
      statusCode: pokemonRes.status >= 400 ? pokemonRes.status : 502,
      message: `No se pudo obtener el Pokémon #${id} (HTTP ${pokemonRes.status})`,
    });
  }

  let speciesData: Record<string, unknown> | null = null;
  try {
    const speciesRes = await event.context.http.get(
      `https://pokeapi.co/api/v2/pokemon-species/${id}`,
    );
    if (speciesRes.status >= 200 && speciesRes.status < 300 && speciesRes.body) {
      speciesData = JSON.parse(speciesRes.body) as Record<string, unknown>;
    }
  } catch {
    speciesData = null;
  }

  const rawPokemon = JSON.parse(pokemonRes.body) as Record<string, unknown>;
  const pokemon = normalizePokemonPayload(id, rawPokemon, speciesData);

  event.context.db.execute('INSERT OR REPLACE INTO pokemon_cache (id, payload) VALUES (?, ?);', [
    id,
    JSON.stringify(pokemon),
  ]);

  return { pokemon, isFavorite, note, cached: false };
});

import { defineNativeEventHandler, getQuery } from '@ng-desktop/server';
import {
  type CatalogListItem,
  ensurePokedexSchema,
  loadPokedexCatalog,
} from '../../../pokedex-store';

export default defineNativeEventHandler(async (event) => {
  ensurePokedexSchema(event.context.db);
  const query = getQuery(event);
  const needle = String(query['q'] ?? '').trim();
  const offset = Math.max(0, Number.parseInt(String(query['offset'] ?? '0'), 10) || 0);
  const limit = Math.max(
    1,
    Math.min(50, Number.parseInt(String(query['limit'] ?? '12'), 10) || 12),
  );
  const favoritesOnly = query['favoritesOnly'] === 'true' || query['favoritesOnly'] === '1';

  const favRows = event.context.db.query<{ id: number | string }>(
    'SELECT id FROM favorites ORDER BY id ASC;',
  );
  const favoriteIds = favRows.map((row) => Number(row.id)).filter((id) => Number.isInteger(id));
  const favoriteSet = new Set(favoriteIds);

  const catalog = await loadPokedexCatalog(event);
  let working: CatalogListItem[] = catalog;

  if (favoritesOnly) {
    working = working.filter((item) => favoriteSet.has(item.id));
  }

  if (needle.length > 0) {
    const byKey = new Map<string, CatalogListItem>();
    const searchStrings: string[] = [];
    for (const item of working) {
      const key = `${item.id} ${item.name} ${item.displayName}`;
      searchStrings.push(key);
      byKey.set(key, item);
    }
    const matches = event.context.compute.fuzzyFilter(searchStrings, needle, working.length);
    const filtered: CatalogListItem[] = [];
    for (const match of matches) {
      const item = byKey.get(match.value);
      if (item) {
        filtered.push(item);
      }
    }
    working = filtered;
  }

  const total = working.length;
  const slice = working.slice(offset, offset + limit).map((item) => ({
    ...item,
    isFavorite: favoriteSet.has(item.id),
  }));

  return {
    total,
    offset,
    limit,
    hasMore: offset + slice.length < total,
    items: slice,
    favoriteIds,
  };
});

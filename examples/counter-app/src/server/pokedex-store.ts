import type { NativeServerDbContext, NativeServerEvent } from '@ng-desktop/server';
import { createError } from '@ng-desktop/server';

export interface CatalogListItem {
  id: number;
  name: string;
  displayName: string;
  number: string;
  spriteUrl: string;
}

export interface PokemonTypeBadge {
  label: string;
  badgeClass: string;
}

export interface PokemonStatItem {
  key: string;
  label: string;
  value: number;
}

export interface PokemonDetailRecord {
  id: number;
  name: string;
  displayName: string;
  number: string;
  spriteUrl: string;
  genus: string;
  description: string;
  heightMeters: string;
  weightKg: string;
  ability: string;
  baseExperience: string;
  types: PokemonTypeBadge[];
  stats: PokemonStatItem[];
}

export interface FavoritePokemonRow {
  id: number;
  name: string;
  note: string;
}

interface RawNamedUrl {
  name?: string;
  url?: string;
}

interface RawPokemonApiResponse {
  id?: number;
  name?: string;
  height?: number;
  weight?: number;
  base_experience?: number;
  sprites?: {
    front_default?: string;
    other?: {
      'official-artwork'?: { front_default?: string };
    };
  };
  types?: Array<{ slot?: number; type?: RawNamedUrl }>;
  abilities?: Array<{ is_hidden?: boolean; ability?: RawNamedUrl }>;
  stats?: Array<{ base_stat?: number; stat?: RawNamedUrl }>;
}

interface RawSpeciesApiResponse {
  genera?: Array<{ genus?: string; language?: RawNamedUrl }>;
  flavor_text_entries?: Array<{ flavor_text?: string; language?: RawNamedUrl }>;
}

const TYPE_BADGE_MAP: Record<string, PokemonTypeBadge> = {
  grass: { label: 'PLANTA', badgeClass: 'badge-grass' },
  poison: { label: 'VENENO', badgeClass: 'badge-poison' },
  fire: { label: 'FUEGO', badgeClass: 'badge-fire' },
  water: { label: 'AGUA', badgeClass: 'badge-water' },
  electric: { label: 'ELÉCTRICO', badgeClass: 'badge-electric' },
  flying: { label: 'VOLADOR', badgeClass: 'badge-flying' },
  ghost: { label: 'FANTASMA', badgeClass: 'badge-ghost' },
};

const STAT_LABEL_MAP: Record<string, string> = {
  hp: 'PS',
  attack: 'Ataque',
  defense: 'Defensa',
  'special-attack': 'At. Esp.',
  'special-defense': 'Def. Esp.',
  speed: 'Velocidad',
};

let schemaReady = false;
let catalogMemoryCache: CatalogListItem[] | null = null;

export function resetMemoryCachesForTesting(): void {
  schemaReady = false;
  catalogMemoryCache = null;
}

export function ensurePokedexSchema(db: NativeServerDbContext): void {
  if (schemaReady) {
    return;
  }
  db.execute(
    'CREATE TABLE IF NOT EXISTS catalog_cache (cache_key TEXT PRIMARY KEY, payload TEXT NOT NULL);',
  );
  db.execute(
    'CREATE TABLE IF NOT EXISTS pokemon_cache (id INTEGER PRIMARY KEY, payload TEXT NOT NULL);',
  );
  db.execute(
    'CREATE TABLE IF NOT EXISTS favorites (id INTEGER PRIMARY KEY, name TEXT NOT NULL, note TEXT NOT NULL);',
  );
  db.execute(
    'CREATE TABLE IF NOT EXISTS field_notes (id INTEGER PRIMARY KEY, note TEXT NOT NULL);',
  );
  schemaReady = true;
}

export function formatPokemonNumber(id: number): string {
  return `N.º ${String(id).padStart(4, '0')}`;
}

export function formatDisplayName(rawName: string): string {
  const clean = rawName.trim();
  if (!clean) {
    return '';
  }
  return clean
    .split(/[-_\s]+/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1).toLowerCase())
    .join(' ');
}

export function spriteUrlForId(id: number): string {
  return `https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/official-artwork/${id}.png`;
}

export function parsePokemonIdFromUrl(url: string | undefined, fallbackId: number): number {
  if (url) {
    const match = url.match(/\/pokemon\/(\d+)\/?$/);
    if (match?.[1]) {
      const parsed = Number.parseInt(match[1], 10);
      if (Number.isInteger(parsed) && parsed > 0) {
        return parsed;
      }
    }
  }
  return fallbackId;
}

export async function loadPokedexCatalog(event: NativeServerEvent): Promise<CatalogListItem[]> {
  ensurePokedexSchema(event.context.db);
  if (catalogMemoryCache && catalogMemoryCache.length > 0) {
    return catalogMemoryCache;
  }

  const cachedRows = event.context.db.query<{ payload?: string }>(
    'SELECT payload FROM catalog_cache WHERE cache_key = ?;',
    ['kanto-151'],
  );
  const rawCached = cachedRows[0]?.payload;
  if (typeof rawCached === 'string' && rawCached.length > 2) {
    try {
      const parsed = JSON.parse(rawCached) as CatalogListItem[];
      if (Array.isArray(parsed) && parsed.length > 0) {
        catalogMemoryCache = parsed;
        return parsed;
      }
    } catch {
      // Ignore corrupt cache row and refresh from HTTP
    }
  }

  const upstream = await event.context.http.get(
    'https://pokeapi.co/api/v2/pokemon?limit=151&offset=0',
  );
  if (upstream.status < 200 || upstream.status >= 300) {
    throw createError({
      statusCode: upstream.status >= 400 ? upstream.status : 502,
      message: `No se pudo obtener el catálogo de PokéAPI (HTTP ${upstream.status})`,
    });
  }

  const parsedBody = JSON.parse(upstream.body) as { results?: RawNamedUrl[] };
  const rawResults = Array.isArray(parsedBody.results) ? parsedBody.results : [];
  const items: CatalogListItem[] = rawResults.map((entry, idx) => {
    const id = parsePokemonIdFromUrl(entry.url, idx + 1);
    const name = String(entry.name ?? `pokemon-${id}`).trim();
    return {
      id,
      name,
      displayName: formatDisplayName(name),
      number: formatPokemonNumber(id),
      spriteUrl: spriteUrlForId(id),
    };
  });

  if (items.length > 0) {
    catalogMemoryCache = items;
    event.context.db.execute(
      'INSERT OR REPLACE INTO catalog_cache (cache_key, payload) VALUES (?, ?);',
      ['kanto-151', JSON.stringify(items)],
    );
  }
  return items;
}

export function normalizePokemonPayload(
  id: number,
  rawPokemon: RawPokemonApiResponse,
  rawSpecies?: RawSpeciesApiResponse | null,
): PokemonDetailRecord {
  const rawName = String(rawPokemon.name ?? `pokemon-${id}`);
  const artwork =
    rawPokemon.sprites?.other?.['official-artwork']?.front_default ??
    rawPokemon.sprites?.front_default ??
    spriteUrlForId(id);

  const sortedTypes = [...(rawPokemon.types ?? [])].sort(
    (a, b) => Number(a.slot ?? 0) - Number(b.slot ?? 0),
  );
  const types: PokemonTypeBadge[] = sortedTypes
    .map((entry) => {
      const typeKey = String(entry.type?.name ?? '').toLowerCase();
      if (!typeKey) {
        return null;
      }
      return (
        TYPE_BADGE_MAP[typeKey] ?? {
          label: typeKey.toUpperCase(),
          badgeClass: 'badge-normal',
        }
      );
    })
    .filter((t): t is PokemonTypeBadge => t !== null);

  const primaryAbilityRaw =
    rawPokemon.abilities?.find((a) => !a.is_hidden)?.ability?.name ??
    rawPokemon.abilities?.[0]?.ability?.name ??
    'Desconocida';

  const stats: PokemonStatItem[] = (rawPokemon.stats ?? []).map((entry) => {
    const key = String(entry.stat?.name ?? 'stat');
    return {
      key,
      label: STAT_LABEL_MAP[key] ?? formatDisplayName(key),
      value: Math.max(0, Math.min(255, Number(entry.base_stat ?? 0))),
    };
  });

  const esGenus = rawSpecies?.genera?.find((g) => g.language?.name === 'es')?.genus;
  const enGenus = rawSpecies?.genera?.find((g) => g.language?.name === 'en')?.genus;
  const esFlavor = rawSpecies?.flavor_text_entries?.find(
    (f) => f.language?.name === 'es',
  )?.flavor_text;
  const enFlavor = rawSpecies?.flavor_text_entries?.find(
    (f) => f.language?.name === 'en',
  )?.flavor_text;

  const descriptionRaw = esFlavor ?? enFlavor ?? `Pokémon #${id} de la Pokédex Nacional.`;
  const description = descriptionRaw
    .replace(/[\f\r\n]+/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();

  return {
    id,
    name: rawName,
    displayName: formatDisplayName(rawName),
    number: formatPokemonNumber(id),
    spriteUrl: artwork,
    genus: esGenus ?? enGenus ?? `Pokémon #${id}`,
    description,
    heightMeters:
      typeof rawPokemon.height === 'number' ? `${(rawPokemon.height / 10).toFixed(1)} m` : '--',
    weightKg:
      typeof rawPokemon.weight === 'number' ? `${(rawPokemon.weight / 10).toFixed(1)} kg` : '--',
    ability: formatDisplayName(primaryAbilityRaw),
    baseExperience:
      typeof rawPokemon.base_experience === 'number' ? `${rawPokemon.base_experience} EXP` : '--',
    types,
    stats,
  };
}

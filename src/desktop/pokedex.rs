use super::{DesktopServices, NativeHttpRequest};
use std::fs;
use std::path::{Path, PathBuf};

const CACHE_DIR: &str = "/tmp/ng_native_pokedex";

/// Lightweight catalog entry returned by `PokéAPI` v2 (`GET /api/v2/pokemon?limit=1025`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PokemonIndexEntry {
    pub id: u32,
    pub name: String,
}

/// Strongly-typed Pokémon metadata retrieved from `pokeapi.co` (`/pokemon` + `/pokemon-species`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PokemonInfo {
    pub id: u32,
    pub name: String,
    pub primary_type: String,
    pub secondary_type: Option<String>,
    pub hp: u32,
    pub attack: u32,
    pub defense: u32,
    pub sp_attack: u32,
    pub sp_defense: u32,
    pub speed: u32,
    pub height_dm: u32,
    pub weight_hg: u32,
    pub base_exp: u32,
    pub ability: String,
    pub genus: String,
    pub description: String,
    pub sprite_path: String,
}

impl PokemonInfo {
    /// Formats Pokémon Pokédex number as featured on `pokemon.com` (e.g. "N.º 0025").
    pub fn format_id(&self) -> String {
        format!("N.º {:04}", self.id)
    }

    /// Formats height in meters (e.g. 4 dm -> 0.4 m).
    pub fn format_height(&self) -> String {
        format!("{:.1} m", f64::from(self.height_dm) / 10.0)
    }

    /// Formats weight in kilograms (e.g. 60 hg -> 6.0 kg).
    pub fn format_weight(&self) -> String {
        format!("{:.1} kg", f64::from(self.weight_hg) / 10.0)
    }

    /// Sum of all 6 official base stats.
    pub const fn total_stats(&self) -> u32 {
        self.hp + self.attack + self.defense + self.sp_attack + self.sp_defense + self.speed
    }

    /// Capitalized Pokémon display name (e.g. "pikachu" -> "Pikachu").
    pub fn display_name(&self) -> String {
        capitalize_slug(&self.name)
    }

    /// Translates an English `PokéAPI` type slug into Spanish Pokédex badge text.
    pub fn type_label_es(type_en: &str) -> &'static str {
        match type_en.trim().to_ascii_lowercase().as_str() {
            "grass" => "PLANTA",
            "poison" => "VENENO",
            "fire" => "FUEGO",
            "water" => "AGUA",
            "electric" => "ELÉCTRICO",
            "flying" => "VOLADOR",
            "ghost" => "FANTASMA",
            "bug" => "BICHO",
            "psychic" => "PSÍQUICO",
            "fighting" => "LUCHA",
            "rock" => "ROCA",
            "ground" => "TIERRA",
            "ice" => "HIELO",
            "dragon" => "DRAGÓN",
            "steel" => "ACERO",
            "dark" => "SINIESTRO",
            "fairy" => "HADA",
            "" => "",
            _ => "NORMAL",
        }
    }

    /// Maps an English `PokéAPI` type slug onto the closest badge CSS class defined in `app.css`.
    pub fn type_badge_class(type_en: &str) -> &'static str {
        match type_en.trim().to_ascii_lowercase().as_str() {
            "grass" | "bug" => "badge-grass",
            "poison" | "psychic" | "fairy" => "badge-poison",
            "fire" | "fighting" => "badge-fire",
            "water" | "ice" => "badge-water",
            "electric" => "badge-electric",
            "flying" | "dragon" => "badge-flying",
            _ => "badge-ghost",
        }
    }

    /// Parses raw JSON from `https://pokeapi.co/api/v2/pokemon/{id_or_name}`.
    pub fn parse_json(json: &str) -> Option<Self> {
        let id = Self::extract_number(json, "\"id\":")?;
        let name = Self::extract_string(json, "\"name\":")?;
        let height_dm = Self::extract_number(json, "\"height\":").unwrap_or(0);
        let weight_hg = Self::extract_number(json, "\"weight\":").unwrap_or(0);
        let base_exp = Self::extract_number(json, "\"base_experience\":").unwrap_or(0);

        let primary_type =
            Self::extract_primary_type(json).unwrap_or_else(|| String::from("normal"));
        let secondary_type = Self::extract_secondary_type(json);
        let ability = Self::extract_primary_ability(json)
            .map_or_else(|| String::from("—"), |s| capitalize_slug(&s));
        let hp = Self::extract_stat(json, "hp").unwrap_or(45);
        let attack = Self::extract_stat(json, "attack").unwrap_or(49);
        let defense = Self::extract_stat(json, "defense").unwrap_or(49);
        let sp_attack = Self::extract_stat(json, "special-attack").unwrap_or(50);
        let sp_defense = Self::extract_stat(json, "special-defense").unwrap_or(50);
        let speed = Self::extract_stat(json, "speed").unwrap_or(45);

        Some(Self {
            id,
            name,
            primary_type,
            secondary_type,
            hp,
            attack,
            defense,
            sp_attack,
            sp_defense,
            speed,
            height_dm,
            weight_hg,
            base_exp,
            ability,
            genus: String::from("Pokémon Nacional"),
            description: String::from("Información oficial de PokéAPI v2."),
            sprite_path: format!("{CACHE_DIR}/{id}.png"),
        })
    }

    /// Populates `genus` and `description` from `https://pokeapi.co/api/v2/pokemon-species/{id}`.
    pub fn apply_species_json(&mut self, species_json: &str) {
        if let Some(genera_idx) = species_json.find("\"genera\":") {
            let section = &species_json[genera_idx..];
            if let Some(genus) = extract_localized_field(section, "\"genus\":", "es")
                .or_else(|| extract_localized_field(section, "\"genus\":", "en"))
            {
                self.genus = genus;
            }
        }
        if let Some(flavor_idx) = species_json.find("\"flavor_text_entries\":") {
            let section = &species_json[flavor_idx..];
            if let Some(desc) = extract_localized_field(section, "\"flavor_text\":", "es")
                .or_else(|| extract_localized_field(section, "\"flavor_text\":", "en"))
            {
                self.description = desc;
            }
        }
    }

    /// Parses paginated `GET /api/v2/pokemon?limit=1025` results into `(id, name)` entries.
    pub fn parse_index_json(json: &str) -> Vec<PokemonIndexEntry> {
        let Some(results_idx) = json.find("\"results\":") else {
            return Vec::new();
        };
        let mut cursor = &json[results_idx..];
        let mut out = Vec::new();
        while let Some(name_idx) = cursor.find("\"name\":") {
            let after_name = &cursor[name_idx + 7..];
            let Some(name) = extract_quoted_value(after_name) else {
                break;
            };
            let Some(url_idx) = after_name.find("\"url\":") else {
                break;
            };
            let after_url = &after_name[url_idx + 6..];
            let Some(url) = extract_quoted_value(after_url) else {
                break;
            };
            if let Some(id) = parse_id_from_pokeapi_url(&url)
                && (1..=1025).contains(&id)
            {
                out.push(PokemonIndexEntry { id, name });
            }
            cursor = after_url;
        }
        out
    }

    fn extract_number(json: &str, key: &str) -> Option<u32> {
        let idx = json.find(key)?;
        let rest = json[idx + key.len()..].trim_start();
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        digits.parse::<u32>().ok()
    }

    fn extract_string(json: &str, key: &str) -> Option<String> {
        let idx = json.find(key)?;
        extract_quoted_value(&json[idx + key.len()..])
    }

    fn extract_primary_type(json: &str) -> Option<String> {
        let types_idx = json.find("\"types\":")?;
        let rest = &json[types_idx..];
        let sub_idx = rest.find("\"type\":")?;
        Self::extract_string(&rest[sub_idx..], "\"name\":")
    }

    fn extract_secondary_type(json: &str) -> Option<String> {
        let types_idx = json.find("\"types\":")?;
        let rest = &json[types_idx..];
        let type_key = "\"type\":";
        let first_idx = rest.find(type_key)?;
        let second_rest = &rest[first_idx + type_key.len()..];
        let second_idx = second_rest.find(type_key)?;
        Self::extract_string(&second_rest[second_idx..], "\"name\":")
    }

    fn extract_primary_ability(json: &str) -> Option<String> {
        let abil_idx = json.find("\"abilities\":")?;
        let rest = &json[abil_idx..];
        let sub_idx = rest.find("\"ability\":")?;
        Self::extract_string(&rest[sub_idx..], "\"name\":")
    }

    fn extract_stat(json: &str, stat_name: &str) -> Option<u32> {
        let compact = format!("\"name\":\"{stat_name}\"");
        let spaced = format!("\"name\": \"{stat_name}\"");
        let idx = json.find(&compact).or_else(|| json.find(&spaced))?;
        let before = &json[..idx];
        let stat_key = "\"base_stat\":";
        let last_stat = before.rfind(stat_key)?;
        let rest = before[last_stat + stat_key.len()..].trim_start();
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        digits.parse::<u32>().ok()
    }
}

fn capitalize_slug(slug: &str) -> String {
    slug.split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + chars.as_str()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_quoted_value(after_colon: &str) -> Option<String> {
    let quote_start = after_colon.find('"')?;
    let rest = &after_colon[quote_start + 1..];
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => return Some(out),
            '\\' => match chars.next() {
                Some('n' | 'r' | 'f' | 't') => out.push(' '),
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    if hex.eq_ignore_ascii_case("000c") {
                        out.push(' ');
                    } else if let Ok(code) = u32::from_str_radix(&hex, 16)
                        && let Some(decoded) = char::from_u32(code)
                    {
                        out.push(if decoded.is_control() { ' ' } else { decoded });
                    }
                }
                Some(escaped) => out.push(escaped),
                None => break,
            },
            c => out.push(c),
        }
    }
    None
}

fn extract_localized_field(section: &str, field_key: &str, lang: &str) -> Option<String> {
    let mut cursor = section;
    let needle_compact = format!("\"name\":\"{lang}\"");
    let needle_spaced = format!("\"name\": \"{lang}\"");
    while let Some(idx) = cursor.find(field_key) {
        let after = &cursor[idx + field_key.len()..];
        let Some(val) = extract_quoted_value(after) else {
            break;
        };
        let window_end = after.find(field_key).unwrap_or(after.len()).min(260);
        let window = &after[..window_end];
        if window.contains(&needle_compact) || window.contains(&needle_spaced) {
            let collapsed = val.split_whitespace().collect::<Vec<_>>().join(" ");
            if !collapsed.is_empty() {
                return Some(collapsed);
            }
        }
        cursor = &after[after.find('"').map_or(0, |p| p + 1)..];
    }
    None
}

fn parse_id_from_pokeapi_url(url: &str) -> Option<u32> {
    url.trim_end_matches('/')
        .rsplit('/')
        .next()?
        .parse::<u32>()
        .ok()
}

fn artwork_cache_path(id: u32) -> PathBuf {
    PathBuf::from(format!("{CACHE_DIR}/{id}.png"))
}

fn artwork_url(id: u32) -> String {
    format!(
        "https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/official-artwork/{id}.png"
    )
}

fn has_valid_cached_file(path: &Path, min_bytes: u64) -> bool {
    fs::metadata(path).is_ok_and(|m| m.len() > min_bytes)
}

fn read_cached_json(name: &str) -> Option<String> {
    let path = format!("{CACHE_DIR}/{name}");
    fs::read_to_string(path).ok().filter(|s| s.len() > 16)
}

fn write_cached_json(name: &str, body: &str) {
    if body.len() > 16 {
        let _ = fs::create_dir_all(CACHE_DIR);
        let _ = fs::write(format!("{CACHE_DIR}/{name}"), body);
    }
}

struct FallbackSpec {
    id: u32,
    name: &'static str,
    primary_type: &'static str,
    secondary_type: Option<&'static str>,
    stats: [u32; 9], // [hp, atk, def, sp_atk, sp_def, spd, height_dm, weight_hg, base_exp]
    ability: &'static str,
    genus: &'static str,
    description: &'static str,
}

const OFFLINE_SPECS: &[FallbackSpec] = &[
    FallbackSpec {
        id: 1,
        name: "bulbasaur",
        primary_type: "grass",
        secondary_type: Some("poison"),
        stats: [45, 49, 49, 65, 65, 45, 7, 69, 64],
        ability: "Overgrow",
        genus: "Pokémon Semilla",
        description: "Una rara semilla le fue plantada en el lomo al nacer. La planta brota y crece con este Pokémon.",
    },
    FallbackSpec {
        id: 4,
        name: "charmander",
        primary_type: "fire",
        secondary_type: None,
        stats: [39, 52, 43, 60, 50, 65, 6, 85, 62],
        ability: "Blaze",
        genus: "Pokémon Lagartija",
        description: "Prefiere los sitios calientes. Dicen que cuando llueve sale vapor de la punta de su cola.",
    },
    FallbackSpec {
        id: 6,
        name: "charizard",
        primary_type: "fire",
        secondary_type: Some("flying"),
        stats: [78, 84, 78, 109, 85, 100, 17, 905, 267],
        ability: "Blaze",
        genus: "Pokémon Llama",
        description: "Escupe un fuego tan caliente que funde las rocas. Causa incendios forestales sin querer.",
    },
    FallbackSpec {
        id: 25,
        name: "pikachu",
        primary_type: "electric",
        secondary_type: None,
        stats: [35, 55, 40, 50, 50, 90, 4, 60, 112],
        ability: "Static",
        genus: "Pokémon Ratón",
        description: "Cuando se enfada, este Pokémon descarga la energía que almacena en el interior de las bolsas de las mejillas.",
    },
    FallbackSpec {
        id: 94,
        name: "gengar",
        primary_type: "ghost",
        secondary_type: Some("poison"),
        stats: [60, 65, 60, 130, 75, 110, 15, 405, 250],
        ability: "Cursed Body",
        genus: "Pokémon Sombra",
        description: "Las noches de luna llena, a este Pokémon le gusta imitar las sombras de la gente y burlarse de sus miedos.",
    },
];

fn offline_fallback_pokemon(clean: &str) -> Option<PokemonInfo> {
    let spec = OFFLINE_SPECS
        .iter()
        .find(|s| s.name == clean || s.id.to_string() == clean)?;
    Some(PokemonInfo {
        id: spec.id,
        name: String::from(spec.name),
        primary_type: String::from(spec.primary_type),
        secondary_type: spec.secondary_type.map(String::from),
        hp: spec.stats[0],
        attack: spec.stats[1],
        defense: spec.stats[2],
        sp_attack: spec.stats[3],
        sp_defense: spec.stats[4],
        speed: spec.stats[5],
        height_dm: spec.stats[6],
        weight_hg: spec.stats[7],
        base_exp: spec.stats[8],
        ability: String::from(spec.ability),
        genus: String::from(spec.genus),
        description: String::from(spec.description),
        sprite_path: artwork_cache_path(spec.id).to_string_lossy().into_owned(),
    })
}

fn default_index_fallback() -> Vec<PokemonIndexEntry> {
    [
        (1, "bulbasaur"),
        (2, "ivysaur"),
        (3, "venusaur"),
        (4, "charmander"),
        (5, "charmeleon"),
        (6, "charizard"),
        (7, "squirtle"),
        (8, "wartortle"),
        (9, "blastoise"),
        (25, "pikachu"),
        (94, "gengar"),
        (150, "mewtwo"),
    ]
    .into_iter()
    .map(|(id, name)| PokemonIndexEntry {
        id,
        name: String::from(name),
    })
    .collect()
}

impl DesktopServices {
    /// Retrieves the official 1,025-Pokémon index from `PokéAPI v2` (cached on disk).
    pub fn fetch_pokemon_index(&mut self) -> Vec<PokemonIndexEntry> {
        if let Some(cached) = read_cached_json("index_1025.json") {
            let entries = PokemonInfo::parse_index_json(&cached);
            if !entries.is_empty() {
                return entries;
            }
        }
        let response = self.execute_http(&NativeHttpRequest::new(
            "GET",
            "https://pokeapi.co/api/v2/pokemon?limit=1025&offset=0",
        ));
        if response.status < 400 {
            let entries = PokemonInfo::parse_index_json(&response.body);
            if !entries.is_empty() {
                write_cached_json("index_1025.json", &response.body);
                return entries;
            }
        }
        default_index_fallback()
    }

    /// Retrieves a batch of Pokémon using HTTP/2 parallel multiplexing for uncached JSON + sprites.
    /// Species flavor descriptions are intentionally deferred until `fetch_pokemon_detail`.
    pub fn fetch_pokemon_batch(&mut self, ids: &[u32]) -> Vec<PokemonInfo> {
        let mut pairs: Vec<(String, PathBuf)> = Vec::new();
        for &id in ids {
            let json_path = PathBuf::from(format!("{CACHE_DIR}/poke_{id}.json"));
            if !has_valid_cached_file(&json_path, 16) {
                pairs.push((format!("https://pokeapi.co/api/v2/pokemon/{id}"), json_path));
            }
            let img_path = artwork_cache_path(id);
            if !has_valid_cached_file(&img_path, 256) {
                pairs.push((artwork_url(id), img_path));
            }
        }
        if !pairs.is_empty() {
            let _ = self.download_files_parallel(&pairs);
        }

        ids.iter()
            .filter_map(|&id| {
                read_cached_json(&format!("poke_{id}.json"))
                    .and_then(|json| PokemonInfo::parse_json(&json))
                    .or_else(|| offline_fallback_pokemon(&id.to_string()))
            })
            .collect()
    }

    /// Retrieves complete Pokémon detail (including localized `/pokemon-species/{id}` metadata).
    pub fn fetch_pokemon_detail(&mut self, id: u32) -> Result<PokemonInfo, String> {
        self.fetch_pokemon(&id.to_string())
    }

    /// Retrieves Pokémon + species metadata from `PokéAPI v2` with warm `SoupSession` + disk caching.
    pub fn fetch_pokemon(&mut self, query: &str) -> Result<PokemonInfo, String> {
        let clean = query.trim().to_ascii_lowercase();
        if clean.is_empty() {
            return Err(String::from("Empty Pokemon query"));
        }

        let cache_key = format!("poke_{clean}.json");
        let mut parsed = read_cached_json(&cache_key).and_then(|s| PokemonInfo::parse_json(&s));
        let mut last_status = (200_u16, String::from("OK"));

        if parsed.is_none() {
            let url = format!("https://pokeapi.co/api/v2/pokemon/{clean}");
            let response = self.execute_http(&NativeHttpRequest::new("GET", url));
            last_status = (response.status, response.status_text.clone());
            if response.status < 400
                && let Some(info) = PokemonInfo::parse_json(&response.body)
            {
                write_cached_json(&format!("poke_{}.json", info.id), &response.body);
                write_cached_json(&cache_key, &response.body);
                parsed = Some(info);
            }
        }

        if let Some(mut poke) = parsed {
            let img_path = artwork_cache_path(poke.id);
            if !has_valid_cached_file(&img_path, 256) {
                let _ = self.download_files_parallel(&[(artwork_url(poke.id), img_path.clone())]);
            }
            poke.sprite_path = img_path.to_string_lossy().into_owned();
            self.populate_species_metadata(&mut poke);
            return Ok(poke);
        }

        offline_fallback_pokemon(&clean).ok_or_else(|| {
            format!(
                "Failed to fetch Pokémon '{clean}' ({}: {})",
                last_status.0, last_status.1
            )
        })
    }

    fn populate_species_metadata(&mut self, poke: &mut PokemonInfo) {
        let species_key = format!("species_{}.json", poke.id);
        if let Some(cached) = read_cached_json(&species_key) {
            poke.apply_species_json(&cached);
            return;
        }
        let url = format!("https://pokeapi.co/api/v2/pokemon-species/{}", poke.id);
        let response = self.execute_http(&NativeHttpRequest::new("GET", url));
        if response.status < 400 && response.body.contains("\"genera\"") {
            write_cached_json(&species_key, &response.body);
            poke.apply_species_json(&response.body);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pokemon_json_payload_accurately() {
        let json = r#"{
            "id": 25,
            "name": "pikachu",
            "height": 4,
            "weight": 60,
            "base_experience": 112,
            "abilities": [
                {"ability": {"name": "static"}}
            ],
            "types": [
                {"slot": 1, "type": {"name": "electric"}}
            ],
            "stats": [
                {"base_stat": 35, "stat": {"name": "hp"}},
                {"base_stat": 55, "stat": {"name": "attack"}},
                {"base_stat": 40, "stat": {"name": "defense"}},
                {"base_stat": 50, "stat": {"name": "special-attack"}},
                {"base_stat": 50, "stat": {"name": "special-defense"}},
                {"base_stat": 90, "stat": {"name": "speed"}}
            ]
        }"#;

        let mut p = PokemonInfo::parse_json(json).unwrap();
        assert_eq!(p.id, 25);
        assert_eq!(p.display_name(), "Pikachu");
        assert_eq!(p.format_id(), "N.º 0025");
        assert_eq!(p.ability, "Static");
        assert_eq!(p.sp_attack, 50);
        assert_eq!(p.sp_defense, 50);
        assert_eq!(p.total_stats(), 320);

        let species_json = r#"{
            "genera": [
                {"genus": "Pokémon Ratón", "language": {"name": "es"}}
            ],
            "flavor_text_entries": [
                {"flavor_text": "Almacena electricidad\nen las mejillas.", "language": {"name": "es"}}
            ]
        }"#;
        p.apply_species_json(species_json);
        assert_eq!(p.genus, "Pokémon Ratón");
        assert_eq!(p.description, "Almacena electricidad en las mejillas.");
    }

    #[test]
    fn parses_paginated_pokeapi_index_entries() {
        let index_json = r#"{
            "count": 1351,
            "results": [
                {"name": "bulbasaur", "url": "https://pokeapi.co/api/v2/pokemon/1/"},
                {"name": "pikachu", "url": "https://pokeapi.co/api/v2/pokemon/25/"}
            ]
        }"#;
        let entries = PokemonInfo::parse_index_json(index_json);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, 1);
        assert_eq!(entries[0].name, "bulbasaur");
        assert_eq!(entries[1].id, 25);
        assert_eq!(entries[1].name, "pikachu");
        assert_eq!(PokemonInfo::type_label_es("electric"), "ELÉCTRICO");
    }
}

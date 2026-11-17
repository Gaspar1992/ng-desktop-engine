//! Pokédex controller: drives infinite-scroll `@defer` pagination, live `PokéAPI` v2
//! search filtering, and full Pokémon species detail sheets over native GTK4 widgets.

use super::{DesktopServices, NativeHttpEngine, PokemonIndexEntry, PokemonInfo};
use crate::engine::{EventPayload, NativeEvent, NodeId, RenderEngine};
use crate::platforms::AnyPlatformBackend;
use crate::runtime::JsRuntime;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

type Engine = RenderEngine<AnyPlatformBackend>;
type DetailResult = Result<PokemonInfo, String>;

const PAGE_SIZE: usize = 12;
const SEARCH_LIMIT: usize = 18;
const SEARCH_DEBOUNCE: Duration = Duration::from_millis(120);

enum BatchOutcome {
    Initial {
        generation: u64,
        index: Vec<PokemonIndexEntry>,
        batch: Vec<PokemonInfo>,
    },
    Page {
        generation: u64,
        batch: Vec<PokemonInfo>,
        replace: bool,
        next_loaded: Option<usize>,
        subtitle: String,
    },
}

#[derive(Default)]
struct UiNodes {
    list_view: Option<NodeId>,
    list_subtitle: Option<NodeId>,
    pokemon_grid: Option<NodeId>,
    load_more_btn: Option<NodeId>,
    defer_placeholder: Option<NodeId>,
    detail_view: Option<NodeId>,
    spinner: Option<NodeId>,
    id_spin: Option<NodeId>,
    name: Option<NodeId>,
    genus: Option<NodeId>,
    description: Option<NodeId>,
    stats_label: Option<NodeId>,
    number: Option<NodeId>,
    sprite: Option<NodeId>,
    type_primary: Option<NodeId>,
    type_secondary: Option<NodeId>,
    height: Option<NodeId>,
    weight: Option<NodeId>,
    ability: Option<NodeId>,
    base_exp: Option<NodeId>,
    notes: Option<NodeId>,
    hp_bar: Option<NodeId>,
    atk_bar: Option<NodeId>,
    def_bar: Option<NodeId>,
    sp_atk_bar: Option<NodeId>,
    sp_def_bar: Option<NodeId>,
    spd_bar: Option<NodeId>,
    lbl_hp_bar: Option<NodeId>,
    lbl_atk_bar: Option<NodeId>,
    lbl_def_bar: Option<NodeId>,
    lbl_sp_atk_bar: Option<NodeId>,
    lbl_sp_def_bar: Option<NodeId>,
    lbl_spd_bar: Option<NodeId>,
}

impl UiNodes {
    fn resolve(runtime: &JsRuntime) -> Self {
        let node = |var: &str| runtime.resolve_var(var).ok();
        Self {
            list_view: node("list_view"),
            list_subtitle: node("list_subtitle"),
            pokemon_grid: node("pokemon_grid"),
            load_more_btn: node("btn_load_more"),
            defer_placeholder: node("defer_placeholder"),
            detail_view: node("detail_view"),
            spinner: node("loading_spinner"),
            id_spin: node("poke_id_spin"),
            name: node("hello"),
            genus: node("poke_genus"),
            description: node("poke_description"),
            stats_label: node("lbl_stats"),
            number: node("poke_num"),
            sprite: node("poke_sprite"),
            type_primary: node("poke_type_primary"),
            type_secondary: node("poke_type_secondary"),
            height: node("lbl_height"),
            weight: node("lbl_weight"),
            ability: node("lbl_ability"),
            base_exp: node("lbl_base_exp"),
            notes: node("poke_notes"),
            hp_bar: node("hp_bar"),
            atk_bar: node("atk_bar"),
            def_bar: node("def_bar"),
            sp_atk_bar: node("sp_atk_bar"),
            sp_def_bar: node("sp_def_bar"),
            spd_bar: node("spd_bar"),
            lbl_hp_bar: node("lbl_hp_bar"),
            lbl_atk_bar: node("lbl_atk_bar"),
            lbl_def_bar: node("lbl_def_bar"),
            lbl_sp_atk_bar: node("lbl_sp_atk_bar"),
            lbl_sp_def_bar: node("lbl_sp_def_bar"),
            lbl_spd_bar: node("lbl_spd_bar"),
        }
    }
}

/// Drives infinite-scroll Pokédex pagination, live search filtering, and detail sheets.
pub struct PokedexController {
    nodes: UiNodes,
    http_engine: Arc<Mutex<NativeHttpEngine>>,
    active_batch_gen: Arc<AtomicU64>,
    last_signal: i64,
    in_detail_view: bool,
    catalog: Vec<PokemonIndexEntry>,
    loaded_count: usize,
    active_query: String,
    grid_cards: Vec<NodeId>,
    card_buttons: HashMap<NodeId, u32>,
    pending_batch: Option<Receiver<BatchOutcome>>,
    pending_detail: Option<Receiver<DetailResult>>,
    queued_search: Option<(String, Instant)>,
}

impl PokedexController {
    pub fn new(runtime: &JsRuntime) -> Self {
        Self {
            nodes: UiNodes::resolve(runtime),
            http_engine: runtime.desktop().shared_http_engine(),
            active_batch_gen: Arc::new(AtomicU64::new(0)),
            last_signal: 0,
            in_detail_view: false,
            catalog: Vec::new(),
            loaded_count: 0,
            active_query: String::new(),
            grid_cards: Vec::new(),
            card_buttons: HashMap::new(),
            pending_batch: None,
            pending_detail: None,
            queued_search: None,
        }
    }

    fn next_batch_generation(&self) -> u64 {
        self.active_batch_gen.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Loads the `PokéAPI` v2 catalog index and first page of 12 Pokémon asynchronously.
    pub fn start_initial_load(&mut self, engine: &mut Engine) {
        self.set_subtitle(engine, "Conectando con PokéAPI v2 (lote inicial 1..12)...");
        let generation = self.next_batch_generation();
        let gen_flag = Arc::clone(&self.active_batch_gen);
        let http = Arc::clone(&self.http_engine);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            if gen_flag.load(Ordering::Relaxed) != generation {
                return;
            }
            let mut svc = DesktopServices::with_http_engine(http);
            let index = svc.fetch_pokemon_index();
            let ids: Vec<u32> = index.iter().take(PAGE_SIZE).map(|e| e.id).collect();
            let batch = svc.fetch_pokemon_batch(&ids);
            let _ = tx.send(BatchOutcome::Initial {
                generation,
                index,
                batch,
            });
        });
        self.pending_batch = Some(rx);
        self.sync_loading_spinner(engine);
    }

    /// Re-binds node handles and view state after a hot reload re-created the scene.
    pub fn refresh_after_reload(&mut self, engine: &mut Engine, runtime: &JsRuntime) {
        self.nodes = UiNodes::resolve(runtime);
        self.grid_cards.clear();
        self.card_buttons.clear();
        self.loaded_count = 0;
        self.last_signal = runtime.signal_value("pokemonId").unwrap_or(0);
        self.set_view(engine, self.last_signal != 0);
        self.start_initial_load(engine);
    }

    pub fn show_list(&mut self, engine: &mut Engine) {
        self.set_view(engine, false);
    }

    /// Routes native widget events (`SearchEntry`, `SpinButton`, card clicks, scroll edge).
    pub fn handle_event(&mut self, ev: &NativeEvent, engine: &mut Engine, runtime: &mut JsRuntime) {
        if let EventPayload::Text(ref query) = ev.payload {
            self.queue_search(query);
            return;
        }
        if Some(ev.node) == self.nodes.id_spin
            && let EventPayload::Int(id) = ev.payload
            && id > 0
        {
            let _ = runtime.set_signal(engine, "pokemonId", id);
            return;
        }
        if ev.event_name == "viewport-reached" {
            if self.active_query.is_empty() && !self.in_detail_view {
                self.load_next_page(engine);
            }
            return;
        }
        if ev.event_name == "clicked" {
            if Some(ev.node) == self.nodes.load_more_btn {
                self.load_next_page(engine);
            } else if let Some(&poke_id) = self.card_buttons.get(&ev.node) {
                self.open_pokemon_detail(engine, runtime, poke_id);
            }
        }
    }

    /// Queues a live search filter; fires after [`SEARCH_DEBOUNCE`] ms of idle typing.
    pub fn queue_search(&mut self, query: &str) {
        let clean = query.trim().to_ascii_lowercase();
        if clean.is_empty() && self.active_query.is_empty() && self.queued_search.is_none() {
            return;
        }
        self.queued_search = Some((clean, Instant::now()));
    }

    /// Advances time-based, signal-driven, and worker-thread state once per frame.
    pub fn tick(&mut self, engine: &mut Engine, runtime: &mut JsRuntime) {
        if let Some((query, queued_at)) = &self.queued_search
            && queued_at.elapsed() >= SEARCH_DEBOUNCE
        {
            let query = query.clone();
            self.queued_search = None;
            self.start_search_filter(engine, &query);
        }
        self.sync_navigation(engine, runtime);
        self.poll_batch(engine);
        self.poll_detail(engine, runtime);
    }

    fn open_pokemon_detail(&mut self, engine: &mut Engine, runtime: &mut JsRuntime, poke_id: u32) {
        self.last_signal = i64::from(poke_id);
        let _ = runtime.set_signal(engine, "pokemonId", self.last_signal);
        self.set_view(engine, true);
        self.start_detail_fetch(engine, &poke_id.to_string());
    }

    fn load_next_page(&mut self, engine: &mut Engine) {
        if self.pending_batch.is_some() || !self.active_query.is_empty() || self.catalog.is_empty()
        {
            return;
        }
        let end = (self.loaded_count + PAGE_SIZE).min(self.catalog.len());
        if self.loaded_count >= end {
            return;
        }
        let ids: Vec<u32> = self.catalog[self.loaded_count..end]
            .iter()
            .map(|e| e.id)
            .collect();
        let total = self.catalog.len();
        let subtitle = format!(
            "Mostrando 1..{end} de {total} Pokémon — Desplázate para cargar más (@defer on viewport)"
        );
        self.set_defer_status(
            engine,
            &format!("Cargando Pokémon {}..{end}...", self.loaded_count + 1),
        );

        let generation = self.next_batch_generation();
        let gen_flag = Arc::clone(&self.active_batch_gen);
        let http = Arc::clone(&self.http_engine);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            if gen_flag.load(Ordering::Relaxed) != generation {
                return;
            }
            let batch = DesktopServices::with_http_engine(http).fetch_pokemon_batch(&ids);
            let _ = tx.send(BatchOutcome::Page {
                generation,
                batch,
                replace: false,
                next_loaded: Some(end),
                subtitle,
            });
        });
        self.pending_batch = Some(rx);
        self.sync_loading_spinner(engine);
    }

    fn start_search_filter(&mut self, engine: &mut Engine, query: &str) {
        self.active_query = query.to_string();
        self.set_view(engine, false);
        let generation = self.next_batch_generation();
        let gen_flag = Arc::clone(&self.active_batch_gen);
        let http = Arc::clone(&self.http_engine);

        if query.is_empty() {
            let ids: Vec<u32> = self.catalog.iter().take(PAGE_SIZE).map(|e| e.id).collect();
            let count = ids.len();
            let total = self.catalog.len();
            let subtitle = format!(
                "Mostrando 1..{count} de {total} Pokémon — Desplázate para cargar más (@defer on viewport)"
            );
            self.set_subtitle(engine, "Restaurando catálogo inicial...");
            let (tx, rx) = mpsc::channel();
            thread::spawn(move || {
                if gen_flag.load(Ordering::Relaxed) != generation {
                    return;
                }
                let batch = DesktopServices::with_http_engine(http).fetch_pokemon_batch(&ids);
                let _ = tx.send(BatchOutcome::Page {
                    generation,
                    batch,
                    replace: true,
                    next_loaded: Some(count),
                    subtitle,
                });
            });
            self.pending_batch = Some(rx);
            self.sync_loading_spinner(engine);
            return;
        }

        let matched_ids: Vec<u32> = self
            .catalog
            .iter()
            .filter(|e| e.name.contains(query) || e.id.to_string().starts_with(query))
            .take(SEARCH_LIMIT)
            .map(|e| e.id)
            .collect();
        let owned_query = query.to_string();
        self.set_subtitle(
            engine,
            &format!("Buscando '{owned_query}' en PokéAPI v2..."),
        );

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            if gen_flag.load(Ordering::Relaxed) != generation {
                return;
            }
            let mut svc = DesktopServices::with_http_engine(http);
            let batch = if matched_ids.is_empty() {
                svc.fetch_pokemon(&owned_query).into_iter().collect()
            } else {
                svc.fetch_pokemon_batch(&matched_ids)
            };
            let subtitle = if batch.is_empty() {
                format!("No se encontraron Pokémon que encajen con '{owned_query}'.")
            } else {
                format!(
                    "Resultados para '{owned_query}': {} Pokémon encontrados (pulsa «Ver ficha ›»)",
                    batch.len()
                )
            };
            let _ = tx.send(BatchOutcome::Page {
                generation,
                batch,
                replace: true,
                next_loaded: None,
                subtitle,
            });
        });
        self.pending_batch = Some(rx);
        self.sync_loading_spinner(engine);
    }

    fn sync_navigation(&mut self, engine: &mut Engine, runtime: &JsRuntime) {
        let current = runtime.signal_value("pokemonId").unwrap_or(0);
        if current == self.last_signal {
            return;
        }
        self.last_signal = current;
        if current <= 0 {
            self.pending_detail = None;
            self.sync_loading_spinner(engine);
            self.set_view(engine, false);
        } else {
            self.set_view(engine, true);
            self.start_detail_fetch(engine, &current.to_string());
        }
    }

    fn start_detail_fetch(&mut self, engine: &mut Engine, query: &str) {
        let (tx, rx) = mpsc::channel();
        let query = query.to_string();
        let http = Arc::clone(&self.http_engine);
        thread::spawn(move || {
            let mut svc = DesktopServices::with_http_engine(http);
            let res = if let Ok(id) = query.parse::<u32>() {
                svc.fetch_pokemon_detail(id)
            } else {
                svc.fetch_pokemon(&query)
            };
            let _ = tx.send(res);
        });
        self.pending_detail = Some(rx);
        self.sync_loading_spinner(engine);
    }

    fn poll_batch(&mut self, engine: &mut Engine) {
        let Some(rx) = &self.pending_batch else {
            return;
        };
        let outcome = match rx.try_recv() {
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                self.pending_batch = None;
                self.sync_loading_spinner(engine);
                return;
            }
            Ok(res) => res,
        };
        self.pending_batch = None;
        self.sync_loading_spinner(engine);

        let current_gen = self.active_batch_gen.load(Ordering::Relaxed);
        match outcome {
            BatchOutcome::Initial {
                generation,
                index,
                batch,
            } => {
                if generation != current_gen {
                    return;
                }
                self.catalog = index;
                self.clear_grid_cards(engine);
                self.loaded_count = batch.len();
                self.append_cards(engine, &batch);
                let total = self.catalog.len();
                self.set_subtitle(
                    engine,
                    &format!(
                        "Mostrando 1..{} de {total} Pokémon — Desplázate para cargar más (@defer on viewport)",
                        self.loaded_count
                    ),
                );
                self.set_defer_status(
                    engine,
                    "Desplázate hacia abajo para cargar el siguiente lote (@defer on viewport)...",
                );
            }
            BatchOutcome::Page {
                generation,
                batch,
                replace,
                next_loaded,
                subtitle,
            } => {
                if generation != current_gen {
                    return;
                }
                if replace {
                    self.clear_grid_cards(engine);
                }
                if let Some(count) = next_loaded {
                    self.loaded_count = count;
                }
                self.append_cards(engine, &batch);
                self.set_subtitle(engine, &subtitle);
                let hint = if self.active_query.is_empty() {
                    "Desplázate hacia abajo para cargar el siguiente lote (@defer on viewport)..."
                } else {
                    "Borra la búsqueda para volver a la lista infinita."
                };
                self.set_defer_status(engine, hint);
            }
        }
    }

    fn poll_detail(&mut self, engine: &mut Engine, runtime: &mut JsRuntime) {
        let Some(rx) = &self.pending_detail else {
            return;
        };
        let outcome = match rx.try_recv() {
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(String::from("fetch worker stopped")),
            Ok(result) => result,
        };
        self.pending_detail = None;
        self.sync_loading_spinner(engine);
        match outcome {
            Ok(poke) => {
                println!(
                    "\n[PokéAPI] {} {} ({})",
                    poke.format_id(),
                    poke.display_name(),
                    poke.primary_type
                );
                self.render_detail(engine, &poke);
                self.set_view(engine, true);
                self.last_signal = i64::from(poke.id);
                let _ = runtime.set_signal(engine, "pokemonId", i64::from(poke.id));
            }
            Err(err) => eprintln!("[PokéAPI] {err}"),
        }
    }

    fn clear_grid_cards(&mut self, engine: &mut Engine) {
        if let Some(grid) = self.nodes.pokemon_grid {
            for card in self.grid_cards.drain(..) {
                let _ = engine.remove_child(grid, card);
            }
        }
        self.card_buttons.clear();
    }

    fn append_cards(&mut self, engine: &mut Engine, batch: &[PokemonInfo]) {
        let Some(grid) = self.nodes.pokemon_grid else {
            return;
        };
        for poke in batch {
            if let Ok((card, btn)) = Self::build_card_node(engine, poke) {
                let _ = engine.append_child(grid, card);
                self.grid_cards.push(card);
                self.card_buttons.insert(btn, poke.id);
            }
        }
    }

    fn build_card_node(
        engine: &mut Engine,
        poke: &PokemonInfo,
    ) -> Result<(NodeId, NodeId), crate::engine::EngineError> {
        let card = engine.create_element("ng-box")?;
        engine.set_attribute(card, "orientation", "vertical")?;
        engine.set_attribute(card, "spacing", "6px")?;
        engine.add_class(card, "grid-card")?;

        let img = engine.create_element("ng-image")?;
        engine.set_attribute(img, "width", "120")?;
        engine.set_attribute(img, "height", "120")?;
        engine.set_attribute(img, "src", &poke.sprite_path)?;
        engine.set_text(img, &poke.sprite_path)?;
        engine.append_child(card, img)?;

        let num = engine.create_element("ng-label")?;
        engine.set_text(num, poke.format_id())?;
        engine.add_class(num, "poke-num")?;
        engine.append_child(card, num)?;

        let name = engine.create_element("ng-label")?;
        engine.set_text(name, poke.display_name())?;
        engine.add_class(name, "card-name")?;
        engine.append_child(card, name)?;

        let types_row = engine.create_element("ng-box")?;
        engine.set_attribute(types_row, "orientation", "horizontal")?;
        engine.set_attribute(types_row, "spacing", "6px")?;
        Self::append_type_badge(engine, types_row, &poke.primary_type)?;
        if let Some(sec) = &poke.secondary_type {
            Self::append_type_badge(engine, types_row, sec)?;
        }
        engine.append_child(card, types_row)?;

        let btn = engine.create_element("ng-button")?;
        engine.set_text(btn, "Ver ficha ›")?;
        engine.add_class(btn, "suggested-action")?;
        engine.append_child(card, btn)?;

        Ok((card, btn))
    }

    fn append_type_badge(
        engine: &mut Engine,
        parent: NodeId,
        type_en: &str,
    ) -> Result<(), crate::engine::EngineError> {
        let badge = engine.create_element("ng-label")?;
        engine.set_text(badge, PokemonInfo::type_label_es(type_en))?;
        engine.add_class(badge, "type-badge-text")?;
        engine.add_class(badge, PokemonInfo::type_badge_class(type_en))?;
        engine.append_child(parent, badge)
    }

    fn set_view(&mut self, engine: &mut Engine, show_detail: bool) {
        self.in_detail_view = show_detail;
        for (node, visible) in [
            (self.nodes.list_view, !show_detail),
            (self.nodes.detail_view, show_detail),
        ] {
            if let Some(node) = node {
                let _ = engine.set_attribute(node, "visible", visible.to_string());
            }
        }
        if !show_detail && let Some(spin) = self.nodes.id_spin {
            let _ = engine.set_attribute(spin, "value", "0");
        }
    }

    fn set_subtitle(&self, engine: &mut Engine, text: &str) {
        if let Some(lbl) = self.nodes.list_subtitle {
            let _ = engine.set_text(lbl, text);
        }
    }

    fn set_defer_status(&self, engine: &mut Engine, text: &str) {
        if let Some(lbl) = self.nodes.defer_placeholder {
            let _ = engine.set_text(lbl, text);
        }
    }

    fn sync_loading_spinner(&self, engine: &mut Engine) {
        let loading = self.pending_batch.is_some() || self.pending_detail.is_some();
        if let Some(spinner) = self.nodes.spinner {
            let _ = engine.set_attribute(spinner, "visible", loading.to_string());
        }
    }

    fn render_detail(&self, engine: &mut Engine, poke: &PokemonInfo) {
        let nodes = &self.nodes;
        let sec_label = poke
            .secondary_type
            .as_deref()
            .map_or("", PokemonInfo::type_label_es);
        let texts = [
            (nodes.name, poke.display_name().to_uppercase()),
            (nodes.number, poke.format_id()),
            (nodes.genus, poke.genus.clone()),
            (nodes.description, poke.description.clone()),
            (
                nodes.type_primary,
                String::from(PokemonInfo::type_label_es(&poke.primary_type)),
            ),
            (nodes.type_secondary, String::from(sec_label)),
            (nodes.height, poke.format_height()),
            (nodes.weight, poke.format_weight()),
            (nodes.ability, poke.ability.clone()),
            (nodes.base_exp, format!("{} EXP", poke.base_exp)),
            (
                nodes.notes,
                format!("{} — {}", poke.format_id(), poke.description),
            ),
            (
                nodes.stats_label,
                format!(
                    "PS: {} | ATQ: {} | DEF: {} | AT.ESP: {} | DF.ESP: {} | VEL: {} (Total: {})",
                    poke.hp,
                    poke.attack,
                    poke.defense,
                    poke.sp_attack,
                    poke.sp_defense,
                    poke.speed,
                    poke.total_stats()
                ),
            ),
            (nodes.lbl_hp_bar, format!("PS ({} / 255)", poke.hp)),
            (nodes.lbl_atk_bar, format!("Ataque ({} / 255)", poke.attack)),
            (
                nodes.lbl_def_bar,
                format!("Defensa ({} / 255)", poke.defense),
            ),
            (
                nodes.lbl_sp_atk_bar,
                format!("Ataque Especial ({} / 255)", poke.sp_attack),
            ),
            (
                nodes.lbl_sp_def_bar,
                format!("Defensa Especial ({} / 255)", poke.sp_defense),
            ),
            (
                nodes.lbl_spd_bar,
                format!("Velocidad ({} / 255)", poke.speed),
            ),
        ];
        for (node, text) in texts {
            if let Some(node) = node {
                let _ = engine.set_text(node, text);
            }
        }
        if let Some(sec_node) = nodes.type_secondary {
            let _ = engine.set_attribute(
                sec_node,
                "visible",
                poke.secondary_type.is_some().to_string(),
            );
        }
        if let Some(sprite) = nodes.sprite {
            let _ = engine.set_text(sprite, poke.sprite_path.clone());
            let _ = engine.set_attribute(sprite, "src", poke.sprite_path.clone());
        }
        for (node, value) in [
            (nodes.id_spin, poke.id),
            (nodes.hp_bar, poke.hp),
            (nodes.atk_bar, poke.attack),
            (nodes.def_bar, poke.defense),
            (nodes.sp_atk_bar, poke.sp_attack),
            (nodes.sp_def_bar, poke.sp_defense),
            (nodes.spd_bar, poke.speed),
        ] {
            if let Some(node) = node {
                let _ = engine.set_attribute(node, "value", value.to_string());
            }
        }
    }
}

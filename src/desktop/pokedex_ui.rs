//! Pokédex example glue: binds engine state (signals, search entry) to the native widget tree.
//!
//! Network access runs on a worker thread so the GTK event pump never blocks; the loading
//! state is surfaced through the `ng-spinner` primitive.

use super::{DesktopServices, PokemonInfo};
use crate::engine::{NodeId, RenderEngine};
use crate::platforms::AnyPlatformBackend;
use crate::runtime::JsRuntime;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

type Engine = RenderEngine<AnyPlatformBackend>;
type FetchResult = Result<PokemonInfo, String>;

/// Idle time after the last keystroke before the search entry triggers a request.
const SEARCH_DEBOUNCE: Duration = Duration::from_millis(500);
/// Placeholder text of the search entry that must never be treated as a query.
const SEARCH_PLACEHOLDER_VALUE: &str = "ng-native-dev";

#[derive(Default)]
struct UiNodes {
    list_view: Option<NodeId>,
    detail_view: Option<NodeId>,
    spinner: Option<NodeId>,
    name: Option<NodeId>,
    stats_label: Option<NodeId>,
    number: Option<NodeId>,
    sprite: Option<NodeId>,
    type_primary: Option<NodeId>,
    type_secondary: Option<NodeId>,
    height: Option<NodeId>,
    weight: Option<NodeId>,
    hp_bar: Option<NodeId>,
    atk_bar: Option<NodeId>,
    def_bar: Option<NodeId>,
    spd_bar: Option<NodeId>,
}

impl UiNodes {
    fn resolve(runtime: &JsRuntime) -> Self {
        let node = |var: &str| runtime.resolve_var(var).ok();
        Self {
            list_view: node("list_view"),
            detail_view: node("detail_view"),
            spinner: node("loading_spinner"),
            name: node("hello"),
            stats_label: node("lbl_stats"),
            number: node("poke_num"),
            sprite: node("poke_sprite"),
            type_primary: node("poke_type_primary"),
            type_secondary: node("poke_type_secondary"),
            height: node("lbl_height"),
            weight: node("lbl_weight"),
            hp_bar: node("hp_bar"),
            atk_bar: node("atk_bar"),
            def_bar: node("def_bar"),
            spd_bar: node("spd_bar"),
        }
    }
}

/// Drives list/detail navigation and async data loading for the Pokédex example.
pub struct PokedexController {
    nodes: UiNodes,
    /// Last observed value of the `pokemonId` signal (0 = list view).
    last_signal: i64,
    /// In-flight request; replacing it cancels interest in the previous result.
    pending: Option<Receiver<FetchResult>>,
    /// Search text waiting for the debounce window to elapse.
    queued_search: Option<(String, Instant)>,
}

impl PokedexController {
    pub fn new(runtime: &JsRuntime) -> Self {
        Self {
            nodes: UiNodes::resolve(runtime),
            last_signal: 0,
            pending: None,
            queued_search: None,
        }
    }

    /// Re-binds node handles and view state after a hot reload re-created the scene.
    pub fn refresh_after_reload(&mut self, engine: &mut Engine, runtime: &JsRuntime) {
        self.nodes = UiNodes::resolve(runtime);
        self.last_signal = runtime.signal_value("pokemonId").unwrap_or(0);
        self.set_view(engine, self.last_signal != 0);
    }

    pub fn show_list(&self, engine: &mut Engine) {
        self.set_view(engine, false);
    }

    /// Queues a search; the request fires once typing pauses (see [`SEARCH_DEBOUNCE`]).
    pub fn queue_search(&mut self, query: &str) {
        let clean = query.trim();
        if clean.is_empty() || clean == SEARCH_PLACEHOLDER_VALUE {
            self.queued_search = None;
        } else {
            self.queued_search = Some((clean.to_string(), Instant::now()));
        }
    }

    /// Advances time-based and signal-driven state. Call once per event-loop iteration.
    pub fn tick(&mut self, engine: &mut Engine, runtime: &mut JsRuntime) {
        if let Some((query, queued_at)) = &self.queued_search
            && queued_at.elapsed() >= SEARCH_DEBOUNCE
        {
            let query = query.clone();
            self.queued_search = None;
            self.start_fetch(engine, &query);
        }
        self.sync_navigation(engine, runtime);
        self.poll_fetch(engine, runtime);
    }

    fn sync_navigation(&mut self, engine: &mut Engine, runtime: &JsRuntime) {
        let current = runtime.signal_value("pokemonId").unwrap_or(0);
        if current == self.last_signal {
            return;
        }
        self.last_signal = current;
        if current == 0 {
            self.pending = None;
            self.set_loading(engine, false);
            self.set_view(engine, false);
        } else {
            self.start_fetch(engine, &current.to_string());
        }
    }

    fn start_fetch(&mut self, engine: &mut Engine, query: &str) {
        self.set_loading(engine, true);
        let (tx, rx) = mpsc::channel();
        let query = query.to_string();
        thread::spawn(move || {
            let _ = tx.send(DesktopServices::new().fetch_pokemon(&query));
        });
        self.pending = Some(rx);
    }

    fn poll_fetch(&mut self, engine: &mut Engine, runtime: &mut JsRuntime) {
        let Some(rx) = &self.pending else { return };
        let outcome = match rx.try_recv() {
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(String::from("fetch worker stopped")),
            Ok(result) => result,
        };
        self.pending = None;
        self.set_loading(engine, false);
        match outcome {
            Ok(poke) => {
                println!(
                    "\n[PokéAPI] {} {} ({})",
                    poke.format_id(),
                    poke.display_name(),
                    poke.primary_type
                );
                self.render(engine, &poke);
                self.set_view(engine, true);
                self.last_signal = i64::from(poke.id);
                let _ = runtime.set_signal(engine, "pokemonId", i64::from(poke.id));
            }
            Err(err) => eprintln!("[PokéAPI] {err}"),
        }
    }

    fn set_view(&self, engine: &mut Engine, show_detail: bool) {
        for (node, visible) in [
            (self.nodes.list_view, !show_detail),
            (self.nodes.detail_view, show_detail),
        ] {
            if let Some(node) = node {
                let _ = engine.set_attribute(node, "visible", visible.to_string());
            }
        }
    }

    fn set_loading(&self, engine: &mut Engine, loading: bool) {
        if let Some(spinner) = self.nodes.spinner {
            let _ = engine.set_attribute(spinner, "visible", loading.to_string());
        }
    }

    fn render(&self, engine: &mut Engine, poke: &PokemonInfo) {
        let nodes = &self.nodes;
        let texts = [
            (nodes.name, poke.display_name().to_uppercase()),
            (nodes.number, poke.format_id()),
            (nodes.sprite, poke.sprite_path.clone()),
            (nodes.type_primary, poke.primary_type.to_uppercase()),
            (
                nodes.type_secondary,
                poke.secondary_type.as_deref().unwrap_or("").to_uppercase(),
            ),
            (nodes.height, poke.format_height()),
            (nodes.weight, poke.format_weight()),
            (
                nodes.stats_label,
                format!(
                    "HP: {} | ATK: {} | DEF: {} | SPD: {}",
                    poke.hp, poke.attack, poke.defense, poke.speed
                ),
            ),
        ];
        for (node, text) in texts {
            if let Some(node) = node {
                let _ = engine.set_text(node, text);
            }
        }
        let bars = [
            (nodes.hp_bar, poke.hp),
            (nodes.atk_bar, poke.attack),
            (nodes.def_bar, poke.defense),
            (nodes.spd_bar, poke.speed),
        ];
        for (node, value) in bars {
            if let Some(node) = node {
                let _ = engine.set_attribute(node, "value", value.to_string());
            }
        }
    }
}

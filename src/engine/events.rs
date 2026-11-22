use super::node::NodeId;

/// Bitmask of DOM/Desktop event categories enabled on a native widget node.
/// Passive widgets keep `EventMask::NONE` (`0`) so OS toolkits attach zero extra controllers until subscribed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct EventMask(pub u32);

impl EventMask {
    pub const NONE: Self = Self(0);
    pub const CLICK: Self = Self(1 << 0);
    pub const CONTEXT_MENU: Self = Self(1 << 1);
    pub const POINTER_MOTION: Self = Self(1 << 2);
    pub const FOCUS: Self = Self(1 << 3);
    pub const KEYBOARD: Self = Self(1 << 4);
    pub const ACTIVATE: Self = Self(1 << 5);
    pub const VALUE_CHANGE: Self = Self(1 << 6);
    pub const SCROLL: Self = Self(1 << 7);
    pub const WHEEL: Self = Self(1 << 8);
    pub const WINDOW_STATE: Self = Self(1 << 9);
    pub const DRAG_DROP: Self = Self(1 << 10);
    pub const CLOSE_REQUEST: Self = Self(1 << 11);

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Returns only the bits newly requested in `next` that were not yet active in `prev`.
    #[must_use]
    pub const fn newly_enabled(prev: Self, next: Self) -> Self {
        Self(next.0 & !prev.0)
    }

    #[must_use]
    pub fn from_event_name(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "click" | "clicked" | "dblclick" | "doubleclick" | "mousedown" | "mouseup"
            | "pointerdown" | "pointerup" => Self::CLICK,
            "contextmenu" | "rightclick" => Self::CONTEXT_MENU,
            "motion" | "mouseenter" | "mouseleave" | "mousemove" | "mouseover" | "mouseout"
            | "pointerenter" | "pointerleave" | "pointermove" | "hover" => Self::POINTER_MOTION,
            "focus" | "blur" | "focusin" | "focusout" => Self::FOCUS,
            "keyboard" | "keydown" | "keyup" => Self::KEYBOARD,
            "activate" | "submit" | "search" | "cancel" | "stop-search" => Self::ACTIVATE,
            "input" | "change" | "changed" | "value-changed" | "toggled" => Self::VALUE_CHANGE,
            "scroll" | "scroll-edge" | "scroll-top" | "scroll-bottom" | "edge-reached"
            | "viewport-reached" => Self::SCROLL,
            "wheel" | "mousewheel" => Self::WHEEL,
            "resize" | "maximize" => Self::WINDOW_STATE,
            "close-request" | "closerequest" | "beforeunload" => Self::CLOSE_REQUEST,
            "dragenter" | "dragleave" | "drop" | "files-dropped" => Self::DRAG_DROP,
            _ => Self::NONE,
        }
    }

    #[must_use]
    pub fn parse_attr(raw: &str) -> Self {
        let trimmed = raw.trim();
        if let Ok(bits) = trimmed.parse::<u32>() {
            return Self(bits);
        }
        let mut mask = Self::NONE;
        for token in trimmed.split([',', ' ', '\t']).filter(|s| !s.is_empty()) {
            mask = mask.union(Self::from_event_name(token));
        }
        mask
    }
}

/// Returns true for continuous high-frequency events whose latest sample supersedes earlier samples in the same frame.
#[must_use]
pub fn is_coalescible_event(event_name: &str) -> bool {
    matches!(event_name, "mousemove" | "scroll" | "wheel" | "resize")
}

/// Deduplicates high-frequency continuous events (`mousemove`, `scroll`, `wheel`, `resize`) per `(NodeId, event_name)`
/// within a single frame while preserving exact FIFO ordering of discrete user actions (`clicked`, `keydown`, etc.).
#[must_use]
pub fn coalesce_frame_events(events: Vec<NativeEvent>) -> Vec<NativeEvent> {
    if events.len() <= 1 {
        return events;
    }
    let mut coalesced: Vec<NativeEvent> = Vec::with_capacity(events.len());
    for ev in events {
        if is_coalescible_event(&ev.event_name)
            && let Some(pos) = coalesced.iter().rposition(|existing| {
                existing.node == ev.node && existing.event_name == ev.event_name
            })
        {
            coalesced[pos].payload = ev.payload;
        } else {
            coalesced.push(ev);
        }
    }
    coalesced
}

/// Typed payload carried by a native OS widget signal (`changed`, `toggled`, `value-changed`, `clicked`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventPayload {
    None,
    Text(String),
    Bool(bool),
    Int(i64),
}

/// Complete native UI event dispatched from an OS backend into `RenderEngine` and `JsRuntime`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeEvent {
    pub node: NodeId,
    pub event_name: String,
    pub payload: EventPayload,
}

#[derive(Debug, Clone)]
pub struct EventListener {
    pub listener_id: u64,
    pub node: NodeId,
    pub event_name: String,
}

/// Manages event subscriptions between native widget signals and Angular callbacks.
#[derive(Default)]
pub struct EventRegistry {
    listeners: Vec<EventListener>,
}

impl EventRegistry {
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }

    pub fn register(&mut self, node: NodeId, event_name: impl Into<String>, listener_id: u64) {
        self.listeners.push(EventListener {
            listener_id,
            node,
            event_name: event_name.into(),
        });
    }

    pub fn unregister(&mut self, listener_id: u64) -> bool {
        let initial_len = self.listeners.len();
        self.listeners.retain(|l| l.listener_id != listener_id);
        self.listeners.len() < initial_len
    }

    pub fn remove_for_node(&mut self, node: NodeId) {
        self.listeners.retain(|l| l.node != node);
    }

    pub fn clear(&mut self) {
        self.listeners.clear();
    }

    pub fn matching_listeners(&self, node: NodeId, event_name: &str) -> Vec<u64> {
        self.listeners
            .iter()
            .filter(|l| l.node == node && l.event_name == event_name)
            .map(|l| l.listener_id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_filters_and_cleans_up_node_listeners() {
        let mut events = EventRegistry::new();
        let n1 = NodeId {
            index: 1,
            generation: 1,
        };
        let n2 = NodeId {
            index: 2,
            generation: 1,
        };

        events.register(n1, "clicked", 10);
        events.register(n1, "clicked", 11);
        events.register(n2, "clicked", 20);

        assert_eq!(events.matching_listeners(n1, "clicked"), vec![10, 11]);
        assert!(events.unregister(10));
        assert_eq!(events.matching_listeners(n1, "clicked"), vec![11]);

        events.remove_for_node(n1);
        assert_eq!(events.matching_listeners(n1, "clicked"), Vec::<u64>::new());
        assert_eq!(events.matching_listeners(n2, "clicked"), vec![20]);
    }

    #[test]
    fn parses_event_masks_computes_deltas_and_coalesces_continuous_events() {
        let m1 = EventMask::parse_attr("click,motion");
        assert!(m1.contains(EventMask::CLICK));
        assert!(m1.contains(EventMask::POINTER_MOTION));
        assert!(!m1.contains(EventMask::KEYBOARD));

        let m2 = EventMask::parse_attr("click,keyboard,wheel,close-request");
        let delta = EventMask::newly_enabled(m1, m2);
        assert!(!delta.contains(EventMask::CLICK));
        assert!(delta.contains(EventMask::KEYBOARD));
        assert!(delta.contains(EventMask::WHEEL));
        assert!(delta.contains(EventMask::CLOSE_REQUEST));
        assert_eq!(
            EventMask::from_event_name("beforeunload"),
            EventMask::CLOSE_REQUEST
        );

        assert_eq!(EventMask::parse_attr("17"), EventMask(17));

        let n1 = NodeId {
            index: 1,
            generation: 1,
        };
        let mut frame = Vec::new();
        for i in 0..100 {
            frame.push(NativeEvent {
                node: n1,
                event_name: String::from("mousemove"),
                payload: EventPayload::Int(i),
            });
            if i == 25 {
                frame.push(NativeEvent {
                    node: n1,
                    event_name: String::from("clicked"),
                    payload: EventPayload::None,
                });
            }
        }
        frame.push(NativeEvent {
            node: n1,
            event_name: String::from("resize"),
            payload: EventPayload::Int(800),
        });
        frame.push(NativeEvent {
            node: n1,
            event_name: String::from("resize"),
            payload: EventPayload::Int(1200),
        });

        let out = coalesce_frame_events(frame);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].event_name, "mousemove");
        assert_eq!(out[0].payload, EventPayload::Int(99));
        assert_eq!(out[1].event_name, "clicked");
        assert_eq!(out[2].event_name, "resize");
        assert_eq!(out[2].payload, EventPayload::Int(1200));
    }
}

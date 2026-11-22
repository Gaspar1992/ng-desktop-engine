use ng_native_engine::engine::{
    EngineError, EventPayload, NodeId, RenderCommand, RenderEngine, StyleRule,
};
use ng_native_engine::platforms::{GtkLinuxBackend, HeadlessBackend, WinUiBackend};
use ng_native_engine::runtime::{JsEngineBackend, JsRuntime};

#[test]
fn phase2_scene_graph_containers_attributes_and_anchor_operations() {
    let mut engine = RenderEngine::new(HeadlessBackend::new());
    let mut runtime = JsRuntime::new();

    let script = r"
        create win ng-window
        text win NgNative Phase 2
        create hdr ng-header-bar
        text hdr Panel Principal
        attr hdr subtitle Libadwaita + GTK4
        create scroll ng-scroll
        attr scroll min-height 320px
        create root_box ng-box
        attr root_box orientation vertical
        attr root_box spacing 16px
        attr root_box hexpand true
        attr root_box vexpand true
        attr root_box halign center
        attr root_box valign start
        attr root_box margin 12px
        create grid ng-grid
        attr grid row-spacing 8px
        attr grid column-spacing 12px
        create sep ng-separator
        attr sep orientation horizontal
        comment anchor_if @if(showExtra)
        create lbl_fixed ng-label
        text lbl_fixed Elemento Fijo
        append win hdr
        append win scroll
        append scroll root_box
        append root_box grid
        append root_box sep
        append root_box anchor_if
        append root_box lbl_fixed
    ";

    runtime.eval_tick(&mut engine, script).unwrap();

    let win = runtime.resolve_var("win").unwrap();
    let hdr = runtime.resolve_var("hdr").unwrap();
    let scroll = runtime.resolve_var("scroll").unwrap();
    let root_box = runtime.resolve_var("root_box").unwrap();

    assert_eq!(
        engine.get_attribute(hdr, "subtitle"),
        Some("Libadwaita + GTK4")
    );
    assert_eq!(engine.get_attribute(scroll, "min-height"), Some("320px"));
    assert_eq!(engine.get_attribute(root_box, "halign"), Some("center"));
    assert_eq!(engine.get_attribute(root_box, "margin"), Some("12px"));

    // Dynamically insert a node before the `@if` comment anchor, then remove it
    let dynamic_insert = r"
        create lbl_dyn ng-label
        text lbl_dyn Insertado via @if
        insert_before root_box lbl_dyn anchor_if
    ";
    runtime.eval_tick(&mut engine, dynamic_insert).unwrap();

    let preview_with_dyn = engine.render_window_preview(win).unwrap();
    assert!(preview_with_dyn.contains("[HeaderBar: Panel Principal — Libadwaita + GTK4]"));
    assert!(preview_with_dyn.contains("[Grid row-gap:8px col-gap:12px]"));
    assert!(preview_with_dyn.contains("Insertado via @if"));

    let dyn_idx = preview_with_dyn.find("Insertado via @if").unwrap();
    let fixed_idx = preview_with_dyn.find("Elemento Fijo").unwrap();
    assert!(
        dyn_idx < fixed_idx,
        "Expected dynamically inserted node before anchor to render ahead of trailing sibling"
    );

    runtime
        .eval_tick(&mut engine, "remove_child root_box lbl_dyn")
        .unwrap();
    let preview_after_remove = engine.render_window_preview(win).unwrap();
    assert!(!preview_after_remove.contains("Insertado via @if"));
    assert!(preview_after_remove.contains("Elemento Fijo"));
}

#[test]
fn node_based_event_dispatch_resolves_js_handles_through_reverse_index() {
    let js_bundle = r#"
        __NG_NATIVE__.createElement("win", "ng-window");
        __NG_NATIVE__.createElement("entry", "ng-entry");
        __NG_NATIVE__.createElement("echo", "ng-label");
        __NG_NATIVE__.appendChild("win", "entry");
        __NG_NATIVE__.listen("entry", "input", function(evt) {
            __NG_NATIVE__.setText("echo", "Echo: " + evt.value);
        });
    "#;

    let mut engine = RenderEngine::new(HeadlessBackend::new());
    let mut runtime = JsRuntime::from_engine_config("jsc").unwrap();
    runtime.eval_js(&mut engine, js_bundle).unwrap();

    let entry = runtime.resolve_var("entry").unwrap();
    runtime
        .trigger_node_event_with_payload(
            &mut engine,
            entry,
            "input",
            &EventPayload::Text(String::from("ReverseIndex")),
        )
        .unwrap();

    if runtime.js_engine().is_jit_enabled() {
        assert!(engine.backend().frames.iter().flatten().any(|cmd| matches!(
            cmd,
            RenderCommand::SetText { text, .. } if text == "Echo: ReverseIndex"
        )));
    }

    // Destroyed handles must no longer route node-based events to JS listeners.
    runtime.eval_tick(&mut engine, "destroy entry").unwrap();
    assert!(runtime.resolve_var("entry").is_err());
}

#[test]
fn scene_graph_prevents_cycles_and_cleans_up_orphaned_listeners() {
    let mut engine = RenderEngine::new(HeadlessBackend::new());
    let mut runtime = JsRuntime::new();

    let script = r"
        create win ng-window
        create box1 ng-box
        create box2 ng-box
        append win box1
        append box1 box2
    ";
    runtime.eval_tick(&mut engine, script).unwrap();

    let win = runtime.resolve_var("win").unwrap();
    let box1 = runtime.resolve_var("box1").unwrap();
    let box2 = runtime.resolve_var("box2").unwrap();

    assert!(engine.get_attribute(box1, "nonexistent").is_none());

    // Destroy intermediate container: un-detached descendants are destroyed recursively.
    runtime.eval_tick(&mut engine, "destroy box1").unwrap();
    assert!(runtime.resolve_var("box1").is_err());
    assert!(runtime.resolve_var("box2").is_err());
    assert!(matches!(
        engine.set_attribute(box2, "spacing", "4px"),
        Err(EngineError::StaleGeneration { .. })
    ));

    let preview = engine.render_window_preview(win).unwrap();
    assert!(!preview.contains("box1"));
}

#[test]
fn phase8_paned_stack_and_expander_containers_render_in_scene_graph() {
    let mut engine = RenderEngine::new(HeadlessBackend::new());
    let mut runtime = JsRuntime::new();

    let script = r"
        create win ng-window
        text win Phase 8 Containers
        create split ng-paned
        attr split orientation horizontal
        attr split position 240px
        create stack ng-stack
        attr stack visible-child page_0
        attr stack transition slide-left
        create exp ng-expander
        attr exp label Details
        attr exp expanded true
        create lbl_left ng-label
        text lbl_left Sidebar
        create lbl_page ng-label
        text lbl_page StackContent
        create lbl_inner ng-label
        text lbl_inner HiddenOrShown
        append exp lbl_inner
        append stack lbl_page
        append stack exp
        append split lbl_left
        append split stack
        append win split
    ";
    runtime.eval_tick(&mut engine, script).unwrap();

    let win = runtime.resolve_var("win").unwrap();
    let split = runtime.resolve_var("split").unwrap();
    let stack = runtime.resolve_var("stack").unwrap();
    let exp = runtime.resolve_var("exp").unwrap();

    assert_eq!(engine.get_attribute(split, "position"), Some("240px"));
    assert_eq!(engine.get_attribute(stack, "visible-child"), Some("page_0"));
    assert_eq!(engine.get_attribute(exp, "expanded"), Some("true"));

    let preview = engine.render_window_preview(win).unwrap();
    assert!(preview.contains("[Paned dir:horizontal, pos:240px]"));
    assert!(preview.contains("[Stack active:\"page_0\", transition:slide-left]"));
    assert!(preview.contains("[Expander ▼ Details]"));
    assert!(preview.contains("Sidebar"));
    assert!(preview.contains("StackContent"));
    assert!(preview.contains("HiddenOrShown"));
}

fn build_test_subtree<B: ng_native_engine::platforms::PlatformBackend>(
    engine: &mut RenderEngine<B>,
) -> (NodeId, NodeId, NodeId, NodeId, NodeId, NodeId) {
    let win = engine.create_element("ng-window").unwrap();
    let container = engine.create_element("ng-box").unwrap();
    let flow = engine.create_element("ng-flow").unwrap();
    let lbl = engine.create_element("ng-label").unwrap();
    let btn = engine.create_element("ng-button").unwrap();
    let detached = engine.create_element("ng-label").unwrap();

    engine.append_child(win, container).unwrap();
    engine.append_child(container, flow).unwrap();
    engine.append_child(flow, lbl).unwrap();
    engine.append_child(container, btn).unwrap();
    engine.append_child(container, detached).unwrap();
    engine.remove_child(container, detached).unwrap();

    engine.register_style(StyleRule {
        class_name: String::from("tag"),
        background: Some(String::from("#222222")),
        foreground: Some(String::from("#ffffff")),
        padding: 4,
        spacing: 2,
        ..StyleRule::default()
    });
    engine.add_class(lbl, "tag").unwrap();
    engine.set_attribute(lbl, "wrap", "true").unwrap();
    engine.listen(lbl, "clicked", 77).unwrap();
    engine.flush_frame();

    (win, container, flow, lbl, btn, detached)
}

#[test]
fn destroy_node_recursively_cleans_subtrees_and_rejects_borrowed_descendant() {
    let mut engine = RenderEngine::new(HeadlessBackend::new());
    let (_win, container, flow, lbl, btn, detached) = build_test_subtree(&mut engine);

    // Borrowing a nested descendant makes destroy_node(container) fail atomically.
    engine
        .emit_event(lbl, "clicked", |eng, lid| {
            assert_eq!(lid, 77);
            assert!(matches!(
                eng.destroy_node(container),
                Err(EngineError::BorrowConflict { id, .. }) if id == lbl
            ));
        })
        .unwrap();
    assert!(engine.set_attribute(container, "spacing", "8px").is_ok());

    engine.destroy_node(container).unwrap();
    engine.flush_frame();

    for gone in [container, flow, lbl, btn] {
        assert!(matches!(
            engine.set_attribute(gone, "id", "x"),
            Err(EngineError::StaleGeneration { .. })
        ));
    }
    assert!(engine.set_attribute(detached, "text", "kept").is_ok());

    let destroyed_ids: Vec<NodeId> = engine
        .backend()
        .frames
        .last()
        .unwrap()
        .iter()
        .filter_map(|cmd| match cmd {
            RenderCommand::DestroyWidget { id } => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(destroyed_ids.len(), 4);
    assert_eq!(destroyed_ids.last().copied(), Some(container));

    // Verify backend maps also purge descendants for both GtkLinuxBackend and WinUiBackend.
    let mut gtk_engine = RenderEngine::new(GtkLinuxBackend::new_headless());
    let (_, g_box, _, _, _, _) = build_test_subtree(&mut gtk_engine);
    gtk_engine.destroy_node(g_box).unwrap();
    gtk_engine.flush_frame();
    assert_eq!(gtk_engine.backend().widget_count(), 2);

    let mut winui_engine = RenderEngine::new(WinUiBackend::new());
    let (_, w_box, _, _, _, _) = build_test_subtree(&mut winui_engine);
    winui_engine.destroy_node(w_box).unwrap();
    winui_engine.flush_frame();
    assert_eq!(winui_engine.backend().control_count(), 2);

    // Destroying a child after its parent subtree was destroyed is idempotent in JsRuntime.
    let mut rt_engine = RenderEngine::new(HeadlessBackend::new());
    let mut runtime = JsRuntime::new();
    runtime
        .eval_tick(
            &mut rt_engine,
            "create p ng-box\ncreate c ng-label\nappend p c\ndestroy p\ndestroy c\n",
        )
        .unwrap();
    assert!(runtime.resolve_var("c").is_err());
}

#[test]
fn gtk_backend_coalesces_css_reloads_per_flush() {
    let mut engine = RenderEngine::new(GtkLinuxBackend::new_headless());
    for idx in 0..8 {
        engine.register_style(StyleRule {
            class_name: format!("c_{idx}"),
            background: Some(String::from("#101010")),
            foreground: None,
            padding: idx,
            spacing: 0,
            ..StyleRule::default()
        });
    }
    engine.flush_frame();
    assert_eq!(engine.backend().css_reload_count(), 1);

    let _lbl = engine.create_element("ng-label").unwrap();
    engine.flush_frame();
    assert_eq!(engine.backend().css_reload_count(), 1);

    engine.register_style(StyleRule {
        class_name: String::from("c_extra"),
        background: None,
        foreground: Some(String::from("#00ff88")),
        padding: 2,
        spacing: 0,
        ..StyleRule::default()
    });
    engine.flush_frame();
    assert_eq!(engine.backend().css_reload_count(), 2);
}

fn exercise_reorder_and_container_removal(backend: GtkLinuxBackend) {
    let mut engine = RenderEngine::new(backend);
    let win = engine.create_element("ng-window").unwrap();
    let hdr = engine.create_element("ng-header-bar").unwrap();
    let hdr_btn = engine.create_element("ng-button").unwrap();
    let scroll = engine.create_element("ng-scroll").unwrap();
    let root_box = engine.create_element("ng-box").unwrap();
    let flow = engine.create_element("ng-flow").unwrap();
    let grid = engine.create_element("ng-grid").unwrap();
    let g1 = engine.create_element("ng-label").unwrap();
    let b1 = engine.create_element("ng-label").unwrap();
    let b2 = engine.create_element("ng-label").unwrap();
    let f1 = engine.create_element("ng-button").unwrap();
    let f2 = engine.create_element("ng-button").unwrap();
    let f3 = engine.create_element("ng-button").unwrap();

    engine.append_child(hdr, hdr_btn).unwrap();
    engine.append_child(win, hdr).unwrap();
    engine.append_child(win, scroll).unwrap();
    engine.append_child(scroll, root_box).unwrap();
    engine.append_child(grid, g1).unwrap();
    engine.append_child(root_box, flow).unwrap();
    engine.append_child(root_box, grid).unwrap();
    engine.append_child(root_box, b1).unwrap();
    engine.append_child(root_box, b2).unwrap();
    engine.append_child(flow, f1).unwrap();
    engine.append_child(flow, f2).unwrap();
    engine.append_child(flow, f3).unwrap();
    engine.flush_frame();

    // Reorder existing children in both Box and FlowBox (start, middle, end).
    engine.insert_before(root_box, b2, Some(b1)).unwrap();
    engine.insert_before(flow, f3, Some(f1)).unwrap();
    engine.insert_before(flow, f1, None).unwrap();
    engine.insert_before(flow, f2, Some(f3)).unwrap();
    engine.flush_frame();

    assert_eq!(
        engine.backend().widget_children(root_box),
        Some([flow, grid, b2, b1].as_slice())
    );
    assert_eq!(
        engine.backend().widget_children(flow),
        Some([f2, f3, f1].as_slice())
    );

    // Exercise remove_child across every container type, then destroy the window subtree.
    for child in [f1, f2, f3] {
        engine.remove_child(flow, child).unwrap();
    }
    engine.remove_child(grid, g1).unwrap();
    for child in [flow, grid, b1, b2] {
        engine.remove_child(root_box, child).unwrap();
    }
    engine.remove_child(hdr, hdr_btn).unwrap();
    engine.remove_child(scroll, root_box).unwrap();
    engine.remove_child(win, hdr).unwrap();
    engine.remove_child(win, scroll).unwrap();
    engine.flush_frame();

    assert_eq!(engine.backend().widget_children(flow), Some(&[][..]));
    assert_eq!(engine.backend().widget_children(grid), Some(&[][..]));
    assert_eq!(engine.backend().widget_children(root_box), Some(&[][..]));

    engine.destroy_node(win).unwrap();
    engine.flush_frame();
    assert_eq!(engine.backend().widget_count(), 12);

    for id in [
        hdr, hdr_btn, scroll, root_box, flow, grid, g1, b1, b2, f1, f2, f3,
    ] {
        engine.destroy_node(id).unwrap();
    }
    engine.flush_frame();
    assert_eq!(engine.backend().widget_count(), 0);
}

static LIVE_GTK_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn gtk_backend_reorders_box_and_flow_box_and_removes_all_containers_safely() {
    exercise_reorder_and_container_removal(GtkLinuxBackend::new_headless());
    let _guard = LIVE_GTK_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let live = GtkLinuxBackend::new();
    if live.is_native_display_active() {
        exercise_reorder_and_container_removal(live);
        let mut live_paned = RenderEngine::new(GtkLinuxBackend::new());
        exercise_paned_insert_before_shifts_start_child_to_end_without_uaf(&mut live_paned);
    }
}

fn exercise_paned_insert_before_shifts_start_child_to_end_without_uaf(
    engine: &mut RenderEngine<GtkLinuxBackend>,
) {
    let win = engine.create_element("ng-window").unwrap();
    let paned = engine.create_element("ng-paned").unwrap();
    let right_pane = engine.create_element("ng-box").unwrap();
    let stats_expander = engine.create_element("ng-expander").unwrap();
    let stats_box = engine.create_element("ng-box").unwrap();
    let notes_expander = engine.create_element("ng-expander").unwrap();
    let notes_text = engine.create_element("ng-text-view").unwrap();
    let anchor = engine.create_comment("stats_anchor");

    engine
        .set_attribute(stats_expander, "expanded", "true")
        .unwrap();
    engine
        .set_attribute(notes_expander, "expanded", "true")
        .unwrap();

    engine.append_child(win, paned).unwrap();
    engine.append_child(paned, right_pane).unwrap();
    engine.append_child(right_pane, stats_expander).unwrap();
    engine.append_child(stats_expander, stats_box).unwrap();
    engine.append_child(stats_box, anchor).unwrap();
    engine.append_child(right_pane, notes_expander).unwrap();
    engine.append_child(notes_expander, notes_text).unwrap();
    engine.flush_frame();

    // Hydrate a left-hand pane via `insert_before` at index 0 before `right_pane`.
    let left_pane = engine.create_element("ng-box").unwrap();
    engine
        .insert_before(paned, left_pane, Some(right_pane))
        .unwrap();

    for idx in 0..6 {
        let lbl = engine.create_element("ng-label").unwrap();
        let bar = engine.create_element("ng-level-bar").unwrap();
        engine.insert_before(stats_box, lbl, Some(anchor)).unwrap();
        engine.insert_before(stats_box, bar, Some(anchor)).unwrap();
        engine.set_text(lbl, format!("Stat {idx}")).unwrap();
        engine.set_attribute(bar, "value", "45").unwrap();
    }
    engine
        .set_text(notes_text, "Field notes survived slot shift")
        .unwrap();
    engine.flush_frame();

    assert_eq!(
        engine.backend().widget_children(paned),
        Some([left_pane, right_pane].as_slice())
    );
    let _ = engine.backend().step_native_events(false);

    engine.destroy_node(win).unwrap();
    engine.flush_frame();
    assert_eq!(engine.backend().widget_count(), 0);
}

#[test]
fn gtk_backend_paned_insert_before_shifts_start_child_to_end_without_uaf() {
    let mut headless = RenderEngine::new(GtkLinuxBackend::new_headless());
    exercise_paned_insert_before_shifts_start_child_to_end_without_uaf(&mut headless);
}

#[test]
fn cross_platform_lazy_event_masks_and_frame_event_coalescing() {
    use ng_native_engine::engine::{EventMask, EventPayload, NativeEvent};
    use ng_native_engine::platforms::MacOsAppKitBackend;

    let mut gtk_eng = RenderEngine::new(GtkLinuxBackend::new_headless());
    let g_card = gtk_eng.create_element("ng-box").unwrap();
    let g_lbl = gtk_eng.create_element("ng-label").unwrap();
    gtk_eng.flush_frame();
    assert_eq!(
        gtk_eng.backend().attached_event_mask(g_card),
        EventMask::NONE
    );
    assert_eq!(
        gtk_eng.backend().attached_event_mask(g_lbl),
        EventMask::NONE
    );

    gtk_eng
        .set_attribute(g_card, "event-mask", "click,motion,keyboard,wheel")
        .unwrap();
    gtk_eng.flush_frame();
    let g_mask = gtk_eng.backend().attached_event_mask(g_card);
    assert!(g_mask.contains(EventMask::CLICK));
    assert!(g_mask.contains(EventMask::POINTER_MOTION));
    assert!(g_mask.contains(EventMask::KEYBOARD));
    assert!(g_mask.contains(EventMask::WHEEL));
    assert_eq!(
        gtk_eng.backend().attached_event_mask(g_lbl),
        EventMask::NONE
    );

    let mut win_eng = RenderEngine::new(WinUiBackend::new());
    let w_card = win_eng.create_element("ng-box").unwrap();
    win_eng
        .set_attribute(w_card, "event-mask", "click,contextmenu,wheel")
        .unwrap();
    win_eng.flush_frame();
    let w_binding = win_eng.backend().get_control_binding(w_card).unwrap();
    assert!(w_binding.event_mask.contains(EventMask::CONTEXT_MENU));
    assert!(
        w_binding
            .win32_event_routes
            .contains(&"WM_CONTEXTMENU/RightTapped")
    );
    assert!(
        w_binding
            .win32_event_routes
            .contains(&"WM_MOUSEWHEEL/PointerWheelChanged")
    );

    let mut mac_eng = RenderEngine::new(MacOsAppKitBackend::new());
    let m_card = mac_eng.create_element("ng-box").unwrap();
    mac_eng
        .set_attribute(m_card, "event-mask", "click,focus,keyboard")
        .unwrap();
    mac_eng.flush_frame();
    let m_binding = mac_eng.backend().get_view_binding(m_card).unwrap();
    assert!(m_binding.event_mask.contains(EventMask::KEYBOARD));
    assert!(m_binding.event_selectors.contains(&"mouseUp:"));
    assert!(m_binding.event_selectors.contains(&"becomeFirstResponder"));
    assert!(m_binding.event_selectors.contains(&"keyDown:"));

    for i in 0..50 {
        win_eng.backend().queue_native_event(NativeEvent {
            node: w_card,
            event_name: String::from("mousemove"),
            payload: EventPayload::Int(i),
        });
        mac_eng.backend().queue_native_event(NativeEvent {
            node: m_card,
            event_name: String::from("scroll"),
            payload: EventPayload::Int(i),
        });
    }
    let w_drained = win_eng.backend().step_native_events();
    let m_drained = mac_eng.backend().step_native_events();
    assert_eq!(w_drained.len(), 1);
    assert_eq!(w_drained[0].payload, EventPayload::Int(49));
    assert_eq!(m_drained.len(), 1);
    assert_eq!(m_drained[0].payload, EventPayload::Int(49));
}

#[test]
fn cross_platform_close_request_and_prevent_close_lifecycle() {
    use ng_native_engine::engine::EventMask;
    use ng_native_engine::platforms::MacOsAppKitBackend;

    let mut gtk_eng = RenderEngine::new(GtkLinuxBackend::new_headless());
    let g_win = gtk_eng.create_element("ng-window").unwrap();
    gtk_eng.flush_frame();
    assert!(!gtk_eng.backend().is_prevent_close(g_win));

    gtk_eng
        .set_attribute(g_win, "prevent-close", "true")
        .unwrap();
    gtk_eng.flush_frame();
    assert!(gtk_eng.backend().is_prevent_close(g_win));
    assert!(
        gtk_eng
            .backend()
            .attached_event_mask(g_win)
            .contains(EventMask::CLOSE_REQUEST)
    );
    gtk_eng
        .set_attribute(g_win, "close-window", "true")
        .unwrap();
    gtk_eng.flush_frame();
    assert!(!gtk_eng.backend().is_prevent_close(g_win));

    let mut win_eng = RenderEngine::new(WinUiBackend::new());
    let w_win = win_eng.create_element("ng-window").unwrap();
    win_eng
        .set_attribute(w_win, "prevent-close", "true")
        .unwrap();
    win_eng.flush_frame();
    let w_win_binding = win_eng.backend().get_control_binding(w_win).unwrap();
    assert!(w_win_binding.event_mask.contains(EventMask::CLOSE_REQUEST));
    assert!(
        w_win_binding
            .win32_event_routes
            .contains(&"WM_CLOSE/Closing")
    );
    assert!(win_eng.backend().get_control(w_win).unwrap().prevent_close);
    win_eng
        .set_attribute(w_win, "close-window", "true")
        .unwrap();
    win_eng.flush_frame();
    assert!(!win_eng.backend().get_control(w_win).unwrap().prevent_close);

    let mut mac_eng = RenderEngine::new(MacOsAppKitBackend::new());
    let m_win = mac_eng.create_element("ng-window").unwrap();
    mac_eng
        .set_attribute(m_win, "prevent-close", "true")
        .unwrap();
    mac_eng.flush_frame();
    let m_win_binding = mac_eng.backend().get_view_binding(m_win).unwrap();
    assert!(m_win_binding.event_mask.contains(EventMask::CLOSE_REQUEST));
    assert!(
        m_win_binding
            .event_selectors
            .contains(&"windowShouldClose:")
    );
    assert!(mac_eng.backend().get_view(m_win).unwrap().prevent_close);
}

#[test]
fn multi_window_and_native_dialog_lifecycle_across_backends() {
    use ng_native_engine::platforms::MacOsAppKitBackend;

    let mut linux = RenderEngine::new(GtkLinuxBackend::new_headless());
    let main_win = linux.create_element("ng-window").unwrap();
    linux.set_attribute(main_win, "id", "main_win").unwrap();
    let second_win = linux.create_element("ng-window").unwrap();
    let dlg = linux.create_element("ng-dialog").unwrap();
    linux.set_text(dlg, "Preferences").unwrap();
    linux.set_attribute(dlg, "modal", "true").unwrap();
    linux
        .set_attribute(dlg, "transient-for", "main_win")
        .unwrap();
    let dlg_lbl = linux.create_element("ng-label").unwrap();
    linux.set_text(dlg_lbl, "Audio Output").unwrap();
    linux.append_child(dlg, dlg_lbl).unwrap();
    linux.append_child(main_win, dlg).unwrap();
    linux.flush_frame();

    assert_eq!(linux.backend().toplevel_count(), 3);
    assert!(linux.backend().is_modal(dlg));
    assert_eq!(linux.backend().transient_for(dlg), Some("main_win"));
    let preview = linux.render_window_preview(dlg).unwrap();
    assert!(preview.contains("Preferences"));
    assert!(preview.contains("Audio Output"));

    linux.destroy_node(dlg).unwrap();
    linux.destroy_node(second_win).unwrap();
    linux.flush_frame();
    assert_eq!(linux.backend().toplevel_count(), 1);

    let mut win_eng = RenderEngine::new(WinUiBackend::new());
    let w_win = win_eng.create_element("ng-window").unwrap();
    let w_dlg = win_eng.create_element("ng-dialog").unwrap();
    win_eng
        .set_attribute(w_dlg, "transient-for", "win")
        .unwrap();
    win_eng.flush_frame();
    assert_eq!(win_eng.backend().toplevel_count(), 2);
    assert_eq!(
        win_eng.backend().get_control(w_dlg).unwrap().native_class,
        "Microsoft.UI.Xaml.Controls.ContentDialog"
    );
    assert!(win_eng.backend().get_control(w_dlg).unwrap().modal);
    assert!(win_eng.backend().get_control(w_win).is_some());

    let mut mac_eng = RenderEngine::new(MacOsAppKitBackend::new());
    let m_dlg = mac_eng.create_element("ng-dialog").unwrap();
    mac_eng.set_attribute(m_dlg, "modal", "false").unwrap();
    mac_eng.flush_frame();
    assert_eq!(mac_eng.backend().toplevel_count(), 1);
    assert_eq!(
        mac_eng.backend().get_view(m_dlg).unwrap().native_class,
        "NSPanel"
    );
    assert!(!mac_eng.backend().get_view(m_dlg).unwrap().modal);
}

#[test]
fn gtk_live_multi_window_and_dialog_presentation() {
    let _guard = LIVE_GTK_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let backend = GtkLinuxBackend::new();
    if !backend.is_native_display_active() {
        return;
    }
    let mut engine = RenderEngine::new(backend);
    let win = engine.create_element("ng-window").unwrap();
    engine.set_attribute(win, "id", "primary_win").unwrap();
    engine.set_text(win, "Main Window").unwrap();
    let bx = engine.create_element("ng-box").unwrap();
    engine.append_child(win, bx).unwrap();

    let dlg = engine.create_element("ng-dialog").unwrap();
    engine.set_text(dlg, "Live Modal").unwrap();
    engine.set_attribute(dlg, "modal", "true").unwrap();
    engine
        .set_attribute(dlg, "transient-for", "primary_win")
        .unwrap();
    let dlg_lbl = engine.create_element("ng-label").unwrap();
    engine.set_text(dlg_lbl, "Inside Modal").unwrap();
    engine.append_child(dlg, dlg_lbl).unwrap();
    // Appending a toplevel dialog under a container must never crash GTK4 box_append.
    engine.append_child(bx, dlg).unwrap();
    engine.flush_frame();

    assert_eq!(engine.backend().toplevel_count(), 2);
    assert!(engine.backend().open_window_count() >= 2);
    engine.destroy_node(dlg).unwrap();
    engine.flush_frame();
    assert_eq!(engine.backend().toplevel_count(), 1);
}

#[test]
fn list_view_and_popover_lifecycle_across_backends() {
    use ng_native_engine::platforms::MacOsAppKitBackend;

    let mut gtk_eng = RenderEngine::new(GtkLinuxBackend::new_headless());
    let win = gtk_eng.create_element("ng-window").unwrap();
    let root = gtk_eng.create_element("ng-box").unwrap();
    let trigger = gtk_eng.create_element("ng-button").unwrap();
    gtk_eng.set_attribute(trigger, "id", "filter_btn").unwrap();
    let list = gtk_eng.create_element("ng-table").unwrap();
    gtk_eng
        .set_attribute(list, "selection-mode", "single")
        .unwrap();
    gtk_eng
        .set_attribute(list, "show-separators", "true")
        .unwrap();
    let row_a = gtk_eng.create_element("ng-label").unwrap();
    gtk_eng.set_text(row_a, "Row A").unwrap();
    let row_b = gtk_eng.create_element("ng-label").unwrap();
    gtk_eng.set_text(row_b, "Row B").unwrap();
    gtk_eng.append_child(list, row_a).unwrap();
    gtk_eng.insert_before(list, row_b, Some(row_a)).unwrap();

    let pop = gtk_eng.create_element("ng-popover").unwrap();
    gtk_eng.set_attribute(pop, "anchor", "filter_btn").unwrap();
    gtk_eng.set_attribute(pop, "position", "bottom").unwrap();
    gtk_eng.set_attribute(pop, "autohide", "true").unwrap();
    gtk_eng.set_attribute(pop, "has-arrow", "true").unwrap();
    gtk_eng.set_attribute(pop, "open", "true").unwrap();
    let pop_lbl = gtk_eng.create_element("ng-label").unwrap();
    gtk_eng.set_text(pop_lbl, "Filter options").unwrap();
    gtk_eng.append_child(pop, pop_lbl).unwrap();

    gtk_eng.append_child(win, root).unwrap();
    gtk_eng.append_child(root, trigger).unwrap();
    gtk_eng.append_child(root, list).unwrap();
    gtk_eng.append_child(root, pop).unwrap();
    gtk_eng.flush_frame();

    assert_eq!(
        gtk_eng.backend().widget_children(list),
        Some([row_b, row_a].as_slice())
    );
    assert!(gtk_eng.backend().is_popover_open(pop));
    assert_eq!(gtk_eng.backend().popover_anchor(pop), Some("filter_btn"));
    gtk_eng.remove_child(list, row_b).unwrap();
    gtk_eng.destroy_node(row_b).unwrap();
    gtk_eng.flush_frame();
    assert_eq!(
        gtk_eng.backend().widget_children(list),
        Some([row_a].as_slice())
    );
    gtk_eng.destroy_node(win).unwrap();
    gtk_eng.flush_frame();
    assert_eq!(gtk_eng.backend().widget_count(), 0);

    let mut win_eng = RenderEngine::new(WinUiBackend::new());
    let w_list = win_eng.create_element("ng-list").unwrap();
    let w_pop = win_eng.create_element("ng-popover").unwrap();
    win_eng.flush_frame();
    assert_eq!(
        win_eng.backend().get_control(w_list).unwrap().native_class,
        "Microsoft.UI.Xaml.Controls.ListView"
    );
    assert_eq!(
        win_eng.backend().get_control(w_pop).unwrap().native_class,
        "Microsoft.UI.Xaml.Controls.Flyout"
    );

    let mut mac_eng = RenderEngine::new(MacOsAppKitBackend::new());
    let m_list = mac_eng.create_element("ng-list").unwrap();
    let m_pop = mac_eng.create_element("ng-menu").unwrap();
    mac_eng.flush_frame();
    assert_eq!(
        mac_eng.backend().get_view(m_list).unwrap().native_class,
        "NSTableView"
    );
    assert_eq!(
        mac_eng.backend().get_view(m_pop).unwrap().native_class,
        "NSPopover"
    );
}

#[test]
fn gtk_live_list_box_and_popover_lifecycle() {
    let _guard = LIVE_GTK_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let backend = GtkLinuxBackend::new();
    if !backend.is_native_display_active() {
        return;
    }
    let mut engine = RenderEngine::new(backend);
    let win = engine.create_element("ng-window").unwrap();
    let root = engine.create_element("ng-box").unwrap();
    let anchor_btn = engine.create_element("ng-button").unwrap();
    engine.set_attribute(anchor_btn, "id", "open_pop").unwrap();
    engine.set_text(anchor_btn, "Menu").unwrap();

    let list = engine.create_element("ng-list").unwrap();
    engine
        .set_attribute(list, "selection-mode", "single")
        .unwrap();
    engine
        .set_attribute(list, "show-separators", "true")
        .unwrap();
    let r1 = engine.create_element("ng-label").unwrap();
    let r2 = engine.create_element("ng-label").unwrap();
    let r3 = engine.create_element("ng-label").unwrap();
    engine.set_text(r1, "First").unwrap();
    engine.set_text(r2, "Second").unwrap();
    engine.set_text(r3, "Third").unwrap();
    engine.append_child(list, r1).unwrap();
    engine.append_child(list, r2).unwrap();
    engine.append_child(list, r3).unwrap();

    let pop = engine.create_element("ng-popover").unwrap();
    engine.set_attribute(pop, "anchor", "open_pop").unwrap();
    engine.set_attribute(pop, "position", "bottom").unwrap();
    engine.set_attribute(pop, "autohide", "true").unwrap();
    engine.set_attribute(pop, "has-arrow", "true").unwrap();
    let pop_content = engine.create_element("ng-label").unwrap();
    engine.set_text(pop_content, "Popover Body").unwrap();
    engine.append_child(pop, pop_content).unwrap();

    engine.append_child(root, anchor_btn).unwrap();
    engine.append_child(root, list).unwrap();
    engine.append_child(root, pop).unwrap();
    engine.append_child(win, root).unwrap();
    engine.flush_frame();

    // Reorder an existing row and remove a row in live GtkListBox, plus popup/popdown GtkPopover.
    engine.insert_before(list, r3, Some(r1)).unwrap();
    engine.remove_child(list, r2).unwrap();
    engine.destroy_node(r2).unwrap();
    engine.set_attribute(pop, "open", "true").unwrap();
    engine.flush_frame();
    let _ = engine.backend().step_native_events(false);

    engine.set_attribute(pop, "open", "false").unwrap();
    engine.flush_frame();
    let _ = engine.backend().step_native_events(false);

    assert_eq!(
        engine.backend().widget_children(list),
        Some([r3, r1].as_slice())
    );
    engine.destroy_node(win).unwrap();
    engine.flush_frame();
    assert_eq!(engine.backend().widget_count(), 0);
}

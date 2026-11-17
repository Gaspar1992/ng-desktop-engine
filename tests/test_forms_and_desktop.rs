use ng_native_engine::desktop::DesktopServices;
use ng_native_engine::engine::{EventPayload, RenderEngine};
use ng_native_engine::platforms::HeadlessBackend;
use ng_native_engine::runtime::{JsEngineBackend, JsRuntime};

#[test]
fn phase4_interactive_form_controls_and_typed_event_payloads() {
    let js_bundle = r#"
        __NG_NATIVE__.createElement("win", "ng-window");
        __NG_NATIVE__.setText("win", "Form Controls Window");
        __NG_NATIVE__.createElement("box", "ng-box");
        __NG_NATIVE__.createElement("entry", "ng-entry");
        __NG_NATIVE__.setAttribute("entry", "placeholder", "Username...");
        __NG_NATIVE__.setText("entry", "InitialUser");
        __NG_NATIVE__.createElement("sw", "ng-switch");
        __NG_NATIVE__.setAttribute("sw", "active", "false");
        __NG_NATIVE__.createElement("chk", "ng-checkbox");
        __NG_NATIVE__.setText("chk", "Accept license");
        __NG_NATIVE__.setAttribute("chk", "active", "true");
        __NG_NATIVE__.createElement("slider", "ng-slider");
        __NG_NATIVE__.setAttribute("slider", "min", "0");
        __NG_NATIVE__.setAttribute("slider", "max", "100");
        __NG_NATIVE__.setAttribute("slider", "value", "45");
        __NG_NATIVE__.createElement("sel", "ng-select");
        __NG_NATIVE__.setAttribute("sel", "options", "Gtk4,WinUI3,AppKit");
        __NG_NATIVE__.setAttribute("sel", "selected", "0");
        __NG_NATIVE__.createElement("summary", "ng-label");
        __NG_NATIVE__.setText("summary", "User: InitialUser | Dark: false | Vol: 45");
        __NG_NATIVE__.appendChild("box", "entry");
        __NG_NATIVE__.appendChild("box", "sw");
        __NG_NATIVE__.appendChild("box", "chk");
        __NG_NATIVE__.appendChild("box", "slider");
        __NG_NATIVE__.appendChild("box", "sel");
        __NG_NATIVE__.appendChild("box", "summary");
        __NG_NATIVE__.appendChild("win", "box");

        globalThis.__formState = { user: "InitialUser", dark: false, vol: 45 };
        function syncSummary() {
            const s = globalThis.__formState;
            __NG_NATIVE__.setText("summary", "User: " + s.user + " | Dark: " + s.dark + " | Vol: " + s.vol);
        }

        __NG_NATIVE__.listen("entry", "input", function(evt) {
            globalThis.__formState.user = evt.value;
            __NG_NATIVE__.setText("entry", evt.value);
            syncSummary();
        });
        __NG_NATIVE__.listen("sw", "toggled", function(evt) {
            globalThis.__formState.dark = evt.checked;
            __NG_NATIVE__.setAttribute("sw", "active", evt.checked ? "true" : "false");
            syncSummary();
        });
        __NG_NATIVE__.listen("slider", "value-changed", function(evt) {
            globalThis.__formState.vol = evt.value;
            __NG_NATIVE__.setAttribute("slider", "value", String(evt.value));
            syncSummary();
        });
    "#;

    let mut engine = RenderEngine::new(HeadlessBackend::new());
    let mut runtime = JsRuntime::from_engine_config("jsc").unwrap();
    runtime.eval_js(&mut engine, js_bundle).unwrap();

    let win = runtime.resolve_var("win").unwrap();
    let initial_preview = engine.render_window_preview(win).unwrap();
    assert!(initial_preview.contains("[Entry: \"InitialUser\"]"));
    assert!(initial_preview.contains("[Switch: OFF]"));
    assert!(initial_preview.contains("[x] Accept license"));
    assert!(initial_preview.contains("[Slider: 45 (0..100)]"));
    assert!(initial_preview.contains("[Select: \"Gtk4\" (0)]"));

    runtime
        .trigger_event_with_payload(
            &mut engine,
            "entry",
            "input",
            &EventPayload::Text(String::from("GraceHopper")),
        )
        .unwrap();
    runtime
        .trigger_event_with_payload(&mut engine, "sw", "toggled", &EventPayload::Bool(true))
        .unwrap();
    runtime
        .trigger_event_with_payload(
            &mut engine,
            "slider",
            "value-changed",
            &EventPayload::Int(90),
        )
        .unwrap();

    let updated_preview = engine.render_window_preview(win).unwrap();
    assert!(updated_preview.contains("User: GraceHopper | Dark: true | Vol: 90"));
    assert!(updated_preview.contains("[Entry: \"GraceHopper\"]"));
    assert!(updated_preview.contains("[Switch: ON]"));
    assert!(updated_preview.contains("[Slider: 90 (0..100)]"));
}

#[test]
fn phase5_desktop_services_and_native_http_bridge() {
    let js_bundle = r#"
        const cfg = __NG_NATIVE__.configureHttp({
            useFetchTransport: true,
            xsrfEnabled: true,
            xsrfCookieName: "NG-XSRF",
            xsrfHeaderName: "X-NG-XSRF",
            xsrfTokenValue: "tok-99",
            maxParallelConnections: 12,
            cacheEnabled: true
        });
        __NG_NATIVE__.registerHttpEndpoint("GET", "https://api.ng-native.dev/version", 200, '{"version":"0.5.0"}');
        __NG_NATIVE__.registerHttpEndpoint("POST", "https://beta.pokeapi.co/graphql/v1beta", 200, '{"data":{"ok":true}}');
        const res = __NG_NATIVE__.httpRequest("GET", "https://api.ng-native.dev/version", undefined, {
            headers: { "Accept": "application/json", "X-App": "pokedex" }
        });
        const batchRes = __NG_NATIVE__.httpBatchRequest([
            { method: "GET", url: "https://api.ng-native.dev/version" },
            { method: "POST", url: "https://beta.pokeapi.co/graphql/v1beta", body: '{"query":"{ok}"}' }
        ]);
        const parsed = JSON.parse(res.body);
        __NG_NATIVE__.setSystemTheme("light");
        __NG_NATIVE__.setClipboard("Clip:" + parsed.version);
        const clip = __NG_NATIVE__.getClipboard();
        const theme = __NG_NATIVE__.getSystemTheme();
        const opened = __NG_NATIVE__.openFileDialog("Open", "/etc/hostname");

        __NG_NATIVE__.createElement("win", "ng-window");
        __NG_NATIVE__.setText("win", "Phase 5 Desktop Window");
        __NG_NATIVE__.createElement("lbl", "ng-label");
        __NG_NATIVE__.setText("lbl", `Theme:${theme} | ${clip} | File:${opened} | Platform:${cfg.platform} | Gql:${batchRes[1].callKind}`);
        __NG_NATIVE__.appendChild("win", "lbl");
    "#;

    let mut engine = RenderEngine::new(HeadlessBackend::new());
    let mut runtime = JsRuntime::from_engine_config("jsc").unwrap();
    runtime.eval_js(&mut engine, js_bundle).unwrap();

    let win = runtime.resolve_var("win").unwrap();
    let preview = engine.render_window_preview(win).unwrap();
    assert!(preview.contains("Theme:light | Clip:0.5.0 | File:/etc/hostname"));
    assert_eq!(runtime.desktop().theme().as_str(), "light");
    assert_eq!(runtime.desktop().clipboard(), "Clip:0.5.0");

    if runtime.js_engine().is_jit_enabled() {
        assert!(
            preview.contains("Platform:linux-libsoup-3.0")
                || preview.contains("Platform:macos-")
                || preview.contains("Platform:windows-")
        );
        assert!(preview.contains("Gql:graphql"));
        let history = runtime.desktop().http_history();
        assert_eq!(history.len(), 3);
        assert_eq!(
            history[0].headers.get("X-App").map(String::as_str),
            Some("pokedex")
        );
        assert_eq!(
            history[2].headers.get("X-NG-XSRF").map(String::as_str),
            Some("tok-99")
        );
    }
}

#[test]
fn nivel1_native_sqlite_ffi_and_system_env() {
    let mut desktop = DesktopServices::new();
    desktop
        .sqlite_mut()
        .execute("CREATE TABLE IF NOT EXISTS teams (id INTEGER PRIMARY KEY, name TEXT);")
        .unwrap();
    desktop
        .sqlite_mut()
        .execute("INSERT INTO teams (id, name) VALUES (25, 'Pikachu');")
        .unwrap();

    if desktop.sqlite().is_native_loaded() {
        let json = desktop
            .sqlite()
            .query_json("SELECT id, name FROM teams ORDER BY id ASC;")
            .unwrap();
        assert_eq!(json, r#"[{"id":"25","name":"Pikachu"}]"#);
    }

    assert_ne!(desktop.system_env().locale(), "");
    assert_ne!(desktop.system_env().timezone(), "");
    assert_eq!(
        desktop.system_env().resolve_image_asset(
            "https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/official-artwork/6.png",
            Some(180)
        ),
        "/tmp/ng_native_pokedex/6.png"
    );

    desktop.system_env_mut().set_network_status(false, false);
    assert_eq!(desktop.system_env().connectivity(), "offline");
    desktop.system_env_mut().log_error("Timeout warning");
    assert_eq!(desktop.system_env().error_logs(), ["Timeout warning"]);
}

use ng_native_engine::config::AngularNativeConfig;
use ng_native_engine::desktop::PokedexController;
use ng_native_engine::engine::{NodeId, RenderEngine};
use ng_native_engine::ipc::{HmrIpcServer, HmrMessage, HmrResponse};
use ng_native_engine::packager::{stage_all_distributions, stage_linux_distribution};
use ng_native_engine::platforms::AnyPlatformBackend;
use ng_native_engine::runtime::{JsEngineBackend, JsRuntime};
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime};

fn bundle_modified_time(path: &str) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn handle_hmr_messages(
    server: &mut HmrIpcServer,
    engine: &mut RenderEngine<AnyPlatformBackend>,
    js_runtime: &mut JsRuntime,
    win_id: NodeId,
) -> bool {
    let mut reloaded = false;
    for (cid, msg) in server.poll_messages() {
        match msg {
            HmrMessage::Reload { script } => {
                if let Err(err) = js_runtime.hot_reload(engine, &script) {
                    eprintln!("[HMR IPC Error] Failed to reload bundle: {err:?}");
                    server.send_response(cid, &HmrResponse::Error(format!("{err:?}")));
                } else {
                    reloaded = true;
                    println!("\n[HMR IPC] Applied live update (signals preserved):");
                    println!("{}", engine.render_window_preview(win_id).unwrap());
                    server.send_response(cid, &HmrResponse::Ok { reloaded: true });
                }
            }
            HmrMessage::Eval { code } => match js_runtime.eval_js(engine, &code) {
                Ok(res) => server.send_response(cid, &HmrResponse::EvalResult(res)),
                Err(err) => {
                    server.send_response(cid, &HmrResponse::Error(format!("{err:?}")));
                }
            },
            HmrMessage::Ping => {
                server.send_response(cid, &HmrResponse::Pong);
            }
        }
    }
    reloaded
}

const STANDALONE_POKEDEX_NGB: &str = include_str!("standalone_pokedex.ngb");

fn ensure_bundle_compiled(bundle_path: &str) -> String {
    if let Ok(script) = fs::read_to_string(bundle_path) {
        return script;
    }

    // Check installed system / bundle locations (.deb /usr/share, macOS .app Resources, adjacent to .exe)
    if let Ok(exe_path) = env::current_exe()
        && let Some(exe_dir) = exe_path.parent()
    {
        for candidate in [
            exe_dir.join("main.ngb"),
            exe_dir.join("../Resources/main.ngb"),
            exe_dir.join("../share/ng-linux-counter/main.ngb"),
        ] {
            if let Ok(script) = fs::read_to_string(&candidate) {
                return script;
            }
        }
    }
    if let Ok(script) = fs::read_to_string("/usr/share/ng-linux-counter/main.ngb") {
        return script;
    }

    if Path::new("packages/builders-native/src/build-bundle.mjs").exists() {
        println!(
            "[NgDesktop CLI] Bundle not found at '{bundle_path}'. Auto-compiling via build-bundle.mjs..."
        );
        let _ = Command::new("node")
            .arg("packages/builders-native/src/build-bundle.mjs")
            .status();
        if let Ok(script) = fs::read_to_string(bundle_path) {
            return script;
        }
    }

    String::from(STANDALONE_POKEDEX_NGB)
}

struct InteractiveContext<'a> {
    cfg: &'a AngularNativeConfig,
    bundle_path: &'a str,
    last_mtime: Option<SystemTime>,
    win_id: NodeId,
}

fn run_interactive_loop(
    mut ctx: InteractiveContext<'_>,
    mut engine: RenderEngine<AnyPlatformBackend>,
    mut js_runtime: JsRuntime,
    mut hmr_server: Option<HmrIpcServer>,
) {
    println!(
        "\n[GTK4 Native Window Active] Opened native Linux window '{}'. Watching '{}' for Hot Reload...",
        ctx.cfg.project_name, ctx.bundle_path
    );
    let mut pokedex = PokedexController::new(&js_runtime);
    pokedex.show_list(&mut engine);
    pokedex.start_initial_load(&mut engine);

    while engine.backend().open_window_count() > 0 {
        for ev in engine.backend().step_native_events(false) {
            pokedex.handle_event(&ev, &mut engine, &mut js_runtime);
            js_runtime
                .trigger_node_event_with_payload(&mut engine, ev.node, &ev.event_name, &ev.payload)
                .unwrap();
        }

        pokedex.tick(&mut engine, &mut js_runtime);
        // Commands queued outside event dispatch (async results, view switches) must be
        // flushed explicitly, otherwise they only reach GTK on the *next* user event.
        engine.flush_frame();

        if let Some(ref mut server) = hmr_server
            && handle_hmr_messages(server, &mut engine, &mut js_runtime, ctx.win_id)
        {
            pokedex.refresh_after_reload(&mut engine, &js_runtime);
        }

        let current_mtime = bundle_modified_time(ctx.bundle_path);
        if current_mtime.is_some()
            && current_mtime != ctx.last_mtime
            && let Ok(updated_script) = fs::read_to_string(ctx.bundle_path)
            && !updated_script.trim().is_empty()
        {
            ctx.last_mtime = current_mtime;
            if let Err(err) = js_runtime.hot_reload(&mut engine, &updated_script) {
                eprintln!("[Hot Reload Error] Failed to apply bundle update: {err:?}");
            } else {
                pokedex.refresh_after_reload(&mut engine, &js_runtime);
                println!(
                    "\n[Hot Reload] Applied live update from '{}' (signals preserved):",
                    ctx.bundle_path
                );
                println!("{}", engine.render_window_preview(ctx.win_id).unwrap());
            }
        }

        thread::sleep(Duration::from_millis(16));
    }

    println!("\n--- Scene Graph Snapshot ---");
    engine.print_tree(ctx.win_id, 0).unwrap();
}

fn main() {
    // 1. Read `angular.json` to configure the project & target platform (or use standalone config for .deb/.exe/.dmg)
    let cfg = AngularNativeConfig::from_file("angular.json")
        .unwrap_or_else(|_| AngularNativeConfig::default_standalone("ng-linux-counter"));
    let bundle_path = format!("{}/main.ngb", cfg.output_path);
    let bootstrap_script = ensure_bundle_compiled(&bundle_path);
    let last_mtime = bundle_modified_time(&bundle_path);

    println!(
        "Loaded angular.json -> project='{}', platform='{}', bundle='{}'\n",
        cfg.project_name, cfg.target_platform, bundle_path
    );

    if env::args().any(|a| a == "--package-all") {
        let artifacts = stage_all_distributions(&cfg, "0.2.0").unwrap();
        println!(
            "[NgNative Packager] Staged multi-OS distribution artifacts (Linux + Windows + macOS):"
        );
        for path in artifacts {
            println!("  -> {path}");
        }
        return;
    }

    if env::args().any(|a| a == "--package") {
        let artifacts = stage_linux_distribution(&cfg, "0.2.0").unwrap();
        println!("[NgNative Packager] Staged Linux distribution artifacts:");
        for path in artifacts {
            println!("  -> {path}");
        }
        return;
    }

    // 2. Resolve the OS Backend & Pluggable JS Engine (`jsc` or `quickjs`) via Factory Method
    let backend = AnyPlatformBackend::from_target(&cfg.target_platform).unwrap();
    let mut engine = RenderEngine::new(backend);
    let mut js_runtime = JsRuntime::from_engine_config(&cfg.js_engine).unwrap();
    println!(
        "Initialized JS Engine -> '{}' (JIT={})\n",
        js_runtime.js_engine().engine_name(),
        js_runtime.js_engine().is_jit_enabled()
    );

    // 3. Tick #1: JS Runtime evaluates the compiled Angular bundle (styles + template + bindings)
    js_runtime
        .eval_tick(&mut engine, &bootstrap_script)
        .unwrap();

    // 4. Render visual preview of the mounted native window
    let win_id = js_runtime.resolve_var("win").unwrap();
    println!("\n--- Rendered Window Preview ---");
    println!("{}", engine.render_window_preview(win_id).unwrap());

    let hmr_socket = format!("{}/hmr.sock", cfg.output_path);
    let hmr_server = HmrIpcServer::bind(&hmr_socket).ok();
    if hmr_server.is_some() {
        println!("[HMR IPC] Socket server active -> '{hmr_socket}'");
    }

    if engine.backend().is_native_display_active() && env::var("NG_NATIVE_BATCH").is_err() {
        run_interactive_loop(
            InteractiveContext {
                cfg: &cfg,
                bundle_path: &bundle_path,
                last_mtime,
                win_id,
            },
            engine,
            js_runtime,
            hmr_server,
        );
    } else {
        // Pump non-blocking GTK4 events if available and run declarative click simulation
        let _ = engine.backend().step_event_loop(false);
        println!("\n--- Simulating declarative (click)=\"increment()\" on 'btn' ---");
        js_runtime
            .trigger_event(&mut engine, "btn", "clicked")
            .unwrap();
        println!("\n--- Scene Graph Snapshot ---");
        engine.print_tree(win_id, 0).unwrap();
    }
}

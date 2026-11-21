use ng_native_engine::config::AngularNativeConfig;
use ng_native_engine::engine::{NodeId, RenderEngine};
use ng_native_engine::ipc::{HmrIpcServer, HmrMessage, HmrResponse};
use ng_native_engine::packager::{stage_all_distributions, stage_linux_distribution};
use ng_native_engine::platforms::{AnyPlatformBackend, PlatformBackend};
use ng_native_engine::runtime::{JsEngineBackend, JsRuntime, NativeServerWorker};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
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
) {
    for (cid, msg) in server.poll_messages() {
        match msg {
            HmrMessage::Reload { script } => {
                if let Err(err) = js_runtime.hot_reload(engine, &script) {
                    eprintln!("[HMR IPC Error] Failed to reload bundle: {err:?}");
                    server.send_response(cid, &HmrResponse::Error(format!("{err:?}")));
                } else {
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
}

fn resolve_bundle_candidate_path(cfg: &AngularNativeConfig, filename: &str) -> Option<PathBuf> {
    let mut candidates = vec![PathBuf::from(format!("{}/{filename}", cfg.output_path))];
    if let Ok(exe_path) = env::current_exe()
        && let Some(exe_dir) = exe_path.parent()
    {
        candidates.push(exe_dir.join(filename));
        candidates.push(exe_dir.join(format!("../Resources/{filename}")));
        candidates.push(exe_dir.join(format!("../share/{}/{filename}", cfg.project_name)));
    }
    candidates.push(PathBuf::from(format!(
        "/usr/share/{}/{filename}",
        cfg.project_name
    )));
    candidates.into_iter().find(|p| p.is_file())
}

fn resolve_app_bundle_path(cfg: &AngularNativeConfig) -> Option<PathBuf> {
    resolve_bundle_candidate_path(cfg, "app.bundle.js")
}

fn resolve_server_bundle_path(cfg: &AngularNativeConfig) -> Option<PathBuf> {
    resolve_bundle_candidate_path(cfg, "server.bundle.js")
}

fn ensure_bundle_compiled(cfg: &AngularNativeConfig, bundle_path: &str) -> Result<String, String> {
    if let Ok(script) = fs::read_to_string(bundle_path) {
        return Ok(script);
    }

    // Check installed system / bundle locations (.deb /usr/share, macOS .app Resources, adjacent to .exe)
    if let Ok(exe_path) = env::current_exe()
        && let Some(exe_dir) = exe_path.parent()
    {
        for candidate in [
            exe_dir.join("main.ngb"),
            exe_dir.join("../Resources/main.ngb"),
            exe_dir.join(format!("../share/{}/main.ngb", cfg.project_name)),
        ] {
            if let Ok(script) = fs::read_to_string(&candidate) {
                return Ok(script);
            }
        }
    }
    if let Ok(script) = fs::read_to_string(format!("/usr/share/{}/main.ngb", cfg.project_name)) {
        return Ok(script);
    }

    if Path::new("packages/builders-native/src/build-bundle.mjs").exists() {
        println!(
            "[NgDesktop CLI] Bundle not found at '{bundle_path}'. Auto-compiling '{}' via build-bundle.mjs...",
            cfg.project_name
        );
        let _ = Command::new("node")
            .args([
                "packages/builders-native/src/build-bundle.mjs",
                "--project",
                &cfg.project_name,
            ])
            .status();
        if let Ok(script) = fs::read_to_string(bundle_path) {
            return Ok(script);
        }
    }

    Err(format!(
        "No compiled Angular bundle found at '{bundle_path}'. Run 'ng-desktop build' or 'pnpm run build' first."
    ))
}

type ServerWatchState = Option<(Arc<NativeServerWorker>, String, Option<SystemTime>)>;

struct InteractiveContext<'a> {
    cfg: &'a AngularNativeConfig,
    bundle_path: &'a str,
    is_js_bundle: bool,
    last_mtime: Option<SystemTime>,
    server_state: ServerWatchState,
    win_id: NodeId,
}

fn poll_server_hot_reload(server_state: &mut ServerWatchState) {
    let Some((worker, path, last_mtime)) = server_state.as_mut() else {
        return;
    };
    let current_mtime = bundle_modified_time(path);
    if current_mtime.is_some()
        && current_mtime != *last_mtime
        && let Ok(updated_server_js) = fs::read_to_string(path.as_str())
        && !updated_server_js.trim().is_empty()
    {
        *last_mtime = current_mtime;
        match worker.reload(&updated_server_js) {
            Ok(()) => println!("[Hot Reload] Reloaded isolated server backend '{path}'"),
            Err(err) => eprintln!("[Hot Reload Error] Server backend reload failed: {err}"),
        }
    }
}

fn run_interactive_loop(
    mut ctx: InteractiveContext<'_>,
    mut engine: RenderEngine<AnyPlatformBackend>,
    mut js_runtime: JsRuntime,
    mut hmr_server: Option<HmrIpcServer>,
) {
    println!(
        "\n[Native Window Active] Opened '{}' ({}). Watching '{}' for Hot Reload...",
        ctx.cfg.project_name,
        engine.backend().backend_name(),
        ctx.bundle_path
    );

    while engine.backend().open_window_count() > 0 {
        for ev in engine.backend().step_native_events(false) {
            if let Err(err) = js_runtime.trigger_node_event_with_payload(
                &mut engine,
                ev.node,
                &ev.event_name,
                &ev.payload,
            ) {
                eprintln!("[JS Event Error] {err:?}");
            }
        }

        if ctx.is_js_bundle
            && let Err(err) = js_runtime.tick(&mut engine)
        {
            eprintln!("[JS Tick Error] {err:?}");
        }
        // Commands queued outside event dispatch (async results, view switches) must be
        // flushed explicitly, otherwise they only reach the native toolkit on the next event.
        engine.flush_frame();

        if let Some(ref mut server) = hmr_server {
            handle_hmr_messages(server, &mut engine, &mut js_runtime, ctx.win_id);
        }

        let current_mtime = bundle_modified_time(ctx.bundle_path);
        if current_mtime.is_some()
            && current_mtime != ctx.last_mtime
            && let Ok(updated_script) = fs::read_to_string(ctx.bundle_path)
            && !updated_script.trim().is_empty()
        {
            ctx.last_mtime = current_mtime;
            let update_res = if ctx.is_js_bundle {
                js_runtime.eval_js(&mut engine, &updated_script).map(|_| ())
            } else {
                js_runtime.hot_reload(&mut engine, &updated_script)
            };
            if let Err(err) = update_res {
                eprintln!("[Hot Reload Error] Failed to apply bundle update: {err:?}");
            } else {
                println!(
                    "\n[Hot Reload] Applied live update from '{}' (signals preserved):",
                    ctx.bundle_path
                );
                println!("{}", engine.render_window_preview(ctx.win_id).unwrap());
            }
        }

        poll_server_hot_reload(&mut ctx.server_state);
        thread::sleep(Duration::from_millis(16));
    }

    println!("\n--- Scene Graph Snapshot ---");
    engine.print_tree(ctx.win_id, 0).unwrap();
}

fn handle_cli_actions(args: &[String], cfg: &AngularNativeConfig) -> bool {
    if args.iter().any(|a| a == "--version") {
        println!(
            "{} {} ({})",
            cfg.project_name, cfg.version, cfg.target_platform
        );
        return true;
    }

    if args.iter().any(|a| a == "--permissions-report") {
        println!("{}", cfg.permissions.to_json());
        return true;
    }

    if args.iter().any(|a| a == "--package-all") {
        let artifacts = stage_all_distributions(cfg, &cfg.version).unwrap();
        println!(
            "[NgDesktop Packager] Staged multi-OS distribution artifacts for '{}' v{} (Linux + Windows + macOS):",
            cfg.project_name, cfg.version
        );
        for path in artifacts {
            println!("  -> {path}");
        }
        return true;
    }

    if args.iter().any(|a| a == "--package") {
        let artifacts = stage_linux_distribution(cfg, &cfg.version).unwrap();
        println!(
            "[NgDesktop Packager] Staged Linux distribution artifacts for '{}' v{}:",
            cfg.project_name, cfg.version
        );
        for path in artifacts {
            println!("  -> {path}");
        }
        return true;
    }

    false
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let cfg = AngularNativeConfig::from_cli_and_env(&args);
    if handle_cli_actions(&args, &cfg) {
        return;
    }

    let backend = AnyPlatformBackend::from_config(&cfg).unwrap();
    let mut engine = RenderEngine::new(backend);
    let mut js_runtime = JsRuntime::from_config(&cfg).unwrap();
    println!(
        "Initialized JS Engine -> '{}' (JIT={})\n",
        js_runtime.js_engine().engine_name(),
        js_runtime.js_engine().is_jit_enabled()
    );

    let server_state: ServerWatchState = if js_runtime.js_engine().is_jit_enabled()
        && let Some(server_path) = resolve_server_bundle_path(&cfg)
    {
        let server_path_str = server_path.to_string_lossy().into_owned();
        let server_source = fs::read_to_string(&server_path).unwrap();
        let server_mtime = bundle_modified_time(&server_path_str);
        let worker = js_runtime
            .attach_server_bundle(&server_source, true)
            .unwrap();
        println!("[Native Server Worker] Isolated backend started from '{server_path_str}'\n");
        Some((worker, server_path_str, server_mtime))
    } else {
        None
    };

    let (bundle_path, is_js_bundle, last_mtime) = if let Some(js_path) =
        resolve_app_bundle_path(&cfg)
        && js_runtime.js_engine().is_jit_enabled()
    {
        let path_str = js_path.to_string_lossy().into_owned();
        let js_source = fs::read_to_string(&js_path).unwrap();
        let mtime = bundle_modified_time(&path_str);
        js_runtime.eval_js(&mut engine, &js_source).unwrap();
        for _ in 0..50 {
            if js_runtime.resolve_primary_window().is_ok() {
                break;
            }
            let _ = js_runtime.tick(&mut engine).unwrap();
            thread::sleep(Duration::from_millis(5));
        }
        (path_str, true, mtime)
    } else {
        let ngb_path = format!("{}/main.ngb", cfg.output_path);
        let bootstrap_script = match ensure_bundle_compiled(&cfg, &ngb_path) {
            Ok(script) => script,
            Err(err) => {
                eprintln!("[NgDesktop Error] {err}");
                std::process::exit(1);
            }
        };
        let mtime = bundle_modified_time(&ngb_path);
        js_runtime
            .eval_tick(&mut engine, &bootstrap_script)
            .unwrap();
        (ngb_path, false, mtime)
    };

    println!(
        "Loaded runtime -> project='{}' v'{}', platform='{}', bundle='{}' ({}), permissions={:?}\n",
        cfg.project_name,
        cfg.version,
        cfg.target_platform,
        bundle_path,
        if is_js_bundle {
            "Angular Ivy JS"
        } else {
            "legacy .ngb"
        },
        cfg.permissions.summary_labels()
    );

    // 4. Render visual preview of the mounted native window
    let win_id = js_runtime.resolve_primary_window().unwrap();
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
                is_js_bundle,
                last_mtime,
                server_state,
                win_id,
            },
            engine,
            js_runtime,
            hmr_server,
        );
    } else {
        // Pump non-blocking native events if available and print the mounted scene graph
        let _ = engine.backend().step_event_loop(false);
        println!("\n--- Scene Graph Snapshot ---");
        engine.print_tree(win_id, 0).unwrap();
    }
}

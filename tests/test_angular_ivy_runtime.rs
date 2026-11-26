use ng_native_engine::config::{AngularNativeConfig, AppPermissions};
use ng_native_engine::engine::{EventPayload, RenderCommand, RenderEngine};
use ng_native_engine::platforms::HeadlessBackend;
use ng_native_engine::runtime::JsRuntime;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::thread;
use std::time::Duration;

fn jsc_runtime(app_id: &str, permissions: AppPermissions) -> Option<JsRuntime> {
    let mut cfg = AngularNativeConfig::default_standalone(app_id);
    cfg.js_engine = String::from("jsc");
    cfg.permissions = permissions;
    JsRuntime::from_config(&cfg).ok()
}

fn quickjs_runtime(app_id: &str, permissions: AppPermissions) -> Option<JsRuntime> {
    let mut cfg = AngularNativeConfig::default_standalone(app_id);
    cfg.js_engine = String::from("quickjs");
    cfg.permissions = permissions;
    JsRuntime::from_config(&cfg).ok()
}

#[test]
fn async_host_services_timers_and_permissions_work_end_to_end() {
    let app_id = format!("ng-ivy-runtime-async-{}", process::id());
    let Some(mut rt) = jsc_runtime(&app_id, AppPermissions::allow_all_sandboxed()) else {
        return;
    };
    let mut engine = RenderEngine::new(HeadlessBackend::new());

    {
        let shared = rt.desktop().shared_http_engine();
        let mut http = shared.lock().unwrap();
        http.register_http_endpoint(
            "GET",
            "https://api.ng-native.test/v1/greeting",
            200,
            r#"{"hello":"ivy"}"#,
        );
    }

    let script = r#"
        __NG_NATIVE__.createElement("win", "ng-window");
        __NG_NATIVE__.setText("win", "Ivy Async Host Test");
        __NG_NATIVE__.createElement("lbl", "ng-label");
        __NG_NATIVE__.setText("lbl", "loading");
        __NG_NATIVE__.createElement("timer_lbl", "ng-label");
        __NG_NATIVE__.setText("timer_lbl", "waiting");
        __NG_NATIVE__.createElement("img_lbl", "ng-label");
        __NG_NATIVE__.createElement("root_box", "ng-box");
        __NG_NATIVE__.appendChild("root_box", "lbl");
        __NG_NATIVE__.appendChild("root_box", "timer_lbl");
        __NG_NATIVE__.appendChild("root_box", "img_lbl");
        __NG_NATIVE__.appendChild("win", "root_box");

        __NG_NATIVE__.httpRequestAsync("GET", "https://api.ng-native.test/v1/greeting")
          .then((res) => {
            const parsed = JSON.parse(res.body);
            __NG_NATIVE__.setText("lbl", `http:${res.status}:${parsed.hello}`);
          });

        setTimeout(() => {
          requestAnimationFrame(() => {
            __NG_NATIVE__.setText("timer_lbl", "timer+raf:ok");
          });
        }, 5);

        __NG_NATIVE__.sqliteExecute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT);");
        __NG_NATIVE__.sqliteExecute("INSERT INTO items (id, name) VALUES (1, 'bulba');");
        const rows = __NG_NATIVE__.sqliteQuery("SELECT id, name FROM items;");
        const fuzzy = __NG_NATIVE__.computeFuzzyFilter(["charmander", "squirtle", "bulbasaur"], "bulb", 1);
        const cfgPath = __NG_NATIVE__.fsResolvePath("cache", "ivy-runtime-check.txt");
        __NG_NATIVE__.fsWriteText(cfgPath, `rows=${rows.length};top=${fuzzy[0].value}`);
        const readBack = __NG_NATIVE__.fsReadText(cfgPath);
        __NG_NATIVE__.setText("img_lbl", readBack);
    "#;

    rt.eval_js(&mut engine, script).unwrap();
    let win = rt.resolve_primary_window().unwrap();

    for _ in 0..40 {
        let _ = rt.tick(&mut engine).unwrap();
        let preview = engine.render_window_preview(win).unwrap();
        if preview.contains("http:200:ivy") && preview.contains("timer+raf:ok") {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }

    let preview = engine.render_window_preview(win).unwrap();
    assert!(preview.contains("http:200:ivy"), "preview:\n{preview}");
    assert!(preview.contains("timer+raf:ok"), "preview:\n{preview}");
    assert!(
        preview.contains("rows=1;top=bulbasaur"),
        "preview:\n{preview}"
    );

    // Clean up XDG cache test file created via HostBridge.
    let cache_file = rt
        .eval_js(
            &mut engine,
            "__NG_NATIVE__.fsResolvePath('cache', 'ivy-runtime-check.txt')",
        )
        .unwrap();
    let _ = fs::remove_file(&cache_file);
    if let Some(parent) = PathBuf::from(&cache_file).parent() {
        let _ = fs::remove_dir_all(parent);
    }
}

#[test]
fn host_service_permission_denial_and_image_cache_hit() {
    // Verify permission denial (`403` and `downloadImageAsync`) on minimal permissions,
    // and verify local disk cache hits when network permission is enabled.
    let denied_app_id = format!("ng-ivy-runtime-denied-{}", process::id());
    let Some(mut denied_rt) = jsc_runtime(&denied_app_id, AppPermissions::minimal()) else {
        return;
    };
    let mut denied_engine = RenderEngine::new(HeadlessBackend::new());
    let img_url = format!("https://cdn.example.test/cached-{}.png", process::id());

    let denied_script = format!(
        r#"
        __NG_NATIVE__.createElement("win", "ng-window");
        __NG_NATIVE__.createElement("status_lbl", "ng-label");
        __NG_NATIVE__.appendChild("win", "status_lbl");
        Promise.all([
          __NG_NATIVE__.httpRequestAsync("GET", "https://example.test/blocked"),
          __NG_NATIVE__.downloadImageAsync("{img_url}")
        ]).then(([httpRes, imgRes]) => {{
          __NG_NATIVE__.setText("status_lbl", `imgOk=${{imgRes.ok}};status=${{httpRes.status}}`);
        }});
    "#
    );
    denied_rt
        .eval_js(&mut denied_engine, &denied_script)
        .unwrap();
    let denied_win = denied_rt.resolve_primary_window().unwrap();
    for _ in 0..20 {
        let _ = denied_rt.tick(&mut denied_engine).unwrap();
        let p = denied_engine.render_window_preview(denied_win).unwrap();
        if p.contains("imgOk=false;status=403") {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    let denied_preview = denied_engine.render_window_preview(denied_win).unwrap();
    assert!(
        denied_preview.contains("imgOk=false;status=403"),
        "denied_preview:\n{denied_preview}"
    );

    let cached_app_id = format!("ng-ivy-runtime-cache-{}", process::id());
    let Some(mut cached_rt) = jsc_runtime(&cached_app_id, AppPermissions::allow_all_sandboxed())
    else {
        return;
    };
    let mut cached_engine = RenderEngine::new(HeadlessBackend::new());
    let env_json = cached_rt
        .host_bridge()
        .handle_service_call(&format!(r#"{{"action":"systemEnv","src":"{img_url}"}}"#));
    let resolved_path = env_json
        .split(r#""resolvedImage":""#)
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("resolvedImage field");
    let local_img = PathBuf::from(resolved_path.split('?').next().unwrap());
    if let Some(parent) = local_img.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&local_img, b"PNG_CACHE_HIT").unwrap();

    let cache_script = format!(
        r#"
        __NG_NATIVE__.createElement("win", "ng-window");
        __NG_NATIVE__.createElement("status_lbl", "ng-label");
        __NG_NATIVE__.appendChild("win", "status_lbl");
        __NG_NATIVE__.downloadImageAsync("{img_url}").then((imgRes) => {{
          __NG_NATIVE__.setText("status_lbl", `cached=${{imgRes.cached}}`);
        }});
    "#
    );
    cached_rt
        .eval_js(&mut cached_engine, &cache_script)
        .unwrap();
    let cached_win = cached_rt.resolve_primary_window().unwrap();
    for _ in 0..20 {
        let _ = cached_rt.tick(&mut cached_engine).unwrap();
        let p = cached_engine.render_window_preview(cached_win).unwrap();
        if p.contains("cached=true") {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    let cached_preview = cached_engine.render_window_preview(cached_win).unwrap();
    assert!(
        cached_preview.contains("cached=true"),
        "cached_preview:\n{cached_preview}"
    );

    let _ = fs::remove_file(&local_img);
    if let Some(parent) = local_img.parent().and_then(Path::parent) {
        let _ = fs::remove_dir_all(parent);
    }
}

#[test]
fn remove_class_dynamic_for_reordering_and_angular_dom_shims_execute_cleanly() {
    let app_id = format!("ng-ivy-runtime-dom-{}", process::id());
    let Some(mut rt) = jsc_runtime(&app_id, AppPermissions::allow_all_sandboxed()) else {
        return;
    };
    let mut engine = RenderEngine::new(HeadlessBackend::new());

    let script = r##"
        __NG_NATIVE__.registerStyle("active", "#1d4ed8", "#ffffff", 4, 2, "radius=6");
        __NG_NATIVE__.registerStyle("badge", "#0f172a", "#38bdf8", 4, 2, "");
        __NG_NATIVE__.createElement("win", "ng-window");
        __NG_NATIVE__.createElement("list", "ng-box");
        __NG_NATIVE__.createElement("anchor", "ng-label");
        __NG_NATIVE__.createComment("end_anchor", "@for(end)");
        __NG_NATIVE__.createElement("item_a", "ng-label");
        __NG_NATIVE__.setText("item_a", "First");
        __NG_NATIVE__.createElement("item_b", "ng-label");
        __NG_NATIVE__.setText("item_b", "Second");

        __NG_NATIVE__.addClass("item_a", "active");
        __NG_NATIVE__.addClass("item_a", "badge");
        __NG_NATIVE__.removeClass("item_a", "active");

        __NG_NATIVE__.appendChild("list", "end_anchor");
        __NG_NATIVE__.insertBefore("list", "item_b", "end_anchor");
        __NG_NATIVE__.insertBefore("list", "item_a", "item_b");
        __NG_NATIVE__.appendChild("list", "anchor");
        __NG_NATIVE__.removeChild("list", "anchor");
        __NG_NATIVE__.appendChild("win", "list");

        // Verify Angular 22 Web/DOM shims operate cleanly in JavaScriptCore
        const styleEl = document.createElement("style");
        styleEl.textContent = ".demo { color: red; }";
        document.head.appendChild(styleEl);
        const q = document.querySelectorAll("style");
        const u = new URL("https://example.test/items?page=2");
        const enc = new TextEncoder().encode(u.searchParams.get("page"));
        const hdr = new Headers({ "X-App": "NgDesktop" });
        const evt = new CustomEvent("native-ready", { detail: enc.length });
        const ctrl = new AbortController();
        ctrl.abort("done");
        new IntersectionObserver(() => {}).observe(styleEl);
        new MutationObserver(() => {}).disconnect();
        new ResizeObserver(() => {}).disconnect();
        if (q.length < 1 || hdr.get("x-app") !== "NgDesktop" || evt.detail !== 1 || !ctrl.signal.aborted) {
          throw new Error("DOM/Web shim check failed");
        }
    "##;

    rt.eval_js(&mut engine, script).unwrap();
    engine.flush_frame();
    let win = rt.resolve_primary_window().unwrap();
    let preview = engine.render_window_preview(win).unwrap();
    let pos_first = preview.find("First").expect("First label in preview");
    let pos_second = preview.find("Second").expect("Second label in preview");
    assert!(
        pos_first < pos_second,
        "Expected First before Second after insertBefore:\n{preview}"
    );
    let saw_remove_class = engine
        .backend()
        .frames
        .iter()
        .flatten()
        .any(|cmd| matches!(cmd, RenderCommand::RemoveStyleClass { .. }));
    assert!(
        saw_remove_class,
        "Expected RemoveStyleClass in backend frames"
    );
    let saw_remove_child = engine
        .backend()
        .frames
        .iter()
        .flatten()
        .any(|cmd| matches!(cmd, RenderCommand::RemoveChild { .. }));
    assert!(saw_remove_child, "Expected RemoveChild in backend frames");
}

fn bundle_ephemeral_angular_22_ivy_app() -> Option<String> {
    let pnpm_dir = Path::new("node_modules/.pnpm");
    let has_esbuild = fs::read_dir(pnpm_dir)
        .ok()?
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_string_lossy().starts_with("esbuild@"));
    if !has_esbuild {
        return None;
    }

    let script = r"
const fs = require('fs');
const path = require('path');
const esbuildDir = fs.readdirSync('node_modules/.pnpm').find((d) => d.startsWith('esbuild@'));
const esbuild = require(path.resolve('node_modules/.pnpm', esbuildDir, 'node_modules/esbuild'));
const entry = `
import '@angular/compiler';
import {
  signal,
  ɵɵdefineComponent,
  ɵɵelementStart,
  ɵɵelementEnd,
  ɵɵtext,
  ɵɵtextInterpolate,
  ɵɵlistener,
  ɵɵadvance,
} from '@angular/core';
import { bootstrapNativeApplication } from '@ng-desktop/platform';

class CounterApp {
  count = signal(1);
  inc() {
    this.count.update((v) => v + 1);
  }
  static ɵfac = () => new CounterApp();
  static ɵcmp = ɵɵdefineComponent({
    type: CounterApp,
    selectors: [['app-root']],
    decls: 6,
    vars: 1,
    consts: [
      ['id', 'win', 'title', 'Ivy Counter'],
      ['orientation', 'vertical', 'class', 'panel'],
      ['id', 'lbl'],
      ['id', 'inc_btn'],
    ],
    template: (rf, ctx) => {
      if (rf & 1) {
        ɵɵelementStart(0, 'ng-window', 0);
        ɵɵelementStart(1, 'ng-box', 1);
        ɵɵelementStart(2, 'ng-label', 2);
        ɵɵtext(3);
        ɵɵelementEnd();
        ɵɵelementStart(4, 'ng-button', 3);
        ɵɵlistener('click', () => ctx.inc());
        ɵɵtext(5, '+1');
        ɵɵelementEnd();
        ɵɵelementEnd();
        ɵɵelementEnd();
      }
      if (rf & 2) {
        ɵɵadvance(3);
        ɵɵtextInterpolate('Count: ' + ctx.count());
      }
    },
    styles: ['.panel[_ngcontent-%COMP%] { background: #1e293b; color: #f8fafc; padding: 10px; border-radius: 8px; }'],
    encapsulation: 2,
  });
}

bootstrapNativeApplication(CounterApp).appRefPromise.catch((err) => {
  console.error(err && err.stack ? err.stack : String(err));
});
`;
const out = esbuild.buildSync({
  stdin: { contents: entry, resolveDir: process.cwd(), loader: 'ts' },
  bundle: true,
  format: 'iife',
  write: false,
  platform: 'browser',
  mainFields: ['fesm2022', 'module', 'main'],
  alias: {
    '@ng-desktop/platform': path.resolve('packages/platform-native/src/index.ts'),
  },
});
process.stdout.write(out.outputFiles[0].text);
";

    let output = Command::new("node").args(["-e", script]).output().ok()?;
    assert!(
        output.status.success(),
        "Failed to bundle Angular 22 Ivy test app:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[test]
fn boots_real_angular_22_ivy_application_via_native_renderer_and_signals() {
    let Some(bundle_js) = bundle_ephemeral_angular_22_ivy_app() else {
        return;
    };

    for (engine_label, rt_opt) in [
        (
            "jsc",
            jsc_runtime(
                &format!("ng-ivy-e2e-jsc-{}", process::id()),
                AppPermissions::allow_all_sandboxed(),
            ),
        ),
        (
            "quickjs",
            quickjs_runtime(
                &format!("ng-ivy-e2e-qjs-{}", process::id()),
                AppPermissions::allow_all_sandboxed(),
            ),
        ),
    ] {
        let mut rt = rt_opt.unwrap_or_else(|| panic!("runtime '{engine_label}' should initialize"));
        let mut engine = RenderEngine::new(HeadlessBackend::new());
        rt.eval_js(&mut engine, &bundle_js).unwrap();
        for _ in 0..30 {
            if rt.resolve_primary_window().is_ok() {
                let preview = engine
                    .render_window_preview(rt.resolve_primary_window().unwrap())
                    .unwrap();
                if preview.contains("Count: 1") {
                    break;
                }
            }
            let _ = rt.tick(&mut engine).unwrap();
            thread::sleep(Duration::from_millis(5));
        }

        let errors_json = rt
            .eval_js(
                &mut engine,
                "JSON.stringify(__NG_NATIVE__.getSystemErrors())",
            )
            .unwrap();
        assert_eq!(
            errors_json, "[]",
            "[{engine_label}] Angular 22 runtime errors: {errors_json}"
        );

        let win = rt.resolve_primary_window().unwrap();
        let initial_preview = engine.render_window_preview(win).unwrap();
        assert!(
            initial_preview.contains("Ivy Counter"),
            "[{engine_label}] initial_preview:\n{initial_preview}"
        );
        assert!(
            initial_preview.contains("Count: 1"),
            "[{engine_label}] initial_preview:\n{initial_preview}"
        );
        assert!(
            initial_preview.contains("+1"),
            "[{engine_label}] initial_preview:\n{initial_preview}"
        );

        let btn_id = rt.resolve_var("inc_btn").unwrap();
        rt.trigger_node_event_with_payload(&mut engine, btn_id, "clicked", &EventPayload::None)
            .unwrap();
        for _ in 0..10 {
            let _ = rt.tick(&mut engine).unwrap();
            let updated = engine.render_window_preview(win).unwrap();
            if updated.contains("Count: 2") {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        let updated_preview = engine.render_window_preview(win).unwrap();
        assert!(
            updated_preview.contains("Count: 2"),
            "[{engine_label}] updated_preview:\n{updated_preview}"
        );
    }
}

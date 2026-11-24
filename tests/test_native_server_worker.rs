use ng_native_engine::config::{AngularNativeConfig, AppPermissions};
use ng_native_engine::engine::RenderEngine;
use ng_native_engine::platforms::HeadlessBackend;
use ng_native_engine::runtime::{JsEngineBackend, JsRuntime};
use std::fs;
use std::path::Path;
use std::process::{self, Command};
use std::thread;
use std::time::Duration;

fn jsc_runtime(app_id: &str, permissions: AppPermissions) -> Option<JsRuntime> {
    let mut cfg = AngularNativeConfig::default_standalone(app_id);
    cfg.js_engine = String::from("jsc");
    cfg.permissions = permissions;
    let rt = JsRuntime::from_config(&cfg).ok()?;
    if !rt.js_engine().is_jit_enabled() {
        return None;
    }
    Some(rt)
}

fn bundle_ts_with_esbuild(entry_ts: &str) -> Option<String> {
    let pnpm_dir = Path::new("node_modules/.pnpm");
    let has_esbuild = fs::read_dir(pnpm_dir)
        .ok()?
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_string_lossy().starts_with("esbuild@"));
    if !has_esbuild {
        return None;
    }

    let runner = r"
const fs = require('fs');
const path = require('path');
const esbuildDir = fs.readdirSync('node_modules/.pnpm').find((d) => d.startsWith('esbuild@'));
const esbuild = require(path.resolve('node_modules/.pnpm', esbuildDir, 'node_modules/esbuild'));
const entry = fs.readFileSync(0, 'utf8');
const out = esbuild.buildSync({
  stdin: { contents: entry, resolveDir: process.cwd(), loader: 'ts' },
  bundle: true,
  format: 'iife',
  write: false,
  platform: 'browser',
  target: 'es2022',
  mainFields: ['fesm2022', 'module', 'main'],
  alias: {
    '@ng-desktop/platform': path.resolve('packages/platform-native/src/index.ts'),
    '@ng-desktop/http': path.resolve('packages/http-native/src/index.ts'),
    '@ng-desktop/server': path.resolve('packages/server-native/src/index.ts'),
  },
});
process.stdout.write(out.outputFiles[0].text);
";

    let mut child = Command::new("node")
        .args(["-e", runner])
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .spawn()
        .ok()?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin.write_all(entry_ts.as_bytes()).ok()?;
    }
    let output = child.wait_with_output().ok()?;
    assert!(
        output.status.success(),
        "Failed to bundle TS entry with esbuild:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn pump_until_preview(
    rt: &mut JsRuntime,
    engine: &mut RenderEngine<HeadlessBackend>,
    win_id: ng_native_engine::engine::NodeId,
    expected: &str,
) -> String {
    for _ in 0..160 {
        let _ = rt.tick(engine).unwrap();
        let preview = engine.render_window_preview(win_id).unwrap();
        if preview.contains(expected) {
            return preview;
        }
        thread::sleep(Duration::from_millis(5));
    }
    engine.render_window_preview(win_id).unwrap()
}

const NOTES_SERVER_ENTRY: &str = r"
import {
  createError,
  defineNativeEventHandler,
  getRouterParam,
  readBody,
  registerNativeServerApp,
  setResponseStatus,
} from '@ng-desktop/server';

const ensureSchema = (event: {
  context: { db: { execute: (sql: string, params?: readonly (string | number | boolean | null)[]) => boolean } };
}) => {
  event.context.db.execute('CREATE TABLE IF NOT EXISTS notes (id INTEGER PRIMARY KEY, text TEXT);');
};

registerNativeServerApp([
  {
    method: 'POST',
    pattern: '/api/notes',
    handler: defineNativeEventHandler((event) => {
      ensureSchema(event);
      const body = readBody<{ id: number; text: string }>(event);
      event.context.db.execute(
        'INSERT OR REPLACE INTO notes (id, text) VALUES (?, ?);',
        [Number(body.id), String(body.text || '')],
      );
      setResponseStatus(event, 201, 'Created');
      return { saved: true, id: body.id };
    }),
  },
  {
    method: 'GET',
    pattern: '/api/notes',
    handler: defineNativeEventHandler((event) => {
      ensureSchema(event);
      return { items: event.context.db.query('SELECT id, text FROM notes ORDER BY id ASC;') };
    }),
  },
  {
    method: 'GET',
    pattern: '/api/notes/:id',
    handler: defineNativeEventHandler((event) => {
      ensureSchema(event);
      const id = getRouterParam(event, 'id');
      const rows = event.context.db.query('SELECT id, text FROM notes WHERE id = ?;', [Number(id)]);
      if (rows.length === 0) {
        throw createError({ statusCode: 404, message: 'Note not found' });
      }
      return rows[0];
    }),
  },
  {
    method: 'GET',
    pattern: '/api/delayed',
    handler: defineNativeEventHandler(
      () =>
        new Promise((resolve) => {
          setTimeout(() => resolve({ settledAfterMs: 500 }), 500);
        }),
    ),
  },
  {
    method: 'GET',
    pattern: '/api/external',
    handler: defineNativeEventHandler(async (event) => {
      const upstream = await event.context.http.get('https://api.ng-native.test/upstream');
      return { upstreamStatus: upstream.status, upstreamBody: upstream.body };
    }),
  },
]);
";

#[test]
fn routes_local_api_calls_to_isolated_server_worker_sqlite_and_hmr() {
    let app_id = format!("ng-server-worker-test-{}", process::id());
    let offline_perms = AppPermissions {
        sqlite: true,
        ..AppPermissions::minimal()
    };
    let Some(rt) = jsc_runtime(&app_id, offline_perms) else {
        return;
    };
    let Some(server_js) = bundle_ts_with_esbuild(NOTES_SERVER_ENTRY) else {
        return;
    };

    let worker = rt.attach_server_bundle(&server_js, false).unwrap();
    let mut rt = rt;
    let mut engine = RenderEngine::new(HeadlessBackend::new());

    // Even with `network: false`, `/api/*` routes succeed against isolated SQLite backend,
    // while missing routes throw `404` and outbound upstream `http.get` returns `403`.
    let ui_script = r#"
        __NG_NATIVE__.createElement("win", "ng-window");
        __NG_NATIVE__.createElement("lbl_notes", "ng-label");
        __NG_NATIVE__.createElement("lbl_err", "ng-label");
        __NG_NATIVE__.createElement("box", "ng-box");
        __NG_NATIVE__.appendChild("box", "lbl_notes");
        __NG_NATIVE__.appendChild("box", "lbl_err");
        __NG_NATIVE__.appendChild("win", "box");

        __NG_NATIVE__.httpRequestAsync(
          "POST",
          "/api/notes",
          JSON.stringify({ id: 7, text: "O'Reilly; persisted-in-worker" })
        ).then((postRes) => {
          return Promise.all([
            Promise.resolve(postRes),
            __NG_NATIVE__.httpRequestAsync("GET", "/api/notes"),
            __NG_NATIVE__.httpRequestAsync("GET", "/api/notes/999"),
            __NG_NATIVE__.httpRequestAsync("GET", "/api/external"),
            __NG_NATIVE__.httpRequestAsync("GET", "/api/delayed"),
          ]);
        }).then(([postRes, listRes, missingRes, extRes, delayedRes]) => {
          const parsed = JSON.parse(listRes.body);
          const delayed = JSON.parse(delayedRes.body);
          __NG_NATIVE__.setText(
            "lbl_notes",
            `post=${postRes.status};idType=${typeof parsed.items[0].id};note=${parsed.items[0].text};delay=${delayedRes.status}:${delayed.settledAfterMs}`
          );
          __NG_NATIVE__.setText(
            "lbl_err",
            `missing=${missingRes.status};extDenied=${JSON.parse(extRes.body).upstreamStatus}`
          );
        }).catch((err) => {
          __NG_NATIVE__.setText(
            "lbl_err",
            `ERR:${err && err.message ? `${err.name || 'Error'}: ${err.message}` : String(err)}`
          );
        });
    "#;
    rt.eval_js(&mut engine, ui_script).unwrap();
    let win = rt.resolve_primary_window().unwrap();

    let preview = pump_until_preview(&mut rt, &mut engine, win, "delay=200:500");
    assert!(
        preview.contains("post=201;idType=number;note=O'Reilly; persisted-in-worker;delay=200:500"),
        "preview:\n{preview}"
    );
    assert!(
        preview.contains("missing=404;extDenied=403"),
        "preview:\n{preview}"
    );

    // Hot-reload server worker bundle while preserving live UI state
    let reloaded_server_js = NOTES_SERVER_ENTRY.replace(
        "return { items: event.context.db.query('SELECT id, text FROM notes ORDER BY id ASC;') };",
        "return { items: [{ id: '7', text: 'hmr-v2' }] };",
    );
    let reloaded_bundle = bundle_ts_with_esbuild(&reloaded_server_js).unwrap();
    worker.reload(&reloaded_bundle).unwrap();
    rt.eval_js(
        &mut engine,
        r#"
        __NG_NATIVE__.httpRequestAsync("GET", "/api/notes").then((res) => {
          const parsed = JSON.parse(res.body);
          __NG_NATIVE__.setText("lbl_notes", `reloaded=${parsed.items[0].text}`);
        });
        "#,
    )
    .unwrap();
    let after_hmr = pump_until_preview(&mut rt, &mut engine, win, "reloaded=hmr-v2");
    assert!(after_hmr.contains("reloaded=hmr-v2"), "hmr:\n{after_hmr}");
}

#[test]
fn server_worker_routes_outbound_http_when_network_granted() {
    let net_app_id = format!("ng-server-worker-net-{}", process::id());
    let Some(online_rt) = jsc_runtime(
        &net_app_id,
        AppPermissions {
            network: true,
            ..AppPermissions::minimal()
        },
    ) else {
        return;
    };
    let Some(server_js) = bundle_ts_with_esbuild(NOTES_SERVER_ENTRY) else {
        return;
    };
    {
        let shared = online_rt.desktop().shared_http_engine();
        let mut http = shared.lock().unwrap();
        http.register_http_endpoint(
            "GET",
            "https://api.ng-native.test/upstream",
            200,
            r#"{"source":"pokeapi-mock"}"#,
        );
    }
    let _net_worker = online_rt.attach_server_bundle(&server_js, false).unwrap();
    let mut online_rt = online_rt;
    let mut online_engine = RenderEngine::new(HeadlessBackend::new());
    online_rt
        .eval_js(
            &mut online_engine,
            r#"
            __NG_NATIVE__.createElement("win", "ng-window");
            __NG_NATIVE__.createElement("lbl", "ng-label");
            __NG_NATIVE__.appendChild("win", "lbl");
            __NG_NATIVE__.httpRequestAsync("GET", "/api/external").then((res) => {
              const p = JSON.parse(res.body);
              const inner = JSON.parse(p.upstreamBody);
              __NG_NATIVE__.setText("lbl", `upstream=${p.upstreamStatus}:${inner.source}`);
            });
            "#,
        )
        .unwrap();
    let online_win = online_rt.resolve_primary_window().unwrap();
    let online_preview = pump_until_preview(
        &mut online_rt,
        &mut online_engine,
        online_win,
        "upstream=200:pokeapi-mock",
    );
    assert!(
        online_preview.contains("upstream=200:pokeapi-mock"),
        "online_preview:\n{online_preview}"
    );
}

const GREETING_SERVER_ENTRY: &str = r"
import { defineNativeEventHandler, registerNativeServerApp } from '@ng-desktop/server';

registerNativeServerApp([
  {
    method: 'GET',
    pattern: '/api/greeting',
    handler: defineNativeEventHandler(() => ({
      message: 'Hello from isolated server worker',
    })),
  },
]);
";

const GREETING_FRONTEND_ENTRY: &str = r"
import '@angular/compiler';
import { HttpClient } from '@angular/common/http';
import {
  inject,
  signal,
  ɵɵadvance,
  ɵɵdefineComponent,
  ɵɵelementEnd,
  ɵɵelementStart,
  ɵɵtext,
  ɵɵtextInterpolate,
} from '@angular/core';
import { provideHttpClient, withFetch } from '@ng-desktop/http';
import { bootstrapNativeApplication } from '@ng-desktop/platform';

class GreetingApp {
  private readonly http = inject(HttpClient);
  readonly message = signal('Loading...');

  constructor() {
    this.http
      .get<{ message: string }>('/api/greeting')
      .subscribe((res) => this.message.set(res.message));
  }

  static ɵfac = () => new GreetingApp();
  static ɵcmp = ɵɵdefineComponent({
    type: GreetingApp,
    selectors: [['app-root']],
    decls: 3,
    vars: 1,
    consts: [['id', 'win', 'title', 'Analog Native App'], ['id', 'status_lbl']],
    template: (rf, ctx) => {
      if (rf & 1) {
        ɵɵelementStart(0, 'ng-window', 0);
        ɵɵelementStart(1, 'ng-label', 1);
        ɵɵtext(2);
        ɵɵelementEnd();
        ɵɵelementEnd();
      }
      if (rf & 2) {
        ɵɵadvance(2);
        ɵɵtextInterpolate(ctx.message());
      }
    },
  });
}

bootstrapNativeApplication(GreetingApp, {
  providers: [provideHttpClient(withFetch())],
}).appRefPromise?.catch((err) => {
  console.error(err && err.stack ? err.stack : String(err));
});
";

#[test]
fn real_angular_22_httpclient_calls_local_native_server_backend() {
    let app_id = format!("ng-angular-server-e2e-{}", process::id());
    let Some(rt) = jsc_runtime(&app_id, AppPermissions::minimal()) else {
        return;
    };
    let (Some(server_js), Some(frontend_js)) = (
        bundle_ts_with_esbuild(GREETING_SERVER_ENTRY),
        bundle_ts_with_esbuild(GREETING_FRONTEND_ENTRY),
    ) else {
        return;
    };

    let _worker = rt.attach_server_bundle(&server_js, false).unwrap();
    let mut rt = rt;
    let mut engine = RenderEngine::new(HeadlessBackend::new());
    rt.eval_js(&mut engine, &frontend_js).unwrap();

    for _ in 0..160 {
        if let Ok(win) = rt.resolve_primary_window() {
            let preview = engine.render_window_preview(win).unwrap();
            if preview.contains("Hello from isolated server worker") {
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
        "Angular 22 runtime errors: {errors_json}"
    );

    let win = rt.resolve_primary_window().unwrap();
    let preview = engine.render_window_preview(win).unwrap();
    assert!(preview.contains("Analog Native App"), "preview:\n{preview}");
    assert!(
        preview.contains("Hello from isolated server worker"),
        "preview:\n{preview}"
    );
}

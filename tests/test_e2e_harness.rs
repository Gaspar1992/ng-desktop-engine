use ng_native_engine::desktop::{NativeFsManager, XdgDirKind};
use ng_native_engine::runtime::{JsEngineBackend, JsRuntime};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static E2E_SEQ: AtomicU64 = AtomicU64::new(1);

struct TestIpcClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl TestIpcClient {
    fn spawn(args: &[&str]) -> (Self, String) {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ng-native-engine"))
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("Failed to spawn ng-native-engine binary for E2E harness");

        let stdin = child.stdin.take().expect("piped stdin");
        let stdout_raw = child.stdout.take().expect("piped stdout");
        let mut stdout = BufReader::new(stdout_raw);

        let mut banner = String::new();
        stdout
            .read_line(&mut banner)
            .expect("Failed reading startup status JSON line");
        (
            Self {
                child,
                stdin,
                stdout,
            },
            banner.trim().to_string(),
        )
    }

    fn send(&mut self, json_line: &str) -> String {
        writeln!(self.stdin, "{json_line}").expect("Failed writing IPC command");
        self.stdin.flush().expect("Failed flushing IPC stdin");

        let mut reply = String::new();
        let read = self
            .stdout
            .read_line(&mut reply)
            .expect("Failed reading IPC reply");
        assert!(
            read > 0,
            "Engine closed stdout unexpectedly after: {json_line}"
        );
        reply.trim().to_string()
    }

    fn send_ok(&mut self, json_line: &str) -> String {
        let reply = self.send(json_line);
        assert!(
            reply.contains(r#""ok":true"#),
            "IPC command failed!\nCommand: {json_line}\nReply: {reply}"
        );
        reply
    }

    fn eval_ok(&mut self, js_code: &str) -> String {
        let escaped = json_escape(js_code);
        self.send_ok(&format!(r#"{{"cmd":"eval","code":"{escaped}"}}"#))
    }

    fn quit(mut self) {
        let bye = self.send_ok(r#"{"cmd":"quit"}"#);
        assert!(bye.contains(r#""bye":true"#), "Unexpected bye: {bye}");
        let status = self.child.wait().expect("Failed waiting on child");
        assert!(status.success(), "Engine exited with failure: {status}");
    }
}

fn json_escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 16);
    for ch in raw.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

fn mock_http_cmd(url: &str, status: u16, body: &str) -> String {
    format!(
        r#"{{"cmd":"mock_http","method":"GET","url":"{}","status":{status},"body":"{}"}}"#,
        json_escape(url),
        json_escape(body),
    )
}

fn make_e2e_temp_dir(prefix: &str) -> PathBuf {
    let seq = E2E_SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("{prefix}-{}-{seq}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create E2E temp dir");
    dir
}

fn require_full_e2e_flag() -> bool {
    std::env::var("NG_E2E_REQUIRE_FULL").is_ok_and(|raw| {
        let trimmed = raw.trim();
        trimmed == "1" || trimmed.eq_ignore_ascii_case("true")
    })
}

fn ensure_pokedex_bundle_ready() -> bool {
    let dist_dir = PathBuf::from("dist/ng-linux-counter");
    let ready = dist_dir.join("app.bundle.js").is_file()
        && dist_dir.join("server.bundle.js").is_file()
        && dist_dir.join("ng-desktop.runtime.json").is_file();
    assert!(
        ready || !require_full_e2e_flag(),
        "NG_E2E_REQUIRE_FULL=1 requires built dist/ng-linux-counter bundles (app.bundle.js, server.bundle.js, ng-desktop.runtime.json). Run `pnpm run build` first."
    );
    ready
}

fn seed_pokedex_upstream_mocks(client: &mut TestIpcClient) {
    let catalog_json = r#"{"results":[{"name":"bulbasaur","url":"https://pokeapi.co/api/v2/pokemon/1/"},{"name":"charmander","url":"https://pokeapi.co/api/v2/pokemon/4/"},{"name":"squirtle","url":"https://pokeapi.co/api/v2/pokemon/7/"},{"name":"pikachu","url":"https://pokeapi.co/api/v2/pokemon/25/"}]}"#;
    let pikachu_json = r#"{"id":25,"name":"pikachu","height":4,"weight":60,"base_experience":112,"types":[{"slot":1,"type":{"name":"electric"}}],"abilities":[{"is_hidden":false,"ability":{"name":"static"}}],"stats":[{"base_stat":35,"stat":{"name":"hp"}},{"base_stat":55,"stat":{"name":"attack"}},{"base_stat":40,"stat":{"name":"defense"}},{"base_stat":50,"stat":{"name":"special-attack"}},{"base_stat":50,"stat":{"name":"special-defense"}},{"base_stat":90,"stat":{"name":"speed"}}],"sprites":{"other":{"official-artwork":{"front_default":"local://25.png"}}}}"#;
    let pikachu_species_json = r#"{"genera":[{"genus":"Pokémon Ratón","language":{"name":"es"}}],"flavor_text_entries":[{"flavor_text":"Almacena electricidad en sus mejillas.","language":{"name":"es"}}]}"#;
    let bulba_json = r#"{"id":1,"name":"bulbasaur","height":7,"weight":69,"base_experience":64,"types":[{"slot":1,"type":{"name":"grass"}},{"slot":2,"type":{"name":"poison"}}],"abilities":[{"is_hidden":false,"ability":{"name":"overgrow"}}],"stats":[{"base_stat":45,"stat":{"name":"hp"}}],"sprites":{"other":{"official-artwork":{"front_default":"local://1.png"}}}}"#;
    let bulba_species_json = r#"{"genera":[{"genus":"Pokémon Semilla","language":{"name":"es"}}],"flavor_text_entries":[{"flavor_text":"Una rara semilla le fue plantada al nacer.","language":{"name":"es"}}]}"#;

    for (url, body) in [
        (
            "https://pokeapi.co/api/v2/pokemon?limit=151&offset=0",
            catalog_json,
        ),
        ("https://pokeapi.co/api/v2/pokemon/25", pikachu_json),
        (
            "https://pokeapi.co/api/v2/pokemon-species/25",
            pikachu_species_json,
        ),
        ("https://pokeapi.co/api/v2/pokemon/1", bulba_json),
        (
            "https://pokeapi.co/api/v2/pokemon-species/1",
            bulba_species_json,
        ),
    ] {
        client.send_ok(&mock_http_cmd(url, 200, body));
    }
}

fn clean_xdg_app_data(app_id: &str) {
    if let Ok(xdg_probe) =
        NativeFsManager::new(app_id).resolve_xdg_path(XdgDirKind::Data, "db.sqlite")
        && let Some(parent) = xdg_probe.parent()
    {
        let _ = fs::remove_dir_all(parent);
    }
}

fn spawn_pokedex_harness(app_id: &str) -> TestIpcClient {
    let (mut client, banner) = TestIpcClient::spawn(&[
        "--headless",
        "--test-ipc=stdio",
        "--defer-boot",
        "--config",
        "dist/ng-linux-counter/ng-desktop.runtime.json",
        "--project",
        app_id,
        "--output-path",
        "dist/ng-linux-counter",
    ]);
    assert!(
        banner.contains(r#""status":"waiting_boot""#),
        "banner: {banner}"
    );
    seed_pokedex_upstream_mocks(&mut client);
    let booted = client.send_ok(r#"{"cmd":"boot"}"#);
    assert!(
        booted.contains(r#""bundle":"angular-ivy""#),
        "booted: {booted}"
    );
    client
}

#[test]
fn agnostic_headless_test_ipc_e2e_workflow() {
    let tmp_dir = make_e2e_temp_dir("ng-e2e-agnostic");
    let out_slash = tmp_dir.to_string_lossy().replace('\\', "/");
    let cfg_path = tmp_dir.join("ng-desktop.runtime.json");
    fs::write(
        &cfg_path,
        format!(
            r#"{{"projectName":"agnostic-e2e","version":"0.7.0","targetPlatform":"headless","outputPath":"{out_slash}","permissions":{{"sqlite":false,"network":false}}}}"#
        ),
    )
    .unwrap();
    fs::write(
        tmp_dir.join("main.ngb"),
        r#"
            style card #1e293b #ffffff 8 6
            create win ng-window
            text win "Generic Headless App"
            create box ng-box
            create lbl ng-label
            attr lbl data-testid "counter-label"
            text lbl "Ready:0"
            create btn ng-button
            attr btn data-testid "inc-btn"
            text btn "Step"
            signal clicks 0
            on_add btn clicked clicks 1
            bind_text lbl clicks "Ready:{}"
            append box lbl
            append box btn
            append win box
        "#,
    )
    .unwrap();

    let cfg_arg = cfg_path.to_string_lossy().into_owned();
    let (mut client, banner) =
        TestIpcClient::spawn(&["--headless", "--test-ipc=stdio", "--config", &cfg_arg]);
    assert!(banner.contains(r#""status":"ready""#), "banner: {banner}");
    assert!(
        banner.contains(r#""backend":"Headless Test Backend""#),
        "banner: {banner}"
    );
    assert_eq!(
        client.send_ok(r#"{"cmd":"ping"}"#),
        r#"{"ok":true,"pong":true}"#
    );

    let snap = client.send_ok(r#"{"cmd":"snapshot"}"#);
    assert!(snap.contains(r#""tag":"ng-window""#), "snap: {snap}");
    assert!(snap.contains(r#""data-testid":"inc-btn""#), "snap: {snap}");

    let query = client.send_ok(r#"{"cmd":"query","id":"counter-label"}"#);
    assert!(query.contains(r#""text":"Ready:0""#), "query: {query}");

    client.send_ok(r#"{"cmd":"dispatch","target":"inc-btn","event":"clicked"}"#);
    client.send_ok(r#"{"cmd":"wait_for","text":"Ready:1","timeoutMs":1000}"#);

    client.eval_ok(r#"__NG_NATIVE__.setText("lbl", "EvalApplied");"#);
    client.send_ok(r#"{"cmd":"wait_for","text":"EvalApplied","timeoutMs":1000}"#);

    client.quit();
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn pokedex_angular_app_full_e2e_via_agnostic_harness() {
    if !ensure_pokedex_bundle_ready() {
        return;
    }
    let app_id = format!("e2e-pokedex-{}", std::process::id());
    clean_xdg_app_data(&app_id);

    let mut client = spawn_pokedex_harness(&app_id);
    let list_ready = client.send_ok(r#"{"cmd":"wait_for","text":"Bulbasaur","timeoutMs":6000}"#);
    assert!(list_ready.contains("Pikachu"), "list_ready: {list_ready}");

    let errs = client.eval_ok("JSON.stringify(__NG_NATIVE__.getSystemErrors())");
    assert!(
        errs.contains(r#""result":"[]""#),
        "Angular runtime errors: {errs}"
    );

    // Toggle theme Dark -> Light -> Dark
    client.send_ok(r#"{"cmd":"dispatch","target":"btn_toggle_theme","event":"clicked"}"#);
    client.send_ok(r#"{"cmd":"wait_for","text":"☾ Oscuro","timeoutMs":3000}"#);
    client.send_ok(r#"{"cmd":"dispatch","target":"btn_toggle_theme","event":"clicked"}"#);
    client.send_ok(r#"{"cmd":"wait_for","text":"☀ Claro","timeoutMs":3000}"#);

    // Filter catalog and open Pikachu detail
    client.send_ok(r#"{"cmd":"dispatch","target":"search_entry","event":"input","value":"pika"}"#);
    client.send_ok(r#"{"cmd":"wait_for","text":"Bulbasaur","absent":true,"timeoutMs":4000}"#);
    client.send_ok(r#"{"cmd":"dispatch","target":"open-poke-25","event":"clicked"}"#);
    let detail = client.send_ok(r#"{"cmd":"wait_for","text":"N.º 0025","timeoutMs":4000}"#);
    assert!(
        detail.contains("Static") && detail.contains("Ratón"),
        "detail: {detail}"
    );
    assert!(detail.contains("PS (35 / 255)"), "detail: {detail}");

    // Favorite & field note roundtrip + Favorites filter
    client.send_ok(r#"{"cmd":"dispatch","target":"btn_toggle_fav","event":"clicked"}"#);
    client.send_ok(r#"{"cmd":"wait_for","text":"En Favoritos","timeoutMs":4000}"#);
    client.send_ok(
        r#"{"cmd":"dispatch","target":"poke_notes","event":"input","value":"Nota E2E Pikachu"}"#,
    );
    client.send_ok(r#"{"cmd":"dispatch","target":"btn_save_note","event":"clicked"}"#);
    client.send_ok(r#"{"cmd":"wait_for","text":"Nota guardada","timeoutMs":4000}"#);

    client.send_ok(r#"{"cmd":"dispatch","target":"btn_back","event":"clicked"}"#);
    client.send_ok(r#"{"cmd":"wait_for","text":"Explorador Angular","timeoutMs":4000}"#);
    client.send_ok(r#"{"cmd":"dispatch","target":"search_entry","event":"input","value":""}"#);
    client.send_ok(r#"{"cmd":"wait_for","text":"Bulbasaur","timeoutMs":4000}"#);
    client.send_ok(r#"{"cmd":"dispatch","target":"btn_filter_favorites","event":"clicked"}"#);
    client.send_ok(r#"{"cmd":"wait_for","text":"Bulbasaur","absent":true,"timeoutMs":4000}"#);
    let favs = client.send_ok(r#"{"cmd":"wait_for","text":"★ Pikachu","timeoutMs":4000}"#);
    assert!(favs.contains("★ Pikachu"), "favs: {favs}");

    let final_errs = client.eval_ok("JSON.stringify(__NG_NATIVE__.getSystemErrors())");
    assert!(
        final_errs.contains(r#""result":"[]""#),
        "Final errors: {final_errs}"
    );
    client.quit();
    clean_xdg_app_data(&app_id);
}

#[test]
fn pokedex_sqlite_persistence_survives_process_restart_e2e() {
    if !ensure_pokedex_bundle_ready() {
        return;
    }
    let app_id = format!("e2e-persist-{}", std::process::id());
    clean_xdg_app_data(&app_id);

    // Process 1: save favorite + UTF-16 emoji note into real disk SQLite, then terminate
    let mut p1 = spawn_pokedex_harness(&app_id);
    p1.send_ok(r#"{"cmd":"wait_for","text":"Pikachu","timeoutMs":6000}"#);
    p1.send_ok(r#"{"cmd":"dispatch","target":"open-poke-25","event":"clicked"}"#);
    p1.send_ok(r#"{"cmd":"wait_for","text":"N.º 0025","timeoutMs":4000}"#);
    p1.send_ok(r#"{"cmd":"dispatch","target":"btn_toggle_fav","event":"clicked"}"#);
    p1.send_ok(r#"{"cmd":"wait_for","text":"En Favoritos","timeoutMs":4000}"#);
    p1.send_ok(
        r#"{"cmd":"dispatch","target":"poke_notes","event":"input","value":"Pikachu ⚡ Persistente 🚀"}"#,
    );
    p1.send_ok(r#"{"cmd":"dispatch","target":"btn_save_note","event":"clicked"}"#);
    p1.send_ok(r#"{"cmd":"wait_for","text":"Nota guardada","timeoutMs":4000}"#);
    p1.quit();

    // Process 2: fresh OS process with same app_id must restore SQLite state from disk
    let mut p2 = spawn_pokedex_harness(&app_id);
    let restored_list = p2.send_ok(r#"{"cmd":"wait_for","text":"★ Pikachu","timeoutMs":6000}"#);
    assert!(restored_list.contains("★ Pikachu"), "{restored_list}");
    p2.send_ok(r#"{"cmd":"dispatch","target":"open-poke-25","event":"clicked"}"#);
    let reopened =
        p2.send_ok(r#"{"cmd":"wait_for","text":"Pikachu ⚡ Persistente 🚀","timeoutMs":4000}"#);
    assert!(reopened.contains("En Favoritos"), "{reopened}");
    p2.quit();

    clean_xdg_app_data(&app_id);
}

#[test]
fn e2e_negative_security_permissions_and_hmr_gating() {
    let tmp_dir = make_e2e_temp_dir("ng-e2e-sec");
    let out_slash = tmp_dir.to_string_lossy().replace('\\', "/");
    let cfg_path = tmp_dir.join("ng-desktop.runtime.json");
    fs::write(
        &cfg_path,
        format!(
            r#"{{"projectName":"e2e-sec","version":"0.7.0","outputPath":"{out_slash}","permissions":{{"network":false,"sqlite":false,"filesystem":{{"xdg":false,"dialogs":false,"drop":false}}}}}}"#
        ),
    )
    .unwrap();
    fs::write(
        tmp_dir.join("app.bundle.js"),
        r#"
            __NG_NATIVE__.createElement("win", "ng-window");
            __NG_NATIVE__.createElement("root", "ng-box");
            __NG_NATIVE__.createElement("zone", "ng-box");
            __NG_NATIVE__.setAttribute("zone", "id", "drop_zone");
            __NG_NATIVE__.createElement("lbl", "ng-label");
            __NG_NATIVE__.setText("lbl", "Safe");
            __NG_NATIVE__.createElement("sec", "ng-label");
            __NG_NATIVE__.setText("sec", "Pending");
            __NG_NATIVE__.appendChild("zone", "lbl");
            __NG_NATIVE__.appendChild("root", "zone");
            __NG_NATIVE__.appendChild("root", "sec");
            __NG_NATIVE__.appendChild("win", "root");
            __NG_NATIVE__.listen("zone", "file-drop", function(evt) {
                __NG_NATIVE__.setText("lbl", "Leaked:" + evt.value);
            });
            const sqlOk = __NG_NATIVE__.sqliteExecute("CREATE TABLE t (id INT);");
            const fsRes = JSON.parse(__NG_NATIVE_HOST_SERVICE__(JSON.stringify({
                action: "fsResolvePath", kind: "data", relPath: "secret.txt"
            })));
            Promise.all([
                __NG_NATIVE__.httpRequestAsync("GET", "https://blocked.example.test/data"),
                __NG_NATIVE__.downloadImageAsync("https://blocked.example.test/icon.png")
            ]).then(([httpRes, imgRes]) => {
                __NG_NATIVE__.setText("sec", `http=${httpRes.status}|img=${imgRes.ok}|sql=${sqlOk}|fs=${fsRes.ok}`);
            });
        "#,
    )
    .unwrap();

    let cfg_arg = cfg_path.to_string_lossy().into_owned();
    let (mut client, banner) =
        TestIpcClient::spawn(&["--headless", "--test-ipc=stdio", "--config", &cfg_arg]);
    assert!(banner.contains(r#""status":"ready""#), "{banner}");
    let sec_line = client.send_ok(
        r#"{"cmd":"wait_for","text":"http=403|img=false|sql=false|fs=false","timeoutMs":3000}"#,
    );
    assert!(sec_line.contains("http=403|img=false|sql=false|fs=false"));

    client.send_ok(
        r#"{"cmd":"dispatch","target":"drop_zone","event":"file-drop","value":"/etc/shadow"}"#,
    );
    client.send_ok(r#"{"cmd":"tick","steps":3,"sleepMs":5}"#);
    let preview = client.send_ok(r#"{"cmd":"preview"}"#);
    assert!(preview.contains("Safe") && !preview.contains("/etc/shadow"));
    assert!(
        !tmp_dir.join("hmr.sock").exists(),
        "Production launch must not expose hmr.sock by default"
    );

    client.quit();
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn e2e_theme_css_dynamic_tree_and_keyup_workflow() {
    let tmp_dir = make_e2e_temp_dir("ng-e2e-theme-dom");
    let out_slash = tmp_dir.to_string_lossy().replace('\\', "/");
    let cfg_path = tmp_dir.join("ng-desktop.runtime.json");
    fs::write(
        &cfg_path,
        format!(
            r#"{{"projectName":"e2e-theme-dom","version":"0.7.0","outputPath":"{out_slash}","permissions":{{"sqlite":false,"network":false}}}}"#
        ),
    )
    .unwrap();
    fs::write(
        tmp_dir.join("app.bundle.js"),
        r##"
            __NG_NATIVE__.registerStyle("card-btn", "#1e293b", "#f8fafc", 10, 4, "radius=8 opacity=95");
            __NG_NATIVE__.registerStyle("card-btn", "#2563eb", "#ffffff", 10, 4, "pseudo=hover");
            __NG_NATIVE__.registerStyle("card-btn", "#e2e8f0", "#0f172a", 10, 4, "media-theme=light");
            __NG_NATIVE__.createElement("win", "ng-window");
            __NG_NATIVE__.createElement("box", "ng-box");
            __NG_NATIVE__.createElement("first", "ng-label");
            __NG_NATIVE__.setText("first", "First");
            __NG_NATIVE__.createElement("last", "ng-label");
            __NG_NATIVE__.setText("last", "Last");
            __NG_NATIVE__.createElement("mid", "ng-label");
            __NG_NATIVE__.setText("mid", "Middle");
            __NG_NATIVE__.createElement("act", "ng-button");
            __NG_NATIVE__.setAttribute("act", "id", "theme_btn");
            __NG_NATIVE__.addClass("act", "card-btn");
            __NG_NATIVE__.appendChild("box", "first");
            __NG_NATIVE__.appendChild("box", "last");
            __NG_NATIVE__.insertBefore("box", "mid", "last");
            __NG_NATIVE__.removeChild("box", "first");
            __NG_NATIVE__.appendChild("box", "act");
            __NG_NATIVE__.appendChild("win", "box");
            __NG_NATIVE__.listen("win", "keyup", function(evt) {
                __NG_NATIVE__.setSystemTheme(evt.value === "LightKey" ? "light" : "dark");
                __NG_NATIVE__.removeClass("act", "card-btn");
                __NG_NATIVE__.setText("act", "Theme=" + __NG_NATIVE__.getSystemTheme());
            });
        "##,
    )
    .unwrap();

    let cfg_arg = cfg_path.to_string_lossy().into_owned();
    let (mut client, _) =
        TestIpcClient::spawn(&["--headless", "--test-ipc=stdio", "--config", &cfg_arg]);
    let initial = client.send_ok(r#"{"cmd":"wait_for","text":"Middle","timeoutMs":2000}"#);
    assert!(!initial.contains("First"), "{initial}");

    client.send_ok(r#"{"cmd":"dispatch","target":"win","event":"keyup","value":"LightKey"}"#);
    let updated = client.send_ok(r#"{"cmd":"wait_for","text":"Theme=light","timeoutMs":2000}"#);
    assert!(updated.contains("Theme=light"), "{updated}");

    client.quit();
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn e2e_dual_js_engine_async_promises_and_utf16_surrogates() {
    let mut engines = vec!["quickjs"];
    if JsRuntime::from_engine_config("jsc")
        .is_ok_and(|rt| rt.js_engine().engine_name().contains("JavaScriptCore"))
    {
        engines.push("jsc");
    }

    for engine_kind in engines {
        let tmp_dir = make_e2e_temp_dir(&format!("ng-e2e-{engine_kind}"));
        let out_slash = tmp_dir.to_string_lossy().replace('\\', "/");
        let cfg_path = tmp_dir.join("ng-desktop.runtime.json");
        fs::write(
            &cfg_path,
            format!(
                r#"{{"projectName":"e2e-js","version":"0.7.0","jsEngine":"{engine_kind}","outputPath":"{out_slash}"}}"#
            ),
        )
        .unwrap();
        fs::write(
            tmp_dir.join("app.bundle.js"),
            r#"
                __NG_NATIVE__.createElement("win", "ng-window");
                __NG_NATIVE__.createElement("lbl", "ng-label");
                __NG_NATIVE__.setText("lbl", "Boot");
                __NG_NATIVE__.appendChild("win", "lbl");
                Promise.resolve("Pok\u00e9dex \uD83D\uDE80").then((label) => {
                    setTimeout(() => {
                        __NG_NATIVE__.setText("lbl", `${label} ⚡ áéíóú`);
                    }, 0);
                });
            "#,
        )
        .unwrap();

        let cfg_arg = cfg_path.to_string_lossy().into_owned();
        let (mut client, _) =
            TestIpcClient::spawn(&["--headless", "--test-ipc=stdio", "--config", &cfg_arg]);
        let res =
            client.send_ok(r#"{"cmd":"wait_for","text":"Pokédex 🚀 ⚡ áéíóú","timeoutMs":3000}"#);
        assert!(res.contains("Pokédex 🚀 ⚡ áéíóú"), "{engine_kind}: {res}");
        client.quit();
        let _ = fs::remove_dir_all(&tmp_dir);
    }
}

#[test]
fn e2e_cli_permissions_report_and_packaging_modes() {
    let tmp_dir = make_e2e_temp_dir("ng-e2e-cli");
    let primary_out = tmp_dir
        .join("dist/app-primary")
        .to_string_lossy()
        .replace('\\', "/");
    let docs_out = tmp_dir
        .join("dist/sibling-docs")
        .to_string_lossy()
        .replace('\\', "/");
    let angular_cfg = tmp_dir.join("angular.json");
    fs::write(
        &angular_cfg,
        format!(
            r#"{{
                "projects": {{
                    "app-primary": {{
                        "architect": {{
                            "build": {{ "options": {{ "outputPath": {{ "base": "{primary_out}" }} }} }},
                            "native": {{
                                "options": {{
                                    "version": "1.2.3",
                                    "permissions": {{ "capabilities": {{ "sqlite": true }} }}
                                }}
                            }}
                        }}
                    }},
                    "sibling-docs": {{
                        "architect": {{
                            "build": {{ "options": {{ "outputPath": "{docs_out}" }} }},
                            "native": {{
                                "options": {{
                                    "permissions": {{ "capabilities": {{ "network": true, "fullSystemAccess": true }} }}
                                }}
                            }}
                        }}
                    }}
                }}
            }}"#
        ),
    )
    .unwrap();

    let bin = env!("CARGO_BIN_EXE_ng-native-engine");
    let report = Command::new(bin)
        .args([
            "--config",
            angular_cfg.to_str().unwrap(),
            "--project",
            "app-primary",
            "--permissions-report",
        ])
        .output()
        .unwrap();
    assert!(report.status.success());
    let report_json = String::from_utf8_lossy(&report.stdout);
    assert!(report_json.contains(r#""sqlite": true"#), "{report_json}");
    assert!(report_json.contains(r#""network": false"#), "{report_json}");
    assert!(
        report_json.contains(r#""fullSystemAccess": false"#),
        "{report_json}"
    );

    let pkg = Command::new(bin)
        .args([
            "--config",
            angular_cfg.to_str().unwrap(),
            "--project",
            "app-primary",
            "--package-all",
        ])
        .output()
        .unwrap();
    assert!(
        pkg.status.success(),
        "{}",
        String::from_utf8_lossy(&pkg.stderr)
    );

    let appx =
        fs::read_to_string(tmp_dir.join("dist/app-primary/packages/windows/AppxManifest.xml"))
            .unwrap();
    assert!(
        appx.contains(r#"EntryPoint="Windows.PartialTrustApplication""#),
        "{appx}"
    );
    assert!(
        appx.contains(r#"uap10:TrustLevel="appContainer""#),
        "{appx}"
    );
    assert!(!appx.contains("NgDesktop.SandboxedApp"), "{appx}");
    assert!(!appx.contains("runFullTrust"), "{appx}");
    assert!(
        tmp_dir
            .join("dist/app-primary/packages/deb/DEBIAN/control")
            .is_file()
    );
    assert!(
        tmp_dir
            .join("dist/app-primary/packages/macos/app-primary.app/Contents/Info.plist")
            .is_file()
    );

    let _ = fs::remove_dir_all(&tmp_dir);
}

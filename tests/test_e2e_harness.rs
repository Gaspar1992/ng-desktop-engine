use ng_native_engine::config::AngularNativeConfig;
use ng_native_engine::desktop::{NativeFsManager, XdgDirKind};
use ng_native_engine::runtime::{JsEngineBackend, JsRuntime};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

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

#[test]
fn agnostic_headless_test_ipc_e2e_workflow() {
    let tmp_dir = std::env::temp_dir().join(format!("ng-e2e-agnostic-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let out_slash = tmp_dir.to_string_lossy().replace('\\', "/");
    let cfg_path = tmp_dir.join("ng-desktop.runtime.json");
    fs::write(
        &cfg_path,
        format!(
            r#"{{"projectName":"agnostic-e2e","version":"0.3.0","targetPlatform":"headless","outputPath":"{out_slash}","permissions":{{"sqlite":false,"network":false}}}}"#
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
    let after_click = client.send_ok(r#"{"cmd":"wait_for","text":"Ready:1","timeoutMs":1000}"#);
    assert!(
        after_click.contains("Ready:1"),
        "after_click: {after_click}"
    );

    client.send_ok(r#"{"cmd":"eval","code":"__NG_NATIVE__.setText(\"lbl\", \"EvalApplied\");"}"#);
    let after_eval = client.send_ok(r#"{"cmd":"wait_for","text":"EvalApplied","timeoutMs":1000}"#);
    assert!(
        after_eval.contains("EvalApplied"),
        "after_eval: {after_eval}"
    );

    client.quit();
    let _ = fs::remove_dir_all(&tmp_dir);
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

fn can_run_pokedex_e2e() -> bool {
    let dist_dir = PathBuf::from("dist/ng-linux-counter");
    if !dist_dir.join("app.bundle.js").is_file()
        || !dist_dir.join("server.bundle.js").is_file()
        || !dist_dir.join("ng-desktop.runtime.json").is_file()
    {
        return false;
    }
    let mut probe_cfg = AngularNativeConfig::default_standalone("e2e-jit-probe");
    probe_cfg.js_engine = String::from("jsc");
    JsRuntime::from_config(&probe_cfg).is_ok_and(|rt| rt.js_engine().is_jit_enabled())
}

fn clean_xdg_app_data(app_id: &str) {
    if let Ok(xdg_probe) =
        NativeFsManager::new(app_id).resolve_xdg_path(XdgDirKind::Data, "db.sqlite")
        && let Some(parent) = xdg_probe.parent()
    {
        let _ = fs::remove_dir_all(parent);
    }
}

#[test]
fn pokedex_angular_app_full_e2e_via_agnostic_harness() {
    if !can_run_pokedex_e2e() {
        return;
    }

    let app_id = format!("e2e-pokedex-{}", std::process::id());
    clean_xdg_app_data(&app_id);

    let (mut client, banner) = TestIpcClient::spawn(&[
        "--headless",
        "--test-ipc=stdio",
        "--defer-boot",
        "--config",
        "dist/ng-linux-counter/ng-desktop.runtime.json",
        "--project",
        &app_id,
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

    let list_ready = client.send_ok(r#"{"cmd":"wait_for","text":"Bulbasaur","timeoutMs":5000}"#);
    assert!(list_ready.contains("Pikachu"), "list_ready: {list_ready}");

    let errs = client
        .send_ok(r#"{"cmd":"eval","code":"JSON.stringify(__NG_NATIVE__.getSystemErrors())"}"#);
    assert!(
        errs.contains(r#""result":"[]""#),
        "Angular runtime errors: {errs}"
    );

    let open_btn_query = client.send_ok(r#"{"cmd":"query","id":"open-poke-25"}"#);
    assert!(
        open_btn_query.contains(r#""testId":"open-poke-25""#),
        "query: {open_btn_query}"
    );

    // Filter via search box -> Pikachu stays, Bulbasaur disappears
    client.send_ok(r#"{"cmd":"dispatch","target":"search_entry","event":"input","value":"pika"}"#);
    client.send_ok(r#"{"cmd":"wait_for","text":"Bulbasaur","absent":true,"timeoutMs":4000}"#);
    let filtered = client.send_ok(r#"{"cmd":"wait_for","text":"Pikachu","timeoutMs":4000}"#);
    assert!(filtered.contains("Pikachu"));

    // Open Pikachu detail sheet from filtered grid
    client.send_ok(r#"{"cmd":"dispatch","target":"open-poke-25","event":"clicked"}"#);
    let detail = client.send_ok(r#"{"cmd":"wait_for","text":"N.º 0025","timeoutMs":4000}"#);
    assert!(detail.contains("Static"), "detail: {detail}");
    assert!(detail.contains("Ratón"), "detail: {detail}");
    assert!(detail.contains("PS (35 / 255)"), "detail: {detail}");

    // Toggle favorite & persist field note via @ng-desktop/server SQLite
    client.send_ok(r#"{"cmd":"dispatch","target":"btn_toggle_fav","event":"clicked"}"#);
    client.send_ok(r#"{"cmd":"wait_for","text":"En Favoritos","timeoutMs":4000}"#);

    client.send_ok(
        r#"{"cmd":"dispatch","target":"poke_notes","event":"input","value":"Nota E2E Pikachu"}"#,
    );
    client.send_ok(r#"{"cmd":"dispatch","target":"btn_save_note","event":"clicked"}"#);
    client.send_ok(r#"{"cmd":"wait_for","text":"Nota guardada","timeoutMs":4000}"#);

    // Return to list, clear search, filter Favorites Only
    client.send_ok(r#"{"cmd":"dispatch","target":"btn_back","event":"clicked"}"#);
    client.send_ok(r#"{"cmd":"wait_for","text":"Explorador Angular","timeoutMs":4000}"#);
    client.send_ok(r#"{"cmd":"dispatch","target":"search_entry","event":"input","value":""}"#);
    client.send_ok(r#"{"cmd":"wait_for","text":"Bulbasaur","timeoutMs":4000}"#);

    client.send_ok(r#"{"cmd":"dispatch","target":"btn_filter_favorites","event":"clicked"}"#);
    client.send_ok(r#"{"cmd":"wait_for","text":"Bulbasaur","absent":true,"timeoutMs":4000}"#);
    let favs_preview = client.send_ok(r#"{"cmd":"wait_for","text":"Pikachu","timeoutMs":4000}"#);
    assert!(
        favs_preview.contains("★ Pikachu"),
        "favs_preview: {favs_preview}"
    );

    // Re-open Pikachu and verify SQLite state survived
    client.send_ok(r#"{"cmd":"dispatch","target":"open-poke-25","event":"clicked"}"#);
    let reopened =
        client.send_ok(r#"{"cmd":"wait_for","text":"Nota E2E Pikachu","timeoutMs":4000}"#);
    assert!(reopened.contains("En Favoritos"), "reopened: {reopened}");

    let final_errs = client
        .send_ok(r#"{"cmd":"eval","code":"JSON.stringify(__NG_NATIVE__.getSystemErrors())"}"#);
    assert!(
        final_errs.contains(r#""result":"[]""#),
        "Final errors: {final_errs}"
    );

    client.quit();
    clean_xdg_app_data(&app_id);
}

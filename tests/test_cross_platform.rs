use ng_native_engine::config::AngularNativeConfig;
use ng_native_engine::desktop::{
    DesktopServices, OsFileWatcherBackend, OsHttpPlatform, OsSqliteBackend, OsSystemEnvBackend,
};
use ng_native_engine::engine::{EventPayload, NativeEvent, RenderEngine};
use ng_native_engine::packager::{
    stage_all_distributions, stage_linux_distribution, stage_macos_distribution,
    stage_windows_distribution,
};
use ng_native_engine::platforms::{
    AnyPlatformBackend, GtkLinuxBackend, HeadlessBackend, MacOsAppKitBackend, PlatformBackend,
    WinUiBackend,
};
use ng_native_engine::runtime::JsRuntime;
use std::fs;

#[test]
fn cross_platform_factory_supports_linux_windows_and_macos() {
    let linux = AnyPlatformBackend::from_target("linux-gtk4").unwrap();
    for (tag, expected) in [
        ("ng-window", "AdwApplicationWindow"),
        ("ng-flow", "GtkFlowBox"),
        ("ng-spinner", "GtkSpinner"),
        ("ng-progress-bar", "GtkProgressBar"),
        ("ng-level-bar", "GtkLevelBar"),
        ("ng-paned", "GtkPaned"),
        ("ng-stack", "GtkStack"),
        ("ng-expander", "GtkExpander"),
        ("ng-text-view", "GtkTextView"),
        ("ng-password-entry", "GtkPasswordEntry"),
        ("ng-search-entry", "GtkSearchEntry"),
        ("ng-spin-button", "GtkSpinButton"),
        ("ng-clamp", "AdwClamp"),
        ("ng-dialog", "AdwDialog"),
        ("ng-list", "GtkListBox"),
        ("ng-table", "GtkListBox"),
        ("ng-popover", "GtkPopover"),
        ("ng-menu", "GtkPopover"),
    ] {
        assert_eq!(linux.map_native_widget(tag), expected);
    }

    let windows = AnyPlatformBackend::from_target("windows-winui").unwrap();
    for (tag, expected) in [
        ("ng-window", "Microsoft.UI.Xaml.Window"),
        ("ng-flow", "Microsoft.UI.Xaml.Controls.ItemsRepeater"),
        ("ng-paned", "Microsoft.UI.Xaml.Controls.SplitView"),
        ("ng-stack", "Microsoft.UI.Xaml.Controls.NavigationView"),
        ("ng-expander", "Microsoft.UI.Xaml.Controls.Expander"),
        ("ng-text-view", "Microsoft.UI.Xaml.Controls.RichEditBox"),
        (
            "ng-password-entry",
            "Microsoft.UI.Xaml.Controls.PasswordBox",
        ),
        (
            "ng-search-entry",
            "Microsoft.UI.Xaml.Controls.AutoSuggestBox",
        ),
        ("ng-spin-button", "Microsoft.UI.Xaml.Controls.NumberBox"),
        ("ng-clamp", "Microsoft.UI.Xaml.Controls.Grid"),
        ("ng-dialog", "Microsoft.UI.Xaml.Controls.ContentDialog"),
        ("ng-list", "Microsoft.UI.Xaml.Controls.ListView"),
        ("ng-table", "Microsoft.UI.Xaml.Controls.ListView"),
        ("ng-popover", "Microsoft.UI.Xaml.Controls.Flyout"),
        ("ng-menu", "Microsoft.UI.Xaml.Controls.Flyout"),
    ] {
        assert_eq!(windows.map_native_widget(tag), expected);
    }

    let macos = AnyPlatformBackend::from_target("macos-appkit").unwrap();
    for (tag, expected) in [
        ("ng-window", "NSWindow"),
        ("ng-flow", "NSCollectionView"),
        ("ng-spinner", "NSProgressIndicator"),
        ("ng-paned", "NSSplitView"),
        ("ng-stack", "NSTabView"),
        ("ng-expander", "NSDisclosureButton"),
        ("ng-text-view", "NSTextView"),
        ("ng-password-entry", "NSSecureTextField"),
        ("ng-search-entry", "NSSearchField"),
        ("ng-spin-button", "NSStepper"),
        ("ng-clamp", "NSBox"),
        ("ng-dialog", "NSPanel"),
        ("ng-list", "NSTableView"),
        ("ng-table", "NSTableView"),
        ("ng-popover", "NSPopover"),
        ("ng-menu", "NSPopover"),
    ] {
        assert_eq!(macos.map_native_widget(tag), expected);
    }
}

#[test]
fn phase11_packager_least_privilege_capabilities() {
    let mut cfg = AngularNativeConfig::default_standalone("phase11-sandbox-app");
    cfg.output_path = String::from("target/test-phase11-packager");
    cfg.icon_search_dir = String::from("target/test-phase11-packager/icons");
    cfg.permissions = ng_native_engine::config::AppPermissions {
        network: true,
        filesystem_dialogs: true,
        ..ng_native_engine::config::AppPermissions::minimal()
    };

    let win_artifacts = stage_windows_distribution(&cfg, "0.3.0").unwrap();
    let mac_artifacts = stage_macos_distribution(&cfg, "0.3.0").unwrap();

    let appx = fs::read_to_string(&win_artifacts[0]).unwrap();
    assert!(!appx.contains("runFullTrust"));
    assert!(appx.contains("internetClient"));
    let mac_entitlements = fs::read_to_string(&mac_artifacts[1]).unwrap();
    assert!(mac_entitlements.contains("com.apple.security.app-sandbox"));
    assert!(mac_entitlements.contains("com.apple.security.network.client"));
    let _ = fs::remove_dir_all(&cfg.output_path);
}

#[test]
fn phase6_linux_distribution_packager_and_winui_appkit_stateful_backends() {
    let cfg = AngularNativeConfig::from_file("tests/fixtures/sample-app/angular.json").unwrap();
    let artifacts = stage_linux_distribution(&cfg, "0.3.0").unwrap();
    assert_eq!(artifacts.len(), 7);
    for path in &artifacts {
        assert!(
            fs::metadata(path).is_ok(),
            "Missing staged Linux artifact: {path}"
        );
    }

    let win_artifacts = stage_windows_distribution(&cfg, "0.3.0").unwrap();
    assert_eq!(win_artifacts.len(), 4);
    let mac_artifacts = stage_macos_distribution(&cfg, "0.3.0").unwrap();
    assert_eq!(mac_artifacts.len(), 5);
    let all_artifacts = stage_all_distributions(&cfg, "0.3.0").unwrap();
    assert_eq!(all_artifacts.len(), 16);
    for path in &all_artifacts {
        assert!(
            fs::metadata(path).is_ok(),
            "Missing staged multi-OS artifact: {path}"
        );
    }

    let script = r"
        style card #242424 #ffffff 16 12
        create win ng-window
        text win CrossPlatform App
        icon win sample-desktop-app src/engine/default_icon.ico
        create box ng-box
        add_class box card
        attr box spacing 12px
        create entry ng-entry
        text entry NativeText
        append box entry
        append win box
    ";

    let mut winui_engine = RenderEngine::new(WinUiBackend::new());
    let mut winui_rt = JsRuntime::new();
    winui_rt.eval_tick(&mut winui_engine, script).unwrap();
    let win_id = winui_rt.resolve_var("win").unwrap();
    let box_id = winui_rt.resolve_var("box").unwrap();
    let entry_id = winui_rt.resolve_var("entry").unwrap();
    assert_eq!(winui_engine.backend().control_count(), 3);
    assert_eq!(winui_engine.backend().scene_state().style_count(), 1);
    assert_eq!(
        winui_engine
            .backend()
            .scene_state()
            .get_window_icon(win_id)
            .unwrap()
            .app_id,
        "sample-desktop-app"
    );
    assert_eq!(
        winui_engine
            .backend()
            .get_control(box_id)
            .unwrap()
            .style_classes
            .len(),
        1
    );
    assert_eq!(
        winui_engine
            .backend()
            .get_control(entry_id)
            .unwrap()
            .native_class,
        "Microsoft.UI.Xaml.Controls.TextBox"
    );
    assert_eq!(
        winui_engine.backend().get_control(entry_id).unwrap().text,
        "NativeText"
    );

    let mut appkit_engine = RenderEngine::new(MacOsAppKitBackend::new());
    let mut appkit_rt = JsRuntime::new();
    appkit_rt.eval_tick(&mut appkit_engine, script).unwrap();
    let mac_entry_id = appkit_rt.resolve_var("entry").unwrap();
    assert_eq!(appkit_engine.backend().view_count(), 3);
    assert_eq!(appkit_engine.backend().scene_state().style_count(), 1);
    assert_eq!(
        appkit_engine
            .backend()
            .get_view(mac_entry_id)
            .unwrap()
            .native_class,
        "NSTextFieldEditable"
    );
    assert_eq!(
        appkit_engine.backend().get_view(mac_entry_id).unwrap().text,
        "NativeText"
    );
    let _ = fs::remove_dir_all(&cfg.output_path);
}

#[test]
fn winui_and_appkit_retain_nested_grid_and_form_hierarchies() {
    let script = r"
        create win ng-window
        create grid ng-grid
        create sw ng-switch
        attr sw active true
        create chk ng-checkbox
        text chk Enable Feature
        create sel ng-select
        attr sel options Alpha,Beta,Gamma
        append grid sw
        append grid chk
        append grid sel
        append win grid
    ";

    let mut winui_engine = RenderEngine::new(WinUiBackend::new());
    let mut winui_rt = JsRuntime::new();
    winui_rt.eval_tick(&mut winui_engine, script).unwrap();
    let grid_id = winui_rt.resolve_var("grid").unwrap();
    let sw_id = winui_rt.resolve_var("sw").unwrap();

    let grid_node = winui_engine.backend().get_control(grid_id).unwrap();
    assert_eq!(grid_node.native_class, "Microsoft.UI.Xaml.Controls.Grid");
    assert_eq!(grid_node.children.len(), 3);

    let sw_node = winui_engine.backend().get_control(sw_id).unwrap();
    assert_eq!(
        sw_node.native_class,
        "Microsoft.UI.Xaml.Controls.ToggleSwitch"
    );
    assert_eq!(sw_node.attributes.get("active"), Some(&"true".to_string()));
}

#[test]
fn all_29_primitives_have_cross_platform_mapping_parity() {
    let primitives = [
        "ng-window",
        "ng-header-bar",
        "ng-scroll",
        "ng-box",
        "ng-grid",
        "ng-separator",
        "ng-label",
        "ng-button",
        "ng-entry",
        "ng-switch",
        "ng-checkbox",
        "ng-slider",
        "ng-select",
        "ng-image",
        "ng-level-bar",
        "ng-progress-bar",
        "ng-spinner",
        "ng-flow",
        "ng-paned",
        "ng-stack",
        "ng-expander",
        "ng-text-view",
        "ng-password-entry",
        "ng-search-entry",
        "ng-spin-button",
        "ng-clamp",
        "ng-dialog",
        "ng-list",
        "ng-popover",
        "ng-anchor",
    ];

    let headless = HeadlessBackend::default();
    let linux = GtkLinuxBackend::new_headless();
    let winui = WinUiBackend::new();
    let appkit = MacOsAppKitBackend::new();

    for tag in primitives {
        assert_ne!(
            headless.map_native_widget(tag),
            "NativeWidget",
            "HeadlessBackend missing mapping for {tag}"
        );
        assert_ne!(
            linux.map_native_widget(tag),
            "GtkWidget",
            "GtkLinuxBackend missing mapping for {tag}"
        );
        if tag != "ng-anchor" {
            assert_ne!(
                winui.map_native_widget(tag),
                "Microsoft.UI.Xaml.UIElement",
                "WinUiBackend missing mapping for {tag}"
            );
            assert_ne!(
                appkit.map_native_widget(tag),
                "NSView",
                "MacOsAppKitBackend missing mapping for {tag}"
            );
        }
    }
}

#[test]
fn nivel3_attributes_events_and_os_drivers_have_cross_platform_parity() {
    let script = r"
        create win ng-window
        create scroll ng-scroll
        attr scroll scroll-to 0,240
        attr scroll scroll-anchor section-favorites
        create dropbox ng-box
        attr dropbox drop-target true
        append scroll dropbox
        append win scroll
    ";

    // 1. Linux GTK4 Backend parity
    let mut linux_engine = RenderEngine::new(GtkLinuxBackend::new_headless());
    let mut linux_rt = JsRuntime::new();
    linux_rt.eval_tick(&mut linux_engine, script).unwrap();
    let l_scroll = linux_rt.resolve_var("scroll").unwrap();
    let l_drop = linux_rt.resolve_var("dropbox").unwrap();
    assert!(linux_engine.backend().is_drop_target(l_drop));
    assert_eq!(
        linux_engine.backend().scroll_position(l_scroll),
        Some((0, 240))
    );
    assert_eq!(
        linux_engine.backend().scroll_anchor(l_scroll),
        Some("section-favorites")
    );

    // 2. Windows WinUI 3 Backend parity + native event queue
    let mut winui_engine = RenderEngine::new(WinUiBackend::new());
    let mut winui_rt = JsRuntime::new();
    winui_rt.eval_tick(&mut winui_engine, script).unwrap();
    let w_scroll = winui_rt.resolve_var("scroll").unwrap();
    let w_drop = winui_rt.resolve_var("dropbox").unwrap();
    let w_scroll_node = winui_engine.backend().get_control(w_scroll).unwrap();
    let w_drop_node = winui_engine.backend().get_control(w_drop).unwrap();
    assert!(w_drop_node.drop_target);
    assert_eq!(w_scroll_node.scroll_position, Some((0, 240)));
    assert_eq!(
        w_scroll_node.scroll_anchor.as_deref(),
        Some("section-favorites")
    );
    winui_engine.backend().queue_native_event(NativeEvent {
        node: w_drop,
        event_name: String::from("file-drop"),
        payload: EventPayload::Text(String::from("C:\\Users\\Dev\\favorites.json")),
    });
    let w_events = winui_engine.backend().step_native_events();
    assert_eq!(w_events.len(), 1);
    assert_eq!(w_events[0].event_name, "file-drop");

    // 3. macOS AppKit Backend parity + native event queue
    let mut appkit_engine = RenderEngine::new(MacOsAppKitBackend::new());
    let mut appkit_rt = JsRuntime::new();
    appkit_rt.eval_tick(&mut appkit_engine, script).unwrap();
    let m_scroll = appkit_rt.resolve_var("scroll").unwrap();
    let m_drop = appkit_rt.resolve_var("dropbox").unwrap();
    let m_scroll_node = appkit_engine.backend().get_view(m_scroll).unwrap();
    let m_drop_node = appkit_engine.backend().get_view(m_drop).unwrap();
    assert!(m_drop_node.drop_target);
    assert_eq!(m_scroll_node.scroll_position, Some((0, 240)));
    assert_eq!(
        m_scroll_node.scroll_anchor.as_deref(),
        Some("section-favorites")
    );
    appkit_engine.backend().queue_native_event(NativeEvent {
        node: m_drop,
        event_name: String::from("file-drop"),
        payload: EventPayload::Text(String::from("/Users/dev/favorites.json")),
    });
    let m_events = appkit_engine.backend().step_native_events();
    assert_eq!(m_events.len(), 1);
    assert_eq!(m_events[0].event_name, "file-drop");

    // 4. OS Service Driver Parity (HTTP, FileWatcher, SQLite, SystemEnv)
    let desktop = DesktopServices::new();
    assert_ne!(OsHttpPlatform::current().as_str(), "");
    assert_eq!(
        desktop.fs_manager().backend(),
        OsFileWatcherBackend::current()
    );
    assert_eq!(desktop.sqlite().backend(), OsSqliteBackend::current());
    assert_ne!(desktop.sqlite().backend().library_name(), "");
    assert_eq!(
        desktop.system_env().backend(),
        OsSystemEnvBackend::current()
    );
    assert_ne!(desktop.system_env().backend().as_str(), "");
}

#[test]
fn macos_appkit_and_windows_winui_dynamic_ffi_drivers_and_window_lifecycle() {
    let create_and_destroy_script = r"
        create win ng-window
        create lbl ng-label
        text lbl Ready
        create btn ng-button
        text btn Run
        append win lbl
        append win btn
    ";

    let mut winui = RenderEngine::new(WinUiBackend::new());
    let mut win_rt = JsRuntime::new();
    win_rt
        .eval_tick(&mut winui, create_and_destroy_script)
        .unwrap();
    let win_id = win_rt.resolve_var("win").unwrap();
    let btn_id = win_rt.resolve_var("btn").unwrap();
    assert_eq!(winui.backend().open_window_count(), 1);
    let win_binding = winui.backend().get_control_binding(win_id).unwrap();
    assert_eq!(win_binding.xaml_class, "Microsoft.UI.Xaml.Window");
    assert_eq!(win_binding.win32_class, "Window");
    let btn_binding = winui.backend().get_control_binding(btn_id).unwrap();
    assert_eq!(btn_binding.win32_class, "BUTTON");

    if cfg!(target_os = "windows") {
        assert!(winui.backend().is_ffi_loaded());
        assert!(winui.backend().resolved_symbol_count() >= 6);
        assert!(win_binding.symbol_bound);
    }

    win_rt.eval_tick(&mut winui, "destroy win").unwrap();
    assert_eq!(winui.backend().open_window_count(), 0);
    assert!(winui.backend().get_control_binding(win_id).is_none());

    let mut appkit = RenderEngine::new(MacOsAppKitBackend::new());
    let mut mac_rt = JsRuntime::new();
    mac_rt
        .eval_tick(&mut appkit, create_and_destroy_script)
        .unwrap();
    let mac_win = mac_rt.resolve_var("win").unwrap();
    let mac_btn = mac_rt.resolve_var("btn").unwrap();
    assert_eq!(appkit.backend().open_window_count(), 1);
    let mac_win_binding = appkit.backend().get_view_binding(mac_win).unwrap();
    assert_eq!(mac_win_binding.objc_class, "NSWindow");
    let mac_btn_binding = appkit.backend().get_view_binding(mac_btn).unwrap();
    assert_eq!(mac_btn_binding.objc_class, "NSButton");
    assert_eq!(mac_btn_binding.primary_selector, "setTitle:");

    if cfg!(target_os = "macos") {
        assert!(appkit.backend().is_ffi_loaded());
        assert!(appkit.backend().resolved_class_count() >= 10);
        assert!(mac_win_binding.class_resolved);
        assert!(mac_btn_binding.selector_resolved);
    }

    mac_rt.eval_tick(&mut appkit, "destroy win").unwrap();
    assert_eq!(appkit.backend().open_window_count(), 0);
    assert!(appkit.backend().get_view_binding(mac_win).is_none());
}

#[test]
fn winui_and_appkit_headless_guards_and_native_live_drivers() {
    let win_headless = WinUiBackend::new_headless();
    assert!(!win_headless.is_native_display_active());
    assert_eq!(win_headless.live_handle_count(), 0);

    let mac_headless = MacOsAppKitBackend::new_headless();
    assert!(!mac_headless.is_native_display_active());
    assert_eq!(mac_headless.live_handle_count(), 0);

    #[cfg(target_os = "windows")]
    if std::env::var("NG_DESKTOP_HEADLESS").is_err() {
        let mut live_win = RenderEngine::new(WinUiBackend::new());
        let w = live_win.create_element("ng-window").unwrap();
        live_win.set_text(w, "Win32 Live Test").unwrap();
        let root = live_win.create_element("ng-box").unwrap();
        let lbl = live_win.create_element("ng-label").unwrap();
        live_win.set_text(lbl, "Live Win32 Label").unwrap();
        let btn = live_win.create_element("ng-button").unwrap();
        live_win.set_text(btn, "Run").unwrap();
        let dlg = live_win.create_element("ng-dialog").unwrap();
        live_win.set_text(dlg, "Win32 Dialog").unwrap();
        live_win.append_child(w, root).unwrap();
        live_win.append_child(root, lbl).unwrap();
        live_win.append_child(root, btn).unwrap();
        live_win.flush_frame();

        assert!(live_win.backend().is_ffi_loaded());
        assert!(live_win.backend().open_window_count() >= 1);
        if live_win.backend().is_native_display_active() {
            assert!(live_win.backend().live_handle_count() >= 4);
            let _ = live_win.backend().step_native_events();
        }
        live_win.destroy_node(dlg).unwrap();
        live_win.destroy_node(w).unwrap();
        live_win.flush_frame();
        assert_eq!(live_win.backend().live_handle_count(), 0);
        assert_eq!(live_win.backend().open_window_count(), 0);
    }

    #[cfg(target_os = "macos")]
    {
        let mac_live = MacOsAppKitBackend::new();
        assert!(mac_live.is_ffi_loaded());
        assert_eq!(
            mac_live
                .probe_objc_string_roundtrip("NgDesktop AppKit")
                .as_deref(),
            Some("NgDesktop AppKit")
        );
    }
}

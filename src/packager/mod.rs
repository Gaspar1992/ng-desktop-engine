use crate::config::{AngularNativeConfig, AppPermissions};
use std::fs;
use std::path::Path;

/// Generates a Freedesktop `.desktop` launcher specification for Linux desktop environments
/// (GNOME, KDE Plasma, Hyprland, Sway, Cosmic).
#[must_use]
pub fn generate_desktop_entry(app_id: &str, display_name: &str) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName={display_name}\nExec=/usr/bin/{app_id}\nIcon={app_id}\nStartupWMClass={app_id}\nCategories=Utility;Development;\nTerminal=false\n"
    )
}

/// Generates a minimal Debian (`DEBIAN/control`) metadata specification using default permissions.
#[must_use]
pub fn generate_debian_control(app_id: &str, version: &str, arch: &str) -> String {
    generate_debian_control_with_permissions(app_id, version, arch, &AppPermissions::minimal())
}

/// Generates a Debian (`DEBIAN/control`) specification whose runtime library `Depends`
/// are derived strictly from the application's static permission manifest.
#[must_use]
pub fn generate_debian_control_with_permissions(
    app_id: &str,
    version: &str,
    arch: &str,
    perms: &AppPermissions,
) -> String {
    let mut depends = vec!["libgtk-4-1", "libjavascriptcoregtk-6.0-1"];
    if perms.network {
        depends.push("libsoup-3.0-0");
    }
    if perms.sqlite {
        depends.push("libsqlite3-0");
    }
    let perm_labels = perms.summary_labels();
    let perm_summary = if perm_labels.is_empty() {
        String::from("minimal")
    } else {
        perm_labels.join(",")
    };
    format!(
        "Package: {app_id}\nVersion: {version}\nSection: utils\nPriority: optional\nArchitecture: {arch}\nDepends: {}\nX-NgDesktop-Permissions: {perm_summary}\nMaintainer: NgDesktop Engine <maintainers@ng-native.dev>\nDescription: Native Angular 22 desktop application powered by ng-desktop-engine (Rust + GTK4)\n",
        depends.join(", ")
    )
}

/// Generates a least-privilege Linux `AppArmor` profile (`etc/apparmor.d/<app_id>`) aligned with
/// the build-time capability manifest.
#[must_use]
pub fn generate_apparmor_profile(app_id: &str, perms: &AppPermissions) -> String {
    let mut rules = vec![
        format!("  /usr/bin/{app_id} mr,"),
        format!("  /usr/share/{app_id}/** r,"),
        String::from("  /usr/lib/** mr,"),
        String::from("  /usr/share/icons/** r,"),
    ];

    if perms.network {
        rules.push(String::from("  network inet stream,"));
        rules.push(String::from("  network inet6 stream,"));
        rules.push(String::from("  network netlink dgram,"));
        rules.push(String::from("  /run/systemd/resolve/** r,"));
    }
    if perms.sqlite || perms.filesystem_xdg {
        rules.push(format!("  owner @{{HOME}}/.local/share/{app_id}/** rwk,"));
        rules.push(format!("  owner @{{HOME}}/.config/{app_id}/** rwk,"));
        rules.push(format!("  owner @{{HOME}}/.cache/{app_id}/** rwk,"));
    }
    if perms.filesystem_dialogs || perms.filesystem_drop {
        rules.push(String::from(
            "  dbus (send) interface=org.freedesktop.portal.FileChooser,",
        ));
    }
    if perms.notifications {
        rules.push(String::from(
            "  dbus (send) interface=org.freedesktop.Notifications,",
        ));
    }
    if perms.full_system_access {
        rules.push(String::from("  /** rwk,"));
    }

    format!(
        "#include <tunables/global>\n\nprofile ngdesktop.{app_id} /usr/bin/{app_id} {{\n{}\n}}\n",
        rules.join("\n")
    )
}

/// Generates an `AppImage` `AppRun` entrypoint script.
#[must_use]
pub fn generate_appimage_apprun(app_id: &str) -> String {
    format!(
        "#!/bin/sh\nSELF=$(readlink -f \"$0\")\nHERE=${{SELF%/*}}\nexport PATH=\"${{HERE}}/usr/bin:${{PATH}}\"\nexec \"${{HERE}}/usr/bin/{app_id}\" \"$@\"\n"
    )
}

/// Generates a minimal Flatpak (`org.gnome.Platform`) manifest without unrestricted host access.
#[must_use]
pub fn generate_flatpak_manifest(app_id: &str) -> String {
    generate_flatpak_manifest_with_permissions(app_id, &AppPermissions::minimal())
}

/// Generates a Flatpak manifest granting ONLY the sandbox finish-args required by `perms`.
#[must_use]
pub fn generate_flatpak_manifest_with_permissions(app_id: &str, perms: &AppPermissions) -> String {
    let flatpak_id = format!("dev.ngnative.{}", app_id.replace('-', "_"));
    let mut finish_args = vec![
        String::from("  - --share=ipc"),
        String::from("  - --socket=wayland"),
        String::from("  - --socket=fallback-x11"),
        String::from("  - --device=dri"),
    ];
    if perms.network {
        finish_args.push(String::from("  - --share=network"));
    }
    if perms.notifications {
        finish_args.push(String::from(
            "  - --talk-name=org.freedesktop.Notifications",
        ));
    }
    if perms.filesystem_xdg && !perms.full_system_access {
        finish_args.push(format!("  - --filesystem=xdg-config/{app_id}:create"));
        finish_args.push(format!("  - --filesystem=xdg-data/{app_id}:create"));
    }
    if perms.full_system_access {
        finish_args.push(String::from("  - --filesystem=host"));
    }
    format!(
        "app-id: {flatpak_id}\nruntime: org.gnome.Platform\nruntime-version: '47'\nsdk: org.gnome.Sdk\ncommand: {app_id}\nfinish-args:\n{}\n",
        finish_args.join("\n")
    )
}

/// Generates an embeddable runtime configuration payload (`ng-desktop.runtime.json`) so packaged
/// executables operate standalone with their workspace identity and least-privilege permissions.
#[must_use]
pub fn generate_runtime_manifest(cfg: &AngularNativeConfig, version: &str) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"schemaVersion\": 1,\n",
            "  \"projectName\": \"{}\",\n",
            "  \"version\": \"{}\",\n",
            "  \"targetPlatform\": \"{}\",\n",
            "  \"outputPath\": \"{}\",\n",
            "  \"iconSearchDir\": \"{}\",\n",
            "  \"permissions\": {}\n",
            "}}"
        ),
        cfg.project_name,
        version,
        cfg.target_platform,
        cfg.output_path,
        cfg.icon_search_dir,
        cfg.permissions.to_json()
    )
}

/// Stages Linux distribution artifacts (`.deb`, `AppArmor` profile, `AppImage`, `Flatpak`, and
/// runtime/permission manifests) inside `<output_path>/packages/`.
pub fn stage_linux_distribution(
    cfg: &AngularNativeConfig,
    version: &str,
) -> Result<Vec<String>, String> {
    let base_dir = Path::new(&cfg.output_path).join("packages");
    let deb_control_dir = base_dir.join("deb/DEBIAN");
    let deb_apps_dir = base_dir.join("deb/usr/share/applications");
    let deb_share_dir = base_dir.join(format!("deb/usr/share/{}", cfg.project_name));
    let deb_apparmor_dir = base_dir.join("deb/etc/apparmor.d");
    let appimage_dir = base_dir.join("appimage");
    let flatpak_dir = base_dir.join("flatpak");

    for dir in [
        &deb_control_dir,
        &deb_apps_dir,
        &deb_share_dir,
        &deb_apparmor_dir,
        &appimage_dir,
        &flatpak_dir,
    ] {
        fs::create_dir_all(dir)
            .map_err(|e| format!("Failed to create dir {}: {e}", dir.display()))?;
    }

    let control_path = deb_control_dir.join("control");
    let desktop_path = deb_apps_dir.join(format!("{}.desktop", cfg.project_name));
    let apparmor_path = deb_apparmor_dir.join(&cfg.project_name);
    let perm_path = deb_share_dir.join("permissions.manifest.json");
    let runtime_path = deb_share_dir.join("ng-desktop.runtime.json");
    let apprun_path = appimage_dir.join("AppRun");
    let flatpak_path = flatpak_dir.join(format!("{}.yml", cfg.project_name));

    fs::write(
        &control_path,
        generate_debian_control_with_permissions(
            &cfg.project_name,
            version,
            "amd64",
            &cfg.permissions,
        ),
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        &desktop_path,
        generate_desktop_entry(&cfg.project_name, &cfg.project_name),
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        &apparmor_path,
        generate_apparmor_profile(&cfg.project_name, &cfg.permissions),
    )
    .map_err(|e| e.to_string())?;
    fs::write(&perm_path, cfg.permissions.to_json()).map_err(|e| e.to_string())?;
    fs::write(&runtime_path, generate_runtime_manifest(cfg, version)).map_err(|e| e.to_string())?;
    fs::write(&apprun_path, generate_appimage_apprun(&cfg.project_name))
        .map_err(|e| e.to_string())?;
    fs::write(
        &flatpak_path,
        generate_flatpak_manifest_with_permissions(&cfg.project_name, &cfg.permissions),
    )
    .map_err(|e| e.to_string())?;

    Ok(vec![
        control_path.to_string_lossy().into_owned(),
        desktop_path.to_string_lossy().into_owned(),
        apparmor_path.to_string_lossy().into_owned(),
        perm_path.to_string_lossy().into_owned(),
        runtime_path.to_string_lossy().into_owned(),
        apprun_path.to_string_lossy().into_owned(),
        flatpak_path.to_string_lossy().into_owned(),
    ])
}

/// Generates a minimal sandboxed Windows `AppxManifest.xml` without `runFullTrust`.
#[must_use]
pub fn generate_windows_appx_manifest(app_id: &str, display_name: &str, version: &str) -> String {
    generate_windows_appx_manifest_with_permissions(
        app_id,
        display_name,
        version,
        &AppPermissions::minimal(),
    )
}

/// Generates a Windows `AppxManifest.xml` emitting only the capabilities required by `perms`.
/// Avoids declaring restricted `runFullTrust` unless `perms.full_system_access` is explicitly true.
#[must_use]
pub fn generate_windows_appx_manifest_with_permissions(
    app_id: &str,
    display_name: &str,
    version: &str,
    perms: &AppPermissions,
) -> String {
    let msix_version = if version.matches('.').count() == 2 {
        format!("{version}.0")
    } else {
        String::from(version)
    };
    let mut capabilities = Vec::new();
    if perms.network {
        capabilities.push("    <Capability Name=\"internetClient\" />");
    }
    if perms.notifications {
        capabilities.push("    <uap:Capability Name=\"userNotificationListener\" />");
    }
    if perms.full_system_access {
        capabilities.push("    <rescap:Capability Name=\"runFullTrust\" />");
    }
    let cap_block = if capabilities.is_empty() {
        String::from("  <Capabilities />")
    } else {
        format!(
            "  <Capabilities>\n{}\n  </Capabilities>",
            capabilities.join("\n")
        )
    };
    let rescap_ns = if perms.full_system_access {
        "\n         xmlns:rescap=\"http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities\""
    } else {
        ""
    };
    let entry_point = if perms.full_system_access {
        "Windows.FullTrustApplication"
    } else {
        "NgDesktop.SandboxedApp"
    };

    format!(
        r##"<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"{rescap_ns}>
  <Identity Name="NgNative.{app_id}" Publisher="CN=NgNative" Version="{msix_version}" ProcessorArchitecture="x64" />
  <Properties>
    <DisplayName>{display_name}</DisplayName>
    <PublisherDisplayName>NgNative Engine</PublisherDisplayName>
  </Properties>
{cap_block}
  <Applications>
    <Application Id="App" Executable="{app_id}.exe" EntryPoint="{entry_point}">
      <uap:VisualElements DisplayName="{display_name}" Description="Angular 22 Native WinUI 3 Desktop App"
                          BackgroundColor="#1e1e1e" Square150x150Logo="Assets\Logo.png" Square44x44Logo="Assets\SmallLogo.png" />
    </Application>
  </Applications>
</Package>
"##
    )
}

/// Generates a portable Windows batch launcher (`run-winui3.cmd`).
#[must_use]
pub fn generate_windows_launcher_batch(app_id: &str) -> String {
    format!(
        "@echo off\r\nREM Launch Angular 22 Native WinUI 3 Application ({app_id})\r\n\"%~dp0{app_id}.exe\" --project=\"{app_id}\" %*\r\n"
    )
}

/// Stages Windows `WinUI 3` distribution artifacts inside `<output_path>/packages/windows/`.
pub fn stage_windows_distribution(
    cfg: &AngularNativeConfig,
    version: &str,
) -> Result<Vec<String>, String> {
    let win_dir = Path::new(&cfg.output_path).join("packages/windows");
    fs::create_dir_all(&win_dir)
        .map_err(|e| format!("Failed to create dir {}: {e}", win_dir.display()))?;

    let manifest_path = win_dir.join("AppxManifest.xml");
    let launcher_path = win_dir.join("run-winui3.cmd");
    let perm_path = win_dir.join("permissions.manifest.json");
    let runtime_path = win_dir.join("ng-desktop.runtime.json");

    fs::write(
        &manifest_path,
        generate_windows_appx_manifest_with_permissions(
            &cfg.project_name,
            &cfg.project_name,
            version,
            &cfg.permissions,
        ),
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        &launcher_path,
        generate_windows_launcher_batch(&cfg.project_name),
    )
    .map_err(|e| e.to_string())?;
    fs::write(&perm_path, cfg.permissions.to_json()).map_err(|e| e.to_string())?;
    fs::write(&runtime_path, generate_runtime_manifest(cfg, version)).map_err(|e| e.to_string())?;

    Ok(vec![
        manifest_path.to_string_lossy().into_owned(),
        launcher_path.to_string_lossy().into_owned(),
        perm_path.to_string_lossy().into_owned(),
        runtime_path.to_string_lossy().into_owned(),
    ])
}

/// Generates a macOS `AppKit` / Cocoa `.app` bundle `Info.plist`.
#[must_use]
pub fn generate_macos_info_plist(app_id: &str, display_name: &str, version: &str) -> String {
    let bundle_id = format!("dev.ngnative.{}", app_id.replace('_', "-"));
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key>
  <string>{bundle_id}</string>
  <key>CFBundleName</key>
  <string>{display_name}</string>
  <key>CFBundleExecutable</key>
  <string>{app_id}</string>
  <key>CFBundleShortVersionString</key>
  <string>{version}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
"#
    )
}

/// Generates a macOS App Sandbox entitlements property list (`Entitlements.plist`) with least-privilege
/// permissions inferred at build time.
#[must_use]
pub fn generate_macos_entitlements_plist(app_id: &str, perms: &AppPermissions) -> String {
    let mut entries = vec![format!(
        "  <key>com.apple.security.app-sandbox</key>\n  <{}/>",
        !perms.full_system_access
    )];
    if perms.network {
        entries.push(String::from(
            "  <key>com.apple.security.network.client</key>\n  <true/>",
        ));
    }
    if perms.filesystem_dialogs || perms.filesystem_drop {
        entries.push(String::from(
            "  <key>com.apple.security.files.user-selected.read-write</key>\n  <true/>",
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<!-- Entitlements for dev.ngnative.{app_id} -->\n<plist version=\"1.0\">\n<dict>\n{}\n</dict>\n</plist>\n",
        entries.join("\n")
    )
}

/// Stages macOS `AppKit` `.app` bundle artifacts inside `<output_path>/packages/macos/<app_id>.app/`.
pub fn stage_macos_distribution(
    cfg: &AngularNativeConfig,
    version: &str,
) -> Result<Vec<String>, String> {
    let app_bundle = Path::new(&cfg.output_path)
        .join("packages/macos")
        .join(format!("{}.app/Contents", cfg.project_name));
    let macos_bin_dir = app_bundle.join("MacOS");
    let resources_dir = app_bundle.join("Resources");

    for dir in [&macos_bin_dir, &resources_dir] {
        fs::create_dir_all(dir)
            .map_err(|e| format!("Failed to create dir {}: {e}", dir.display()))?;
    }

    let plist_path = app_bundle.join("Info.plist");
    let entitlements_path = app_bundle.join("Entitlements.plist");
    let launcher_path = macos_bin_dir.join("run-appkit.sh");
    let perm_path = resources_dir.join("permissions.manifest.json");
    let runtime_path = resources_dir.join("ng-desktop.runtime.json");

    fs::write(
        &plist_path,
        generate_macos_info_plist(&cfg.project_name, &cfg.project_name, version),
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        &entitlements_path,
        generate_macos_entitlements_plist(&cfg.project_name, &cfg.permissions),
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        &launcher_path,
        format!(
            "#!/bin/sh\nDIR=\"$(cd \"$(dirname \"$0\")\" && pwd)\"\nexec \"${{DIR}}/{}\" --project=\"{}\" \"$@\"\n",
            cfg.project_name, cfg.project_name
        ),
    )
    .map_err(|e| e.to_string())?;
    fs::write(&perm_path, cfg.permissions.to_json()).map_err(|e| e.to_string())?;
    fs::write(&runtime_path, generate_runtime_manifest(cfg, version)).map_err(|e| e.to_string())?;

    Ok(vec![
        plist_path.to_string_lossy().into_owned(),
        entitlements_path.to_string_lossy().into_owned(),
        launcher_path.to_string_lossy().into_owned(),
        perm_path.to_string_lossy().into_owned(),
        runtime_path.to_string_lossy().into_owned(),
    ])
}

/// Stages distribution artifacts for all three desktop platforms (Linux GTK4, Windows `WinUI 3`, macOS `AppKit`).
pub fn stage_all_distributions(
    cfg: &AngularNativeConfig,
    version: &str,
) -> Result<Vec<String>, String> {
    let mut all = stage_linux_distribution(cfg, version)?;
    all.extend(stage_windows_distribution(cfg, version)?);
    all.extend(stage_macos_distribution(cfg, version)?);
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_least_privilege_linux_windows_and_macos_manifests_by_default() {
        let minimal = AppPermissions::minimal();

        let control =
            generate_debian_control_with_permissions("my-notes", "0.4.0", "amd64", &minimal);
        assert!(control.contains("Package: my-notes"));
        assert!(control.contains("Depends: libgtk-4-1, libjavascriptcoregtk-6.0-1\n"));
        assert!(!control.contains("libsoup"));
        assert!(control.contains("X-NgDesktop-Permissions: minimal"));

        let desktop = generate_desktop_entry("my-notes", "My Notes");
        assert!(desktop.contains("StartupWMClass=my-notes"));

        let apparmor = generate_apparmor_profile("my-notes", &minimal);
        assert!(apparmor.contains("profile ngdesktop.my-notes /usr/bin/my-notes"));
        assert!(!apparmor.contains("network inet stream"));
        assert!(!apparmor.contains("/** rwk"));

        let apprun = generate_appimage_apprun("my-notes");
        assert!(apprun.contains("exec \"${HERE}/usr/bin/my-notes\""));

        let flatpak = generate_flatpak_manifest_with_permissions("my-notes", &minimal);
        assert!(flatpak.contains("app-id: dev.ngnative.my_notes"));
        assert!(flatpak.contains("--socket=wayland"));
        assert!(!flatpak.contains("--share=network"));
        assert!(!flatpak.contains("--filesystem=host"));

        let win_manifest = generate_windows_appx_manifest_with_permissions(
            "my-notes", "My Notes", "0.4.0", &minimal,
        );
        assert!(win_manifest.contains("Identity Name=\"NgNative.my-notes\""));
        assert!(win_manifest.contains("Version=\"0.4.0.0\""));
        assert!(win_manifest.contains("Executable=\"my-notes.exe\""));
        assert!(!win_manifest.contains("runFullTrust"));
        assert!(!win_manifest.contains("internetClient"));

        let mac_plist = generate_macos_info_plist("my-notes", "My Notes", "0.4.0");
        assert!(mac_plist.contains("<string>dev.ngnative.my-notes</string>"));
        assert!(mac_plist.contains("<key>NSHighResolutionCapable</key>"));

        let mac_entitlements = generate_macos_entitlements_plist("my-notes", &minimal);
        assert!(mac_entitlements.contains("<key>com.apple.security.app-sandbox</key>\n  <true/>"));
        assert!(!mac_entitlements.contains("com.apple.security.network.client"));
    }

    #[test]
    fn emits_only_requested_network_and_dialog_capabilities_without_full_trust() {
        let perms = AppPermissions {
            network: true,
            filesystem_dialogs: true,
            ..AppPermissions::minimal()
        };

        let flatpak = generate_flatpak_manifest_with_permissions("sample-desktop-app", &perms);
        assert!(flatpak.contains("--share=network"));
        assert!(!flatpak.contains("--filesystem=host"));

        let win_manifest = generate_windows_appx_manifest_with_permissions(
            "sample-desktop-app",
            "Sample Desktop App",
            "0.4.0",
            &perms,
        );
        assert!(win_manifest.contains("<Capability Name=\"internetClient\" />"));
        assert!(!win_manifest.contains("runFullTrust"));

        let mac_entitlements = generate_macos_entitlements_plist("sample-desktop-app", &perms);
        assert!(mac_entitlements.contains("com.apple.security.app-sandbox"));
        assert!(mac_entitlements.contains("com.apple.security.network.client"));
        assert!(mac_entitlements.contains("com.apple.security.files.user-selected.read-write"));
    }
}

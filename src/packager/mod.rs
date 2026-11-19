use crate::config::AngularNativeConfig;
use std::fs;
use std::path::Path;

/// Generates a Freedesktop `.desktop` launcher specification for Linux desktop environments
/// (GNOME, KDE Plasma, Hyprland, Sway, Cosmic).
pub fn generate_desktop_entry(app_id: &str, display_name: &str) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName={display_name}\nExec=/usr/bin/{app_id}\nIcon={app_id}\nStartupWMClass={app_id}\nCategories=Utility;Development;\nTerminal=false\n"
    )
}

/// Generates a Debian (`DEBIAN/control`) metadata specification for `.deb` binary packaging.
pub fn generate_debian_control(app_id: &str, version: &str, arch: &str) -> String {
    format!(
        "Package: {app_id}\nVersion: {version}\nSection: utils\nPriority: optional\nArchitecture: {arch}\nDepends: libgtk-4-1, libjavascriptcoregtk-6.0-1\nMaintainer: NgNative Engine <maintainers@ng-native.dev>\nDescription: Native Angular 22 desktop application powered by ng-native-engine (Rust + GTK4)\n"
    )
}

/// Generates an `AppImage` `AppRun` entrypoint script.
pub fn generate_appimage_apprun(app_id: &str) -> String {
    format!(
        "#!/bin/sh\nSELF=$(readlink -f \"$0\")\nHERE=${{SELF%/*}}\nexport PATH=\"${{HERE}}/usr/bin:${{PATH}}\"\nexec \"${{HERE}}/usr/bin/{app_id}\" \"$@\"\n"
    )
}

/// Generates a Flatpak (`org.freedesktop.Platform` / `org.gnome.Platform`) manifest.
pub fn generate_flatpak_manifest(app_id: &str) -> String {
    let flatpak_id = format!("dev.ngnative.{}", app_id.replace('-', "_"));
    format!(
        "app-id: {flatpak_id}\nruntime: org.gnome.Platform\nruntime-version: '47'\nsdk: org.gnome.Sdk\ncommand: {app_id}\nfinish-args:\n  - --share=ipc\n  - --socket=wayland\n  - --socket=fallback-x11\n  - --device=dri\n"
    )
}

/// Stages Linux distribution artifacts (`.deb` layout, `AppImage` layout, and `Flatpak` manifest)
/// inside `<output_path>/packages/`.
pub fn stage_linux_distribution(
    cfg: &AngularNativeConfig,
    version: &str,
) -> Result<Vec<String>, String> {
    let base_dir = Path::new(&cfg.output_path).join("packages");
    let deb_control_dir = base_dir.join("deb/DEBIAN");
    let deb_apps_dir = base_dir.join("deb/usr/share/applications");
    let appimage_dir = base_dir.join("appimage");
    let flatpak_dir = base_dir.join("flatpak");

    for dir in [&deb_control_dir, &deb_apps_dir, &appimage_dir, &flatpak_dir] {
        fs::create_dir_all(dir)
            .map_err(|e| format!("Failed to create dir {}: {e}", dir.display()))?;
    }

    let control_path = deb_control_dir.join("control");
    let desktop_path = deb_apps_dir.join(format!("{}.desktop", cfg.project_name));
    let apprun_path = appimage_dir.join("AppRun");
    let flatpak_path = flatpak_dir.join(format!("{}.yml", cfg.project_name));

    fs::write(
        &control_path,
        generate_debian_control(&cfg.project_name, version, "amd64"),
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        &desktop_path,
        generate_desktop_entry(&cfg.project_name, &cfg.project_name),
    )
    .map_err(|e| e.to_string())?;
    fs::write(&apprun_path, generate_appimage_apprun(&cfg.project_name))
        .map_err(|e| e.to_string())?;
    fs::write(&flatpak_path, generate_flatpak_manifest(&cfg.project_name))
        .map_err(|e| e.to_string())?;

    Ok(vec![
        control_path.to_string_lossy().into_owned(),
        desktop_path.to_string_lossy().into_owned(),
        apprun_path.to_string_lossy().into_owned(),
        flatpak_path.to_string_lossy().into_owned(),
    ])
}

/// Generates a Windows 11 `WinUI 3` / MSIX `AppxManifest.xml` specification.
pub fn generate_windows_appx_manifest(app_id: &str, display_name: &str, version: &str) -> String {
    let msix_version = if version.matches('.').count() == 2 {
        format!("{version}.0")
    } else {
        String::from(version)
    };
    format!(
        r##"<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
         xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities">
  <Identity Name="NgNative.{app_id}" Publisher="CN=NgNative" Version="{msix_version}" ProcessorArchitecture="x64" />
  <Properties>
    <DisplayName>{display_name}</DisplayName>
    <PublisherDisplayName>NgNative Engine</PublisherDisplayName>
  </Properties>
  <Capabilities>
    <rescap:Capability Name="runFullTrust" />
  </Capabilities>
  <Applications>
    <Application Id="App" Executable="ng-native-engine.exe" EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements DisplayName="{display_name}" Description="Angular 22 Native WinUI 3 Desktop App"
                          BackgroundColor="#1e1e1e" Square150x150Logo="Assets\Logo.png" Square44x44Logo="Assets\SmallLogo.png" />
    </Application>
  </Applications>
</Package>
"##
    )
}

/// Generates a portable Windows batch launcher (`run-winui3.cmd`).
pub fn generate_windows_launcher_batch(app_id: &str) -> String {
    format!(
        "@echo off\r\nREM Launch Angular 22 Native WinUI 3 Application ({app_id})\r\n\"%~dp0ng-native-engine.exe\" %*\r\n"
    )
}

/// Stages Windows `WinUI 3` distribution artifacts (`AppxManifest.xml` + `run-winui3.cmd`)
/// inside `<output_path>/packages/windows/`.
pub fn stage_windows_distribution(
    cfg: &AngularNativeConfig,
    version: &str,
) -> Result<Vec<String>, String> {
    let win_dir = Path::new(&cfg.output_path).join("packages/windows");
    fs::create_dir_all(&win_dir)
        .map_err(|e| format!("Failed to create dir {}: {e}", win_dir.display()))?;

    let manifest_path = win_dir.join("AppxManifest.xml");
    let launcher_path = win_dir.join("run-winui3.cmd");

    fs::write(
        &manifest_path,
        generate_windows_appx_manifest(&cfg.project_name, &cfg.project_name, version),
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        &launcher_path,
        generate_windows_launcher_batch(&cfg.project_name),
    )
    .map_err(|e| e.to_string())?;

    Ok(vec![
        manifest_path.to_string_lossy().into_owned(),
        launcher_path.to_string_lossy().into_owned(),
    ])
}

/// Generates a macOS `AppKit` / Cocoa `.app` bundle `Info.plist`.
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
  <string>ng-native-engine</string>
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

/// Stages macOS `AppKit` `.app` bundle artifacts (`Contents/Info.plist` + `Contents/MacOS/run-appkit.sh`)
/// inside `<output_path>/packages/macos/<app_id>.app/`.
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
    let launcher_path = macos_bin_dir.join("run-appkit.sh");

    fs::write(
        &plist_path,
        generate_macos_info_plist(&cfg.project_name, &cfg.project_name, version),
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        &launcher_path,
        "#!/bin/sh\nDIR=\"$(cd \"$(dirname \"$0\")\" && pwd)\"\nexec \"${DIR}/ng-native-engine\" \"$@\"\n",
    )
    .map_err(|e| e.to_string())?;

    Ok(vec![
        plist_path.to_string_lossy().into_owned(),
        launcher_path.to_string_lossy().into_owned(),
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
    fn generates_valid_linux_deb_desktop_appimage_and_flatpak_specs() {
        let control = generate_debian_control("ng-linux-counter", "0.1.0", "amd64");
        assert!(control.contains("Package: ng-linux-counter"));
        assert!(control.contains("Depends: libgtk-4-1, libjavascriptcoregtk-6.0-1"));

        let desktop = generate_desktop_entry("ng-linux-counter", "NgNative Counter");
        assert!(desktop.contains("StartupWMClass=ng-linux-counter"));

        let apprun = generate_appimage_apprun("ng-linux-counter");
        assert!(apprun.contains("exec \"${HERE}/usr/bin/ng-linux-counter\""));

        let flatpak = generate_flatpak_manifest("ng-linux-counter");
        assert!(flatpak.contains("app-id: dev.ngnative.ng_linux_counter"));
        assert!(flatpak.contains("--socket=wayland"));

        let win_manifest =
            generate_windows_appx_manifest("ng-linux-counter", "NgNative Counter", "0.2.0");
        assert!(win_manifest.contains("Identity Name=\"NgNative.ng-linux-counter\""));
        assert!(win_manifest.contains("Version=\"0.2.0.0\""));
        assert!(win_manifest.contains("Executable=\"ng-native-engine.exe\""));

        let mac_plist = generate_macos_info_plist("ng-linux-counter", "NgNative Counter", "0.2.0");
        assert!(mac_plist.contains("<string>dev.ngnative.ng-linux-counter</string>"));
        assert!(mac_plist.contains("<key>NSHighResolutionCapable</key>"));
    }
}

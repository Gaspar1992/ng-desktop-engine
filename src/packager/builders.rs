use super::png::encode_solid_png;
use super::{
    generate_apparmor_profile, generate_appimage_apprun, generate_debian_control_with_permissions,
    generate_desktop_entry, generate_flatpak_manifest_with_permissions,
    generate_macos_entitlements_plist, generate_macos_info_plist, generate_runtime_manifest,
    generate_windows_appx_manifest_with_permissions, generate_windows_launcher_batch,
};
use crate::config::AngularNativeConfig;
use std::cmp::Reverse;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Supported native desktop installer formats built by `ng-desktop-engine`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallerFormat {
    Deb,
    AppImage,
    Flatpak,
    Msix,
    Dmg,
}

impl InstallerFormat {
    /// Parses a CLI installer format token (`deb`, `appimage`, `flatpak`, `msix`, `dmg`).
    #[must_use]
    pub fn from_str_lossy(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "deb" => Some(Self::Deb),
            "appimage" => Some(Self::AppImage),
            "flatpak" => Some(Self::Flatpak),
            "msix" | "appx" => Some(Self::Msix),
            "dmg" => Some(Self::Dmg),
            _ => None,
        }
    }

    /// Canonical lowercase identifier for CLI reporting.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Deb => "deb",
            Self::AppImage => "appimage",
            Self::Flatpak => "flatpak",
            Self::Msix => "msix",
            Self::Dmg => "dmg",
        }
    }

    /// Default installer format(s) appropriate for the active build host OS.
    #[must_use]
    pub const fn host_defaults() -> &'static [Self] {
        if cfg!(target_os = "windows") {
            &[Self::Msix]
        } else if cfg!(target_os = "macos") {
            &[Self::Dmg]
        } else {
            &[Self::Deb, Self::AppImage, Self::Flatpak]
        }
    }

    /// Every supported installer target across Linux, Windows, and macOS.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Deb,
            Self::AppImage,
            Self::Flatpak,
            Self::Msix,
            Self::Dmg,
        ]
    }
}

/// Optional flags controlling installer packaging.
#[derive(Debug, Clone, Copy, Default)]
pub struct InstallerBuildOptions<'a> {
    pub binary_override: Option<&'a Path>,
    pub require_bundle: bool,
}

/// Outcome of compiling or staging one OS installer artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallerBuildStatus {
    Built {
        artifact_path: String,
        size_bytes: u64,
    },
    StagedOnly {
        staging_dir: String,
        missing_tool: String,
        command_hint: String,
    },
}

/// Structured result emitted for each requested installer format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallerBuildResult {
    pub format: InstallerFormat,
    pub status: InstallerBuildStatus,
}

/// Alias kept for backward readability across CLI callers.
pub type BuiltInstaller = InstallerBuildResult;

impl InstallerBuildResult {
    /// Returns `true` when the native OS packager tool compiled the final artifact.
    #[must_use]
    pub const fn is_built(&self) -> bool {
        matches!(self.status, InstallerBuildStatus::Built { .. })
    }
}

/// Maps the current Rust host architecture to its Debian package architecture name.
#[must_use]
pub const fn debian_arch_for_host() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86") {
        "i386"
    } else {
        "amd64"
    }
}

/// Maps the current Rust host architecture to its Windows MSIX `ProcessorArchitecture` token.
#[must_use]
pub const fn msix_arch_for_host() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else {
        "x64"
    }
}

/// Maps the current Rust host architecture to its macOS DMG filename tag.
#[must_use]
pub const fn macos_arch_for_host() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    }
}

fn appimage_arch_for_host() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x86_64"
    }
}

fn validate_project_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains("..")
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(format!(
            "Invalid project name '{name}': expected non-empty ASCII alphanumeric, '-', '_', or '.' without '..'"
        ));
    }
    Ok(())
}

fn sanitize_debian_package_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower == '_' {
            out.push('-');
        } else if lower.is_ascii_alphanumeric() || matches!(lower, '+' | '.' | '-') {
            out.push(lower);
        }
    }
    if out.is_empty() || !out.as_bytes()[0].is_ascii_alphanumeric() {
        format!("ng-{out}")
    } else {
        out
    }
}

#[cfg(unix)]
fn set_unix_mode(path: &Path, mode: u32) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)
        .map_err(|e| format!("Failed reading permissions for {}: {e}", path.display()))?
        .permissions();
    perms.set_mode(mode);
    fs::set_permissions(path, perms)
        .map_err(|e| format!("Failed setting mode {mode:o} on {}: {e}", path.display()))
}

#[cfg(not(unix))]
#[allow(clippy::unnecessary_wraps)]
fn set_unix_mode(_path: &Path, _mode: u32) -> Result<(), String> {
    Ok(())
}

fn mark_executable(path: &Path) -> Result<(), String> {
    set_unix_mode(path, 0o755)
}

fn resolve_engine_binary(binary_override: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(explicit) = binary_override {
        if explicit.is_file() {
            return Ok(explicit.to_path_buf());
        }
        return Err(format!(
            "Specified engine binary '{}' does not exist",
            explicit.display()
        ));
    }
    env::current_exe().map_err(|e| format!("Failed resolving current engine executable: {e}"))
}

fn copy_bundle_files(
    src_dir: &Path,
    dst_share_dir: &Path,
    require_bundle: bool,
) -> Result<(), String> {
    let main_ngb = src_dir.join("main.ngb");
    let app_bundle = src_dir.join("app.bundle.js");
    if require_bundle && !main_ngb.is_file() && !app_bundle.is_file() {
        return Err(format!(
            "Compiled application bundle missing in '{}'. Run 'ng-desktop build' or 'pnpm run build' first.",
            src_dir.display()
        ));
    }

    if main_ngb.is_file() {
        fs::copy(&main_ngb, dst_share_dir.join("main.ngb")).map_err(|e| e.to_string())?;
    } else {
        fs::write(dst_share_dir.join("main.ngb"), "# Placeholder bundle\n")
            .map_err(|e| e.to_string())?;
    }
    for optional_name in ["app.bundle.js", "server.bundle.js"] {
        let src = src_dir.join(optional_name);
        if src.is_file() {
            fs::copy(&src, dst_share_dir.join(optional_name)).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn write_runtime_metadata(
    cfg: &AngularNativeConfig,
    version: &str,
    dst_share_dir: &Path,
) -> Result<(), String> {
    fs::write(
        dst_share_dir.join("permissions.manifest.json"),
        cfg.permissions.to_json(),
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        dst_share_dir.join("ng-desktop.runtime.json"),
        generate_runtime_manifest(cfg, version),
    )
    .map_err(|e| e.to_string())
}

fn dir_size_bytes(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    if meta.is_file() {
        return meta.len();
    }
    if !meta.is_dir() {
        return 0;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| dir_size_bytes(&entry.path()))
        .sum()
}

fn find_icon_png(cfg: &AngularNativeConfig, preferred_size: &str) -> Option<PathBuf> {
    let icon_root = Path::new(&cfg.icon_search_dir);
    for size in [preferred_size, "256x256", "128x128", "64x64", "48x48"] {
        let candidate = icon_root
            .join("hicolor")
            .join(size)
            .join("apps")
            .join(format!("{}.png", cfg.project_name));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn write_png_or_fallback(
    cfg: &AngularNativeConfig,
    dst: &Path,
    preferred_size: &str,
    fallback_dim: u32,
) -> Result<(), String> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if let Some(src_png) = find_icon_png(cfg, preferred_size) {
        fs::copy(src_png, dst).map_err(|e| e.to_string())?;
        return Ok(());
    }
    let bytes = encode_solid_png(fallback_dim, fallback_dim, [225, 29, 72, 255]);
    fs::write(dst, bytes).map_err(|e| e.to_string())
}

fn command_exists(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .output()
        .is_ok_and(|out| out.status.success() || !out.stdout.is_empty() || !out.stderr.is_empty())
}

fn run_tool(cmd: &mut Command, tool_name: &str) -> Result<(), String> {
    let out = cmd
        .output()
        .map_err(|e| format!("Failed invoking {tool_name}: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "{tool_name} failed: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(())
}

fn finish_built_artifact(
    format: InstallerFormat,
    artifact_path: &Path,
) -> Result<InstallerBuildResult, String> {
    let size_bytes = fs::metadata(artifact_path)
        .map_err(|e| format!("Missing output {}: {e}", artifact_path.display()))?
        .len();
    Ok(InstallerBuildResult {
        format,
        status: InstallerBuildStatus::Built {
            artifact_path: artifact_path.to_string_lossy().into_owned(),
            size_bytes,
        },
    })
}

fn build_deb_installer(
    cfg: &AngularNativeConfig,
    version: &str,
    engine_bin: &Path,
    opts: InstallerBuildOptions<'_>,
) -> Result<InstallerBuildResult, String> {
    let base_dir = Path::new(&cfg.output_path).join("packages");
    let deb_dir = base_dir.join("deb");
    let bin_dir = deb_dir.join("usr/bin");
    let share_dir = deb_dir.join(format!("usr/share/{}", cfg.project_name));
    let control_dir = deb_dir.join("DEBIAN");
    let apps_dir = deb_dir.join("usr/share/applications");
    let icons_dir = deb_dir.join("usr/share/icons/hicolor/256x256/apps");
    let apparmor_dir = deb_dir.join("etc/apparmor.d");

    for dir in [
        &bin_dir,
        &share_dir,
        &control_dir,
        &apps_dir,
        &icons_dir,
        &apparmor_dir,
    ] {
        fs::create_dir_all(dir).map_err(|e| format!("Failed creating {}: {e}", dir.display()))?;
    }

    let arch = debian_arch_for_host();
    let deb_pkg_name = sanitize_debian_package_name(&cfg.project_name);
    let dst_bin = bin_dir.join(&cfg.project_name);
    fs::copy(engine_bin, &dst_bin).map_err(|e| e.to_string())?;
    mark_executable(&dst_bin)?;

    copy_bundle_files(Path::new(&cfg.output_path), &share_dir, opts.require_bundle)?;
    write_runtime_metadata(cfg, version, &share_dir)?;
    write_png_or_fallback(
        cfg,
        &icons_dir.join(format!("{}.png", cfg.project_name)),
        "256x256",
        64,
    )?;
    fs::write(
        apps_dir.join(format!("{}.desktop", cfg.project_name)),
        generate_desktop_entry(&cfg.project_name, &cfg.project_name),
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        apparmor_dir.join(&cfg.project_name),
        generate_apparmor_profile(&cfg.project_name, &cfg.permissions),
    )
    .map_err(|e| e.to_string())?;

    let installed_bytes =
        dir_size_bytes(&deb_dir.join("usr")) + dir_size_bytes(&deb_dir.join("etc"));
    let installed_kib = installed_bytes.div_ceil(1024).max(1);
    let mut control =
        generate_debian_control_with_permissions(&deb_pkg_name, version, arch, &cfg.permissions);
    let _ = writeln!(control, "Installed-Size: {installed_kib}");

    set_unix_mode(&control_dir, 0o755)?;
    let control_path = control_dir.join("control");
    fs::write(&control_path, control).map_err(|e| e.to_string())?;
    set_unix_mode(&control_path, 0o644)?;

    let artifact_path = base_dir.join(format!("{deb_pkg_name}_{version}_{arch}.deb"));
    if !command_exists("dpkg-deb") {
        return Ok(InstallerBuildResult {
            format: InstallerFormat::Deb,
            status: InstallerBuildStatus::StagedOnly {
                staging_dir: deb_dir.to_string_lossy().into_owned(),
                missing_tool: String::from("dpkg-deb"),
                command_hint: format!(
                    "dpkg-deb --build --root-owner-group {} {}",
                    deb_dir.display(),
                    artifact_path.display()
                ),
            },
        });
    }

    let mut cmd = Command::new("dpkg-deb");
    cmd.args(["--build", "--root-owner-group"])
        .arg(&deb_dir)
        .arg(&artifact_path);
    run_tool(&mut cmd, "dpkg-deb")?;
    finish_built_artifact(InstallerFormat::Deb, &artifact_path)
}

fn build_appimage_installer(
    cfg: &AngularNativeConfig,
    version: &str,
    engine_bin: &Path,
    opts: InstallerBuildOptions<'_>,
) -> Result<InstallerBuildResult, String> {
    let base_dir = Path::new(&cfg.output_path).join("packages");
    let app_dir = base_dir
        .join("appimage")
        .join(format!("{}.AppDir", cfg.project_name));
    let bin_dir = app_dir.join("usr/bin");
    let share_dir = app_dir.join(format!("usr/share/{}", cfg.project_name));
    for dir in [&bin_dir, &share_dir] {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }

    let dst_bin = bin_dir.join(&cfg.project_name);
    fs::copy(engine_bin, &dst_bin).map_err(|e| e.to_string())?;
    mark_executable(&dst_bin)?;

    copy_bundle_files(Path::new(&cfg.output_path), &share_dir, opts.require_bundle)?;
    write_runtime_metadata(cfg, version, &share_dir)?;

    let apprun_path = app_dir.join("AppRun");
    fs::write(&apprun_path, generate_appimage_apprun(&cfg.project_name))
        .map_err(|e| e.to_string())?;
    mark_executable(&apprun_path)?;

    fs::write(
        app_dir.join(format!("{}.desktop", cfg.project_name)),
        generate_desktop_entry(&cfg.project_name, &cfg.project_name),
    )
    .map_err(|e| e.to_string())?;
    let icon_path = app_dir.join(format!("{}.png", cfg.project_name));
    write_png_or_fallback(cfg, &icon_path, "256x256", 64)?;
    fs::copy(&icon_path, app_dir.join(".DirIcon")).map_err(|e| e.to_string())?;

    let arch = appimage_arch_for_host();
    let artifact_path = base_dir.join(format!("{}-{version}-{arch}.AppImage", cfg.project_name));
    let tool = env::var("APPIMAGETOOL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| command_exists("appimagetool").then(|| String::from("appimagetool")));

    let Some(appimagetool) = tool else {
        return Ok(InstallerBuildResult {
            format: InstallerFormat::AppImage,
            status: InstallerBuildStatus::StagedOnly {
                staging_dir: app_dir.to_string_lossy().into_owned(),
                missing_tool: String::from("appimagetool"),
                command_hint: format!(
                    "ARCH={arch} appimagetool --no-appstream {} {}",
                    app_dir.display(),
                    artifact_path.display()
                ),
            },
        });
    };

    let mut cmd = Command::new(&appimagetool);
    cmd.env("ARCH", arch)
        .arg("--no-appstream")
        .arg(&app_dir)
        .arg(&artifact_path);
    run_tool(&mut cmd, "appimagetool")?;
    finish_built_artifact(InstallerFormat::AppImage, &artifact_path)
}

fn build_flatpak_installer(
    cfg: &AngularNativeConfig,
    version: &str,
    engine_bin: &Path,
    opts: InstallerBuildOptions<'_>,
) -> Result<InstallerBuildResult, String> {
    let base_dir = Path::new(&cfg.output_path).join("packages");
    let flatpak_dir = base_dir.join("flatpak");
    fs::create_dir_all(&flatpak_dir).map_err(|e| e.to_string())?;

    let dst_bin = flatpak_dir.join(&cfg.project_name);
    fs::copy(engine_bin, &dst_bin).map_err(|e| e.to_string())?;
    mark_executable(&dst_bin)?;
    copy_bundle_files(
        Path::new(&cfg.output_path),
        &flatpak_dir,
        opts.require_bundle,
    )?;
    write_runtime_metadata(cfg, version, &flatpak_dir)?;

    let manifest_path = flatpak_dir.join(format!("{}.yml", cfg.project_name));
    fs::write(
        &manifest_path,
        generate_flatpak_manifest_with_permissions(&cfg.project_name, &cfg.permissions),
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        flatpak_dir.join(format!("{}.desktop", cfg.project_name)),
        generate_desktop_entry(&cfg.project_name, &cfg.project_name),
    )
    .map_err(|e| e.to_string())?;
    write_png_or_fallback(
        cfg,
        &flatpak_dir.join(format!("{}.png", cfg.project_name)),
        "256x256",
        64,
    )?;

    let arch = appimage_arch_for_host();
    let flatpak_id = format!("dev.ngnative.{}", cfg.project_name.replace('-', "_"));
    let bundle_path = base_dir.join(format!("{}-{version}-{arch}.flatpak", cfg.project_name));
    if !command_exists("flatpak-builder") || !command_exists("flatpak") {
        return Ok(InstallerBuildResult {
            format: InstallerFormat::Flatpak,
            status: InstallerBuildStatus::StagedOnly {
                staging_dir: flatpak_dir.to_string_lossy().into_owned(),
                missing_tool: String::from("flatpak-builder"),
                command_hint: format!(
                    "flatpak-builder --force-clean --repo={0}/repo {0}/build {1} && flatpak build-bundle {0}/repo {2} {flatpak_id}",
                    flatpak_dir.display(),
                    manifest_path.display(),
                    bundle_path.display()
                ),
            },
        });
    }

    let build_dir = flatpak_dir.join("build");
    let repo_dir = flatpak_dir.join("repo");
    let mut builder_cmd = Command::new("flatpak-builder");
    builder_cmd
        .args(["--force-clean"])
        .arg(format!("--repo={}", repo_dir.display()))
        .arg(&build_dir)
        .arg(&manifest_path);
    run_tool(&mut builder_cmd, "flatpak-builder")?;

    let mut bundle_cmd = Command::new("flatpak");
    bundle_cmd
        .arg("build-bundle")
        .arg(&repo_dir)
        .arg(&bundle_path)
        .arg(&flatpak_id);
    run_tool(&mut bundle_cmd, "flatpak build-bundle")?;
    finish_built_artifact(InstallerFormat::Flatpak, &bundle_path)
}

fn parse_kit_semver_key(name: &str) -> (u32, u32, u32, u32) {
    let mut nums = name.split('.').map(|part| part.parse::<u32>().unwrap_or(0));
    (
        nums.next().unwrap_or(0),
        nums.next().unwrap_or(0),
        nums.next().unwrap_or(0),
        nums.next().unwrap_or(0),
    )
}

fn sorted_kit_version_dirs(kits_bin: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(kits_bin) else {
        return Vec::new();
    };
    let mut versions: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    versions.sort_by_key(|p| {
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        Reverse(parse_kit_semver_key(name))
    });
    versions
}

fn find_makeappx_binary() -> Option<PathBuf> {
    if let Ok(explicit) = env::var("MAKEAPPX_PATH") {
        let p = PathBuf::from(explicit.trim());
        if p.is_file() {
            return Some(p);
        }
    }
    if command_exists("makeappx") {
        return Some(PathBuf::from("makeappx"));
    }
    let arch = msix_arch_for_host();
    for ver_dir in sorted_kit_version_dirs(Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin"))
    {
        for sub in [arch, "x64", "arm64"] {
            let candidate = ver_dir.join(sub).join("makeappx.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn build_msix_installer(
    cfg: &AngularNativeConfig,
    version: &str,
    engine_bin: &Path,
    opts: InstallerBuildOptions<'_>,
) -> Result<InstallerBuildResult, String> {
    let base_dir = Path::new(&cfg.output_path).join("packages");
    let win_dir = base_dir.join("windows");
    let assets_dir = win_dir.join("Assets");
    fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;

    let arch = msix_arch_for_host();
    fs::copy(
        engine_bin,
        win_dir.join(format!("{}.exe", cfg.project_name)),
    )
    .map_err(|e| e.to_string())?;
    copy_bundle_files(Path::new(&cfg.output_path), &win_dir, opts.require_bundle)?;
    write_runtime_metadata(cfg, version, &win_dir)?;

    let mut manifest = generate_windows_appx_manifest_with_permissions(
        &cfg.project_name,
        &cfg.project_name,
        version,
        &cfg.permissions,
    );
    if arch != "x64" {
        manifest = manifest.replace(
            "ProcessorArchitecture=\"x64\"",
            &format!("ProcessorArchitecture=\"{arch}\""),
        );
    }
    fs::write(win_dir.join("AppxManifest.xml"), manifest).map_err(|e| e.to_string())?;
    fs::write(
        win_dir.join("run-winui3.cmd"),
        generate_windows_launcher_batch(&cfg.project_name),
    )
    .map_err(|e| e.to_string())?;

    // Windows MSIX requires valid PNGs at Square150x150Logo and Square44x44Logo.
    write_png_or_fallback(cfg, &assets_dir.join("Logo.png"), "256x256", 150)?;
    write_png_or_fallback(cfg, &assets_dir.join("SmallLogo.png"), "48x48", 44)?;

    let artifact_path = base_dir.join(format!("{}_{version}_{arch}.msix", cfg.project_name));
    let Some(makeappx) = find_makeappx_binary() else {
        return Ok(InstallerBuildResult {
            format: InstallerFormat::Msix,
            status: InstallerBuildStatus::StagedOnly {
                staging_dir: win_dir.to_string_lossy().into_owned(),
                missing_tool: String::from("makeappx.exe"),
                command_hint: format!(
                    "makeappx pack /o /d {} /p {}",
                    win_dir.display(),
                    artifact_path.display()
                ),
            },
        });
    };

    let mut cmd = Command::new(&makeappx);
    cmd.args(["pack", "/o", "/d"])
        .arg(&win_dir)
        .arg("/p")
        .arg(&artifact_path);
    run_tool(&mut cmd, "makeappx")?;
    finish_built_artifact(InstallerFormat::Msix, &artifact_path)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let target = dst.join(entry.file_name());
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).map_err(|e| e.to_string())?;
            if file_type.is_file() && entry.path().parent().is_some_and(|p| p.ends_with("MacOS")) {
                mark_executable(&target)?;
            }
        }
    }
    Ok(())
}

fn stage_dmg_root_and_sign(
    project_name: &str,
    base_dir: &Path,
    app_bundle: &Path,
    entitlements_path: &Path,
) -> Result<PathBuf, String> {
    let dmg_root = base_dir.join("macos/dmg-root");
    if dmg_root.exists() {
        let _ = fs::remove_dir_all(&dmg_root);
    }
    let staged_app = dmg_root.join(format!("{project_name}.app"));
    copy_dir_recursive(app_bundle, &staged_app)?;
    #[cfg(unix)]
    {
        let apps_link = dmg_root.join("Applications");
        let _ = fs::remove_file(&apps_link);
        std::os::unix::fs::symlink("/Applications", &apps_link).map_err(|e| e.to_string())?;
    }

    if cfg!(target_os = "macos") {
        let identity = env::var("NG_DESKTOP_MACOS_IDENTITY").unwrap_or_else(|_| String::from("-"));
        let mut cmd = Command::new("codesign");
        cmd.args(["--force", "--deep", "--sign", &identity]);
        if identity != "-" {
            cmd.args(["--options", "runtime"]);
        }
        cmd.arg("--entitlements")
            .arg(entitlements_path)
            .arg(&staged_app);
        if let Ok(out) = cmd.output()
            && !out.status.success()
        {
            return Err(format!(
                "codesign failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
    }
    Ok(dmg_root)
}

fn build_dmg_installer(
    cfg: &AngularNativeConfig,
    version: &str,
    engine_bin: &Path,
    opts: InstallerBuildOptions<'_>,
) -> Result<InstallerBuildResult, String> {
    let base_dir = Path::new(&cfg.output_path).join("packages");
    let app_bundle = base_dir
        .join("macos")
        .join(format!("{}.app", cfg.project_name));
    let contents = app_bundle.join("Contents");
    let macos_bin_dir = contents.join("MacOS");
    let resources_dir = contents.join("Resources");
    for dir in [&macos_bin_dir, &resources_dir] {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }

    let dst_bin = macos_bin_dir.join(&cfg.project_name);
    fs::copy(engine_bin, &dst_bin).map_err(|e| e.to_string())?;
    mark_executable(&dst_bin)?;

    let launcher_path = macos_bin_dir.join("run-appkit.sh");
    fs::write(
        &launcher_path,
        format!(
            "#!/bin/sh\nDIR=\"$(cd \"$(dirname \"$0\")\" && pwd)\"\nexec \"${{DIR}}/{}\" --project=\"{}\" \"$@\"\n",
            cfg.project_name, cfg.project_name
        ),
    )
    .map_err(|e| e.to_string())?;
    mark_executable(&launcher_path)?;

    copy_bundle_files(
        Path::new(&cfg.output_path),
        &resources_dir,
        opts.require_bundle,
    )?;
    write_runtime_metadata(cfg, version, &resources_dir)?;
    write_png_or_fallback(
        cfg,
        &resources_dir.join(format!("{}.png", cfg.project_name)),
        "256x256",
        128,
    )?;

    fs::write(
        contents.join("Info.plist"),
        generate_macos_info_plist(&cfg.project_name, &cfg.project_name, version),
    )
    .map_err(|e| e.to_string())?;
    let entitlements_path = contents.join("Entitlements.plist");
    fs::write(
        &entitlements_path,
        generate_macos_entitlements_plist(&cfg.project_name, &cfg.permissions),
    )
    .map_err(|e| e.to_string())?;

    let arch = macos_arch_for_host();
    let artifact_path = base_dir.join(format!("{}-{version}-{arch}.dmg", cfg.project_name));
    if !cfg!(target_os = "macos") && !command_exists("hdiutil") {
        return Ok(InstallerBuildResult {
            format: InstallerFormat::Dmg,
            status: InstallerBuildStatus::StagedOnly {
                staging_dir: app_bundle.to_string_lossy().into_owned(),
                missing_tool: String::from("hdiutil"),
                command_hint: format!(
                    "hdiutil create -volname \"{} {version}\" -srcfolder {} -ov -format UDZO {}",
                    cfg.project_name,
                    app_bundle.display(),
                    artifact_path.display()
                ),
            },
        });
    }

    let dmg_root = stage_dmg_root_and_sign(
        &cfg.project_name,
        &base_dir,
        &app_bundle,
        &entitlements_path,
    )?;
    let mut cmd = Command::new("hdiutil");
    cmd.args(["create", "-volname"])
        .arg(format!("{} {version}", cfg.project_name))
        .arg("-srcfolder")
        .arg(&dmg_root)
        .args(["-ov", "-format", "UDZO"])
        .arg(&artifact_path);
    let hdiutil_res = run_tool(&mut cmd, "hdiutil create");
    let _ = fs::remove_dir_all(&dmg_root);
    hdiutil_res?;
    finish_built_artifact(InstallerFormat::Dmg, &artifact_path)
}

/// Builds a native desktop installer with explicit packaging options (`require_bundle`, `binary_override`).
pub fn build_installer_with_options(
    cfg: &AngularNativeConfig,
    version: &str,
    format: InstallerFormat,
    opts: InstallerBuildOptions<'_>,
) -> Result<InstallerBuildResult, String> {
    validate_project_name(&cfg.project_name)?;
    let engine_bin = resolve_engine_binary(opts.binary_override)?;
    match format {
        InstallerFormat::Deb => build_deb_installer(cfg, version, &engine_bin, opts),
        InstallerFormat::AppImage => build_appimage_installer(cfg, version, &engine_bin, opts),
        InstallerFormat::Flatpak => build_flatpak_installer(cfg, version, &engine_bin, opts),
        InstallerFormat::Msix => build_msix_installer(cfg, version, &engine_bin, opts),
        InstallerFormat::Dmg => build_dmg_installer(cfg, version, &engine_bin, opts),
    }
}

/// Builds a native desktop installer in `<output_path>/packages/`, compiling the final `.deb`,
/// `.AppImage`, `.flatpak`, `.msix`, or `.dmg` when the host OS packaging toolchain is present,
/// or returning `StagedOnly` with exact invocation instructions when a tool is unavailable.
pub fn build_installer(
    cfg: &AngularNativeConfig,
    version: &str,
    format: InstallerFormat,
    binary_override: Option<&Path>,
) -> Result<InstallerBuildResult, String> {
    build_installer_with_options(
        cfg,
        version,
        format,
        InstallerBuildOptions {
            binary_override,
            require_bundle: false,
        },
    )
}

/// Builds every installer format natively targeting the current host operating system with options.
pub fn build_installers_for_host_with_options(
    cfg: &AngularNativeConfig,
    version: &str,
    opts: InstallerBuildOptions<'_>,
) -> Result<Vec<InstallerBuildResult>, String> {
    InstallerFormat::host_defaults()
        .iter()
        .map(|&fmt| build_installer_with_options(cfg, version, fmt, opts))
        .collect()
}

/// Builds every installer format natively targeting the current host operating system.
pub fn build_installers_for_host(
    cfg: &AngularNativeConfig,
    version: &str,
    binary_override: Option<&Path>,
) -> Result<Vec<InstallerBuildResult>, String> {
    build_installers_for_host_with_options(
        cfg,
        version,
        InstallerBuildOptions {
            binary_override,
            require_bundle: false,
        },
    )
}

/// Stages and (where host tools exist) builds all 5 installer targets with options.
pub fn build_all_installers_with_options(
    cfg: &AngularNativeConfig,
    version: &str,
    opts: InstallerBuildOptions<'_>,
) -> Result<Vec<InstallerBuildResult>, String> {
    InstallerFormat::all()
        .iter()
        .map(|&fmt| build_installer_with_options(cfg, version, fmt, opts))
        .collect()
}

/// Stages and (where host tools exist) builds all 5 installer targets.
pub fn build_all_installers(
    cfg: &AngularNativeConfig,
    version: &str,
    binary_override: Option<&Path>,
) -> Result<Vec<InstallerBuildResult>, String> {
    build_all_installers_with_options(
        cfg,
        version,
        InstallerBuildOptions {
            binary_override,
            require_bundle: false,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(name: &str, with_bundle: bool) -> AngularNativeConfig {
        let mut cfg = AngularNativeConfig::default_standalone(name);
        let root = env::temp_dir().join(format!("ng-desktop-installer-test-{name}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        if with_bundle {
            fs::write(root.join("main.ngb"), "win Win\n").unwrap();
            fs::write(root.join("app.bundle.js"), "console.log('ng-desktop');\n").unwrap();
        }
        cfg.output_path = root.to_string_lossy().into_owned();
        cfg.icon_search_dir = root.join("icons").to_string_lossy().into_owned();
        cfg
    }

    #[test]
    fn validates_project_names_and_sanitizes_debian_packages() {
        assert!(validate_project_name("ng-linux-counter").is_ok());
        assert!(validate_project_name("").is_err());
        assert!(validate_project_name("../escape").is_err());
        assert!(validate_project_name("foo/bar").is_err());
        assert_eq!(sanitize_debian_package_name("My_App_01"), "my-app-01");
    }

    #[test]
    fn require_bundle_rejects_missing_compiled_bundle() {
        let cfg = test_config("missing-bundle-demo", false);
        let err = build_installer_with_options(
            &cfg,
            "0.6.0",
            InstallerFormat::Deb,
            InstallerBuildOptions {
                binary_override: None,
                require_bundle: true,
            },
        )
        .unwrap_err();
        assert!(err.contains("Compiled application bundle missing"));
        let _ = fs::remove_dir_all(&cfg.output_path);
    }

    #[test]
    fn sorts_windows_kit_versions_numerically_beyond_lex_order() {
        let ver_low = parse_kit_semver_key("10.0.9999.0");
        let ver_high = parse_kit_semver_key("10.0.26100.0");
        assert!(ver_high > ver_low);
    }

    #[test]
    fn builds_real_deb_with_installed_size_and_populates_all_installer_payloads() {
        let cfg = test_config("slice4-demo", true);
        let results = build_all_installers(&cfg, "0.6.0", None).unwrap();
        assert_eq!(results.len(), 5);

        let deb = results
            .iter()
            .find(|r| r.format == InstallerFormat::Deb)
            .unwrap();
        if command_exists("dpkg-deb") {
            let InstallerBuildStatus::Built {
                artifact_path,
                size_bytes,
            } = &deb.status
            else {
                panic!("Expected real .deb when dpkg-deb is installed");
            };
            assert!(*size_bytes > 0);
            let info = Command::new("dpkg-deb")
                .args(["--info", artifact_path])
                .output()
                .unwrap();
            assert!(info.status.success());
            let control_out = String::from_utf8_lossy(&info.stdout);
            assert!(control_out.contains("Installed-Size:"));

            let contents = Command::new("dpkg-deb")
                .args(["--contents", artifact_path])
                .output()
                .unwrap();
            assert!(contents.status.success());
            let listing = String::from_utf8_lossy(&contents.stdout);
            assert!(listing.contains("./usr/bin/slice4-demo"));
            assert!(listing.contains("./usr/share/slice4-demo/main.ngb"));
            assert!(listing.contains("./usr/share/slice4-demo/app.bundle.js"));
        }

        let pkg_dir = Path::new(&cfg.output_path).join("packages");
        assert!(pkg_dir.join("windows/AppxManifest.xml").is_file());
        assert!(pkg_dir.join("windows/Assets/Logo.png").is_file());
        assert!(pkg_dir.join("windows/Assets/SmallLogo.png").is_file());
        assert!(
            pkg_dir
                .join("macos/slice4-demo.app/Contents/Info.plist")
                .is_file()
        );
        assert!(
            pkg_dir
                .join("macos/slice4-demo.app/Contents/MacOS/slice4-demo")
                .is_file()
        );
        assert!(pkg_dir.join("appimage/slice4-demo.AppDir/AppRun").is_file());
        assert!(pkg_dir.join("flatpak/slice4-demo.yml").is_file());

        let _ = fs::remove_dir_all(&cfg.output_path);
    }
}

//! Workspace configuration resolution and least-privilege capability manifests.
//!
//! Parses `angular.json`, Nx `project.json`, runtime manifests (`ng-desktop.runtime.json`),
//! CLI flags, and environment overrides into [`AngularNativeConfig`] and [`AppPermissions`].

pub mod permissions;

pub use permissions::AppPermissions;

use crate::desktop::json::{JsonValue, parse_json};
use crate::engine::EngineError;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Resolved workspace/runtime configuration for an Angular Desktop Engine application.
///
/// Supports Angular CLI (`angular.json`), Nx (`project.json`), builder runtime manifests
/// (`ng-desktop.runtime.json`), CLI flags, environment overrides, and zero-config standalone runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AngularNativeConfig {
    pub project_name: String,
    pub version: String,
    pub os: String,
    pub arch: String,
    pub cpu: String,
    pub target_platform: String,
    pub js_engine: String,
    pub output_path: String,
    pub icon_search_dir: String,
    pub gpu_backend: String,
    pub permissions: AppPermissions,
}

fn default_host_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

fn resolve_target_platform(os: &str) -> String {
    match os.trim().to_ascii_lowercase().as_str() {
        "windows" | "win32" => String::from("windows-winui"),
        "macos" | "darwin" => String::from("macos-appkit"),
        _ => String::from("linux-gtk4"),
    }
}

fn resolve_default_js_engine(os: &str) -> String {
    match os.trim().to_ascii_lowercase().as_str() {
        "windows" | "win32" => String::from("quickjs"),
        _ => String::from("jsc"),
    }
}

fn os_from_target_platform(target_platform: &str) -> String {
    if target_platform.starts_with("windows") {
        String::from("windows")
    } else if target_platform.starts_with("macos") {
        String::from("macos")
    } else {
        String::from("linux")
    }
}

#[must_use]
pub fn extract_cli_flag(args: &[String], flag: &str) -> Option<String> {
    let prefix = format!("{flag}=");
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Some(val) = arg.strip_prefix(&prefix) {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return Some(String::from(trimmed));
            }
        } else if arg == flag
            && let Some(next_val) = iter.next()
        {
            let trimmed = next_val.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("--") {
                return Some(String::from(trimmed));
            }
        }
    }
    None
}

fn read_non_empty_env(name: &str) -> Option<String> {
    env::var(name).ok().and_then(|val| {
        let trimmed = val.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(String::from(trimmed))
        }
    })
}

fn push_unique_path(candidates: &mut Vec<PathBuf>, path: PathBuf) {
    if !candidates.contains(&path) {
        candidates.push(path);
    }
}

impl AngularNativeConfig {
    /// Creates a standalone configuration for self-contained `.deb`, `.exe`, and `.dmg` builds
    /// launched outside a development workspace.
    #[must_use]
    pub fn default_standalone(project_name: &str) -> Self {
        let os = String::from(default_host_os());
        let target_platform = resolve_target_platform(&os);
        let js_engine = resolve_default_js_engine(&os);
        let output_path = format!("dist/{project_name}");
        let icon_search_dir = format!("{output_path}/icons");
        Self {
            project_name: String::from(project_name),
            version: String::from(env!("CARGO_PKG_VERSION")),
            os,
            arch: String::from(env::consts::ARCH),
            cpu: String::from("native"),
            target_platform,
            js_engine,
            output_path,
            icon_search_dir,
            gpu_backend: String::from("auto"),
            permissions: AppPermissions::minimal(),
        }
    }

    /// Reads and resolves native build settings from an `angular.json`, Nx `project.json`, or
    /// `ng-desktop.runtime.json` file, merging an adjacent `permissions.manifest.json` if present.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, EngineError> {
        Self::from_file_for_project(path, None)
    }

    /// Reads and resolves native build settings for a specific workspace project when `project_hint` is provided.
    pub fn from_file_for_project(
        path: impl AsRef<Path>,
        project_hint: Option<&str>,
    ) -> Result<Self, EngineError> {
        let path_ref = path.as_ref();
        let content =
            fs::read_to_string(path_ref).map_err(|e| EngineError::ConfigError(e.to_string()))?;
        let mut cfg = Self::from_json_str_for_project(&content, project_hint)?;
        cfg.merge_adjacent_manifests(path_ref.parent());
        Ok(cfg)
    }

    /// Resolves configuration dynamically from CLI flags (`--config`, `--project`, `--version`,
    /// `--output-path`, `--target-platform`), environment variables (`NG_DESKTOP_*`), local or
    /// installed config manifests (`ng-desktop.runtime.json`, `project.json`, `angular.json`),
    /// and falls back cleanly to a standalone configuration.
    #[must_use]
    pub fn from_cli_and_env(args: &[String]) -> Self {
        let explicit_config =
            extract_cli_flag(args, "--config").or_else(|| read_non_empty_env("NG_DESKTOP_CONFIG"));
        let project_override = extract_cli_flag(args, "--project")
            .or_else(|| read_non_empty_env("NG_DESKTOP_PROJECT"));
        let version_override = extract_cli_flag(args, "--version")
            .or_else(|| read_non_empty_env("NG_DESKTOP_APP_VERSION"));
        let output_override = extract_cli_flag(args, "--output-path")
            .or_else(|| extract_cli_flag(args, "--output"))
            .or_else(|| read_non_empty_env("NG_DESKTOP_OUTPUT_PATH"));
        let target_override = extract_cli_flag(args, "--target-platform")
            .or_else(|| extract_cli_flag(args, "--target"))
            .or_else(|| read_non_empty_env("NG_DESKTOP_TARGET_PLATFORM"));
        let gpu_override = extract_cli_flag(args, "--gpu")
            .or_else(|| read_non_empty_env("NG_DESKTOP_GPU_BACKEND"));

        let mut cfg = explicit_config
            .map_or_else(
                || Self::discover_from_environment(project_override.as_deref()),
                |config_path| {
                    Self::from_file_for_project(config_path, project_override.as_deref()).ok()
                },
            )
            .unwrap_or_else(|| Self::default_standalone("ng-desktop-app"));

        cfg.apply_overrides(
            project_override.as_deref(),
            version_override.as_deref(),
            output_override.as_deref(),
            target_override.as_deref(),
            gpu_override.as_deref(),
        );
        if output_override.is_some() && cfg.permissions == AppPermissions::minimal() {
            cfg.merge_adjacent_manifests(None);
        }
        cfg
    }

    /// Parses Angular CLI (`angular.json`), Nx (`project.json`), or runtime (`ng-desktop.runtime.json`)
    /// JSON payloads into an `AngularNativeConfig`.
    pub fn from_json_str(content: &str) -> Result<Self, EngineError> {
        Self::from_json_str_for_project(content, None)
    }

    /// Parses JSON configuration payloads scoping option lookups strictly to `project_hint` when present.
    pub fn from_json_str_for_project(
        content: &str,
        project_hint: Option<&str>,
    ) -> Result<Self, EngineError> {
        let root = parse_json(content).map_err(EngineError::ConfigError)?;
        let (project_name, scopes) = Self::resolve_project_and_scopes(&root, project_hint)?;

        let raw_target = Self::lookup_string(&scopes, "targetPlatform");
        let raw_os = Self::lookup_string(&scopes, "os");
        let os = match raw_os.as_deref() {
            Some("auto") | None => raw_target
                .as_deref()
                .map_or_else(|| String::from(default_host_os()), os_from_target_platform),
            Some(explicit) => explicit.to_ascii_lowercase(),
        };

        let target_platform = raw_target.unwrap_or_else(|| resolve_target_platform(&os));
        let js_engine = Self::lookup_string(&scopes, "jsEngine")
            .unwrap_or_else(|| resolve_default_js_engine(&os));
        let output_path =
            Self::lookup_output_path(&scopes).unwrap_or_else(|| format!("dist/{project_name}"));
        let icon_search_dir = Self::lookup_string(&scopes, "iconSearchDir")
            .unwrap_or_else(|| format!("{output_path}/icons"));
        let version = Self::lookup_string(&scopes, "version")
            .or_else(|| Self::lookup_string(&scopes, "appVersion"))
            .unwrap_or_else(|| String::from(env!("CARGO_PKG_VERSION")));
        let arch = match Self::lookup_string(&scopes, "arch").as_deref() {
            Some("auto") | None => String::from(env::consts::ARCH),
            Some(explicit) => String::from(explicit),
        };
        let cpu = Self::lookup_string(&scopes, "cpu").unwrap_or_else(|| String::from("auto"));
        let gpu_backend = Self::lookup_string(&scopes, "gpu")
            .or_else(|| Self::lookup_string(&scopes, "gpuBackend"))
            .map_or_else(
                || String::from("auto"),
                |raw| raw.trim().to_ascii_lowercase(),
            );
        let permissions = Self::lookup_permissions(&scopes);

        Ok(Self {
            project_name,
            version,
            os,
            arch,
            cpu,
            target_platform,
            js_engine,
            output_path,
            icon_search_dir,
            gpu_backend,
            permissions,
        })
    }

    fn resolve_project_and_scopes<'a>(
        root: &'a JsonValue,
        project_hint: Option<&str>,
    ) -> Result<(String, Vec<&'a JsonValue>), EngineError> {
        if let Some(runtime_name) = root.get_str("projectName") {
            return Ok((String::from(runtime_name), vec![root]));
        }

        if let Some(projects_obj) = root.get_object("projects")
            && let Some(entries) = projects_obj.as_object()
        {
            let chosen = project_hint
                .and_then(|hint| entries.iter().find(|(name, _)| name == hint))
                .or_else(|| {
                    root.get_str("defaultProject")
                        .and_then(|def| entries.iter().find(|(name, _)| name == def))
                })
                .or_else(|| {
                    entries.iter().find(|(_, proj)| {
                        proj.get_object("architect")
                            .and_then(|a| a.get_object("native"))
                            .is_some()
                            || proj
                                .get_object("targets")
                                .and_then(|t| t.get_object("native"))
                                .is_some()
                    })
                })
                .or_else(|| entries.first());

            if let Some((proj_name, proj_val)) = chosen {
                let mut scopes = Vec::new();
                for container in ["architect", "targets"] {
                    if let Some(targets_obj) = proj_val.get_object(container) {
                        for target in ["native", "build"] {
                            if let Some(opts) = targets_obj
                                .get_object(target)
                                .and_then(|t| t.get_object("options"))
                            {
                                scopes.push(opts);
                            }
                        }
                    }
                }
                scopes.push(proj_val);
                return Ok((proj_name.clone(), scopes));
            }
        }

        if let Some(nx_name) = root.get_str("name") {
            let mut scopes = Vec::new();
            if let Some(targets_obj) = root.get_object("targets") {
                for target in ["native", "build"] {
                    if let Some(opts) = targets_obj
                        .get_object(target)
                        .and_then(|t| t.get_object("options"))
                    {
                        scopes.push(opts);
                    }
                }
            }
            scopes.push(root);
            return Ok((String::from(nx_name), scopes));
        }

        Err(EngineError::ConfigError(String::from(
            "Missing project identifier in config ('projectName', 'defaultProject', 'projects', or Nx 'name')",
        )))
    }

    fn lookup_string(scopes: &[&JsonValue], key: &str) -> Option<String> {
        scopes
            .iter()
            .find_map(|scope| scope.get_str(key).map(String::from))
    }

    fn lookup_output_path(scopes: &[&JsonValue]) -> Option<String> {
        scopes.iter().find_map(|scope| {
            let val = scope.get("outputPath")?;
            val.as_str()
                .map(String::from)
                .or_else(|| val.get_str("base").map(String::from))
        })
    }

    fn lookup_permissions(scopes: &[&JsonValue]) -> AppPermissions {
        for scope in scopes {
            if scope.get_object("permissions").is_some()
                || scope.get_object("capabilities").is_some()
            {
                return AppPermissions::from_json_value(scope);
            }
        }
        AppPermissions::minimal()
    }

    fn discover_from_environment(project_hint: Option<&str>) -> Option<Self> {
        let mut candidates = Vec::new();
        for name in ["ng-desktop.runtime.json", "project.json", "angular.json"] {
            push_unique_path(&mut candidates, PathBuf::from(name));
        }

        if let Some(project) = project_hint {
            push_unique_path(
                &mut candidates,
                PathBuf::from(format!("dist/{project}/ng-desktop.runtime.json")),
            );
            push_unique_path(
                &mut candidates,
                PathBuf::from(format!("/usr/share/{project}/ng-desktop.runtime.json")),
            );
        }

        if let Ok(exe_path) = env::current_exe()
            && let Some(exe_dir) = exe_path.parent()
        {
            push_unique_path(&mut candidates, exe_dir.join("ng-desktop.runtime.json"));
            push_unique_path(
                &mut candidates,
                exe_dir.join("../Resources/ng-desktop.runtime.json"),
            );
            let effective_project = project_hint.map(String::from).or_else(|| {
                exe_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .filter(|s| !s.starts_with("ng_native_engine"))
                    .map(String::from)
            });
            if let Some(project) = effective_project {
                push_unique_path(
                    &mut candidates,
                    exe_dir.join(format!("../share/{project}/ng-desktop.runtime.json")),
                );
                push_unique_path(
                    &mut candidates,
                    PathBuf::from(format!("/usr/share/{project}/ng-desktop.runtime.json")),
                );
            }
        }

        for candidate in candidates {
            if let Ok(cfg) = Self::from_file_for_project(&candidate, project_hint) {
                return Some(cfg);
            }
        }
        None
    }

    fn merge_adjacent_manifests(&mut self, config_dir: Option<&Path>) {
        if self.permissions != AppPermissions::minimal() {
            return;
        }
        let mut manifest_candidates = vec![
            PathBuf::from(&self.output_path).join("permissions.manifest.json"),
            PathBuf::from(&self.output_path).join("ng-desktop.runtime.json"),
        ];
        if let Some(dir) = config_dir {
            push_unique_path(
                &mut manifest_candidates,
                dir.join("permissions.manifest.json"),
            );
        }
        for path in manifest_candidates {
            if let Ok(raw) = fs::read_to_string(path) {
                let parsed = AppPermissions::from_json_str(&raw);
                if parsed != AppPermissions::minimal() {
                    self.permissions = parsed;
                    break;
                }
            }
        }
    }

    fn apply_overrides(
        &mut self,
        project_override: Option<&str>,
        version_override: Option<&str>,
        output_override: Option<&str>,
        target_override: Option<&str>,
        gpu_override: Option<&str>,
    ) {
        if let Some(project) = project_override {
            let old_default_out = format!("dist/{}", self.project_name);
            let old_default_icons = format!("{}/icons", self.output_path);
            self.project_name = String::from(project);
            if output_override.is_none() && self.output_path == old_default_out {
                self.output_path = format!("dist/{project}");
            }
            if self.icon_search_dir == old_default_icons {
                self.icon_search_dir = format!("{}/icons", self.output_path);
            }
        }
        if let Some(version) = version_override {
            self.version = String::from(version);
        }
        if let Some(output) = output_override {
            let old_default_icons = format!("{}/icons", self.output_path);
            self.output_path = String::from(output);
            if self.icon_search_dir == old_default_icons {
                self.icon_search_dir = format!("{output}/icons");
            }
        }
        if let Some(target) = target_override {
            self.target_platform = String::from(target);
            self.os = os_from_target_platform(target);
            self.js_engine = resolve_default_js_engine(&self.os);
        }
        if let Some(gpu) = gpu_override {
            self.gpu_backend = gpu.trim().to_ascii_lowercase();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_angular_json_configuration_from_file() {
        let tmp_dir =
            env::temp_dir().join(format!("ng-desktop-config-test-{}", std::process::id()));
        fs::create_dir_all(&tmp_dir).unwrap();
        let cfg_path = tmp_dir.join("angular.json");
        fs::write(
            &cfg_path,
            r#"{
                "projects": {
                    "sample-desktop-app": {
                        "architect": {
                            "native": {
                                "options": {
                                    "os": "linux",
                                    "arch": "x86_64"
                                }
                            }
                        }
                    }
                }
            }"#,
        )
        .unwrap();

        let cfg = AngularNativeConfig::from_file(&cfg_path).unwrap();
        let _ = fs::remove_dir_all(&tmp_dir);

        assert_eq!(cfg.project_name, "sample-desktop-app");
        assert_eq!(cfg.os, "linux");
        assert_eq!(cfg.arch, "x86_64");
        assert_eq!(cfg.target_platform, "linux-gtk4");
        assert_eq!(cfg.js_engine, "jsc");
        assert_eq!(cfg.output_path, "dist/sample-desktop-app");
        assert_eq!(cfg.icon_search_dir, "dist/sample-desktop-app/icons");
        assert!(!cfg.permissions.full_system_access);
    }

    #[test]
    fn derives_platform_engine_and_output_from_minimal_os_arch_config() {
        let minimal_win = r#"{
            "projects": {
                "my-desktop-app": {
                    "architect": {
                        "native": {
                            "options": {
                                "os": "windows",
                                "arch": "aarch64",
                                "cpu": "neon"
                            }
                        }
                    }
                }
            }
        }"#;
        let cfg = AngularNativeConfig::from_json_str(minimal_win).unwrap();
        assert_eq!(cfg.project_name, "my-desktop-app");
        assert_eq!(cfg.os, "windows");
        assert_eq!(cfg.arch, "aarch64");
        assert_eq!(cfg.cpu, "neon");
        assert_eq!(cfg.target_platform, "windows-winui");
        assert_eq!(cfg.js_engine, "quickjs");
        assert_eq!(cfg.output_path, "dist/my-desktop-app");
        assert_eq!(cfg.icon_search_dir, "dist/my-desktop-app/icons");
    }

    #[test]
    fn parses_nx_project_json_with_permissions_and_version() {
        let nx_project = r#"{
            "name": "nx-invoice-app",
            "projectType": "application",
            "sourceRoot": "apps/nx-invoice-app/src",
            "targets": {
                "native": {
                    "executor": "@ng-desktop/builders:native",
                    "options": {
                        "version": "1.4.2",
                        "targetPlatform": "macos-appkit",
                        "outputPath": "dist/apps/nx-invoice-app",
                        "permissions": {
                            "allow": ["network", "filesystem:dialogs"],
                            "deny": ["clipboard"]
                        }
                    }
                }
            }
        }"#;
        let cfg = AngularNativeConfig::from_json_str(nx_project).unwrap();
        assert_eq!(cfg.project_name, "nx-invoice-app");
        assert_eq!(cfg.version, "1.4.2");
        assert_eq!(cfg.os, "macos");
        assert_eq!(cfg.target_platform, "macos-appkit");
        assert_eq!(cfg.js_engine, "jsc");
        assert_eq!(cfg.output_path, "dist/apps/nx-invoice-app");
        assert_eq!(cfg.icon_search_dir, "dist/apps/nx-invoice-app/icons");
        assert!(cfg.permissions.network);
        assert!(cfg.permissions.filesystem_dialogs);
        assert!(!cfg.permissions.clipboard);
        assert!(!cfg.permissions.full_system_access);
    }

    #[test]
    fn parses_builder_runtime_manifest_json() {
        let runtime_json = r#"{
            "schemaVersion": 1,
            "projectName": "acme-studio",
            "version": "0.9.0",
            "targetPlatform": "windows-winui",
            "outputPath": "dist/acme-studio",
            "iconSearchDir": "dist/acme-studio/assets/icons",
            "permissions": {
                "capabilities": {
                    "network": false,
                    "sqlite": true,
                    "clipboard": true
                }
            }
        }"#;
        let cfg = AngularNativeConfig::from_json_str(runtime_json).unwrap();
        assert_eq!(cfg.project_name, "acme-studio");
        assert_eq!(cfg.version, "0.9.0");
        assert_eq!(cfg.os, "windows");
        assert_eq!(cfg.js_engine, "quickjs");
        assert_eq!(cfg.icon_search_dir, "dist/acme-studio/assets/icons");
        assert!(cfg.permissions.sqlite);
        assert!(cfg.permissions.clipboard);
        assert!(!cfg.permissions.network);
    }

    #[test]
    fn applies_cli_flag_overrides_on_top_of_resolved_config() {
        let args = vec![
            String::from("ng-desktop"),
            String::from("--project=custom-client"),
            String::from("--version=3.1.0"),
            String::from("--output-path=build/custom-client"),
            String::from("--target-platform=macos-appkit"),
        ];
        let cfg = AngularNativeConfig::from_cli_and_env(&args);
        assert_eq!(cfg.project_name, "custom-client");
        assert_eq!(cfg.version, "3.1.0");
        assert_eq!(cfg.output_path, "build/custom-client");
        assert_eq!(cfg.icon_search_dir, "build/custom-client/icons");
        assert_eq!(cfg.target_platform, "macos-appkit");
        assert_eq!(cfg.os, "macos");
    }

    #[test]
    fn isolates_selected_project_scope_from_sibling_projects_and_object_output_path() {
        let multi_project_json = r#"{
            "projects": {
                "ng-linux-counter": {
                    "architect": {
                        "build": {
                            "options": {
                                "outputPath": { "base": "dist/custom-counter" }
                            }
                        },
                        "native": {
                            "options": {
                                "os": "linux",
                                "permissions": {
                                    "capabilities": { "sqlite": true }
                                }
                            }
                        }
                    }
                },
                "ng-desktop-docs": {
                    "architect": {
                        "build": {
                            "options": {
                                "outputPath": "dist/ng-desktop-docs"
                            }
                        },
                        "native": {
                            "options": {
                                "os": "windows",
                                "permissions": {
                                    "capabilities": { "network": true, "fullSystemAccess": true }
                                }
                            }
                        }
                    }
                }
            }
        }"#;

        let primary = AngularNativeConfig::from_json_str(multi_project_json).unwrap();
        assert_eq!(primary.project_name, "ng-linux-counter");
        assert_eq!(primary.output_path, "dist/custom-counter");
        assert_eq!(primary.os, "linux");
        assert!(primary.permissions.sqlite);
        assert!(!primary.permissions.network);
        assert!(!primary.permissions.full_system_access);

        let docs = AngularNativeConfig::from_json_str_for_project(
            multi_project_json,
            Some("ng-desktop-docs"),
        )
        .unwrap();
        assert_eq!(docs.project_name, "ng-desktop-docs");
        assert_eq!(docs.output_path, "dist/ng-desktop-docs");
        assert_eq!(docs.os, "windows");
        assert!(docs.permissions.network);
        assert!(docs.permissions.full_system_access);
        assert!(!docs.permissions.sqlite);
    }

    #[test]
    fn parses_gpu_backend_from_config_and_cli_override() {
        let angular_json = r#"{
            "projects": {
                "gpu-app": {
                    "architect": {
                        "native": {
                            "options": {
                                "os": "linux",
                                "gpu": "Vulkan"
                            }
                        }
                    }
                }
            }
        }"#;
        let cfg = AngularNativeConfig::from_json_str(angular_json).unwrap();
        assert_eq!(cfg.gpu_backend, "vulkan");

        let runtime_json = r#"{
            "projectName": "gpu-app",
            "gpuBackend": "Metal"
        }"#;
        let runtime_cfg = AngularNativeConfig::from_json_str(runtime_json).unwrap();
        assert_eq!(runtime_cfg.gpu_backend, "metal");

        let cli_cfg = AngularNativeConfig::from_cli_and_env(&[
            String::from("ng-native-engine"),
            String::from("--gpu"),
            String::from("Software"),
        ]);
        assert_eq!(cli_cfg.gpu_backend, "software");
    }
}

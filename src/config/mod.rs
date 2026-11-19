use crate::engine::EngineError;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AngularNativeConfig {
    pub project_name: String,
    pub os: String,
    pub arch: String,
    pub cpu: String,
    pub target_platform: String,
    pub js_engine: String,
    pub output_path: String,
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

impl AngularNativeConfig {
    /// Creates a standalone configuration for self-contained `.deb`, `.exe`, and `.dmg` builds
    /// launched outside the development workspace.
    pub fn default_standalone(project_name: &str) -> Self {
        let os = String::from(default_host_os());
        let target_platform = resolve_target_platform(&os);
        let js_engine = resolve_default_js_engine(&os);
        Self {
            project_name: String::from(project_name),
            os,
            arch: String::from(std::env::consts::ARCH),
            cpu: String::from("native"),
            target_platform,
            js_engine,
            output_path: format!("dist/{project_name}"),
        }
    }

    /// Reads and resolves native build settings from an `angular.json` file, supporting
    /// minimal `os` / `arch` / `cpu` options alongside explicit `targetPlatform` / `outputPath`.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, EngineError> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| EngineError::ConfigError(e.to_string()))?;
        Self::from_json_str(&content)
    }

    pub fn from_json_str(content: &str) -> Result<Self, EngineError> {
        let project_name = Self::extract_json_string(content, "defaultProject")
            .or_else(|| Self::extract_first_project_name(content))
            .ok_or_else(|| {
                EngineError::ConfigError(String::from("Missing project in 'angular.json'"))
            })?;

        let raw_target = Self::extract_json_string(content, "targetPlatform");
        let raw_os = Self::extract_json_string(content, "os");
        let os = match raw_os.as_deref() {
            Some("auto") | None => raw_target.as_deref().map_or_else(
                || String::from(default_host_os()),
                |tp| {
                    if tp.starts_with("windows") {
                        String::from("windows")
                    } else if tp.starts_with("macos") {
                        String::from("macos")
                    } else {
                        String::from("linux")
                    }
                },
            ),
            Some(explicit) => explicit.to_ascii_lowercase(),
        };

        let target_platform = raw_target.unwrap_or_else(|| resolve_target_platform(&os));
        let js_engine = Self::extract_json_string(content, "jsEngine")
            .unwrap_or_else(|| resolve_default_js_engine(&os));
        let output_path = Self::extract_json_string(content, "outputPath")
            .unwrap_or_else(|| format!("dist/{project_name}"));
        let arch = match Self::extract_json_string(content, "arch").as_deref() {
            Some("auto") | None => String::from(std::env::consts::ARCH),
            Some(explicit) => String::from(explicit),
        };
        let cpu = Self::extract_json_string(content, "cpu").unwrap_or_else(|| String::from("auto"));

        Ok(Self {
            project_name,
            os,
            arch,
            cpu,
            target_platform,
            js_engine,
            output_path,
        })
    }

    fn extract_first_project_name(json: &str) -> Option<String> {
        let projects_key = "\"projects\"";
        let start = json.find(projects_key)? + projects_key.len();
        let rest = &json[start..];
        let brace = rest.find('{')? + 1;
        let inside = rest[brace..].trim_start();
        if !inside.starts_with('"') {
            return None;
        }
        let name_part = &inside[1..];
        let end_quote = name_part.find('"')?;
        Some(name_part[..end_quote].to_string())
    }

    fn extract_json_string(json: &str, key: &str) -> Option<String> {
        let pattern = format!("\"{key}\"");
        let start = json.find(&pattern)? + pattern.len();
        let rest = &json[start..];
        let colon = rest.find(':')? + 1;
        let after_colon = rest[colon..].trim_start();
        if !after_colon.starts_with('"') {
            return None;
        }
        let value_part = &after_colon[1..];
        let end_quote = value_part.find('"')?;
        Some(value_part[..end_quote].to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_workspace_angular_json_configuration() {
        let cfg = AngularNativeConfig::from_file("angular.json").unwrap();
        assert_eq!(cfg.project_name, "ng-linux-counter");
        assert_eq!(cfg.os, "linux");
        assert_eq!(cfg.arch, "x86_64");
        assert_eq!(cfg.target_platform, "linux-gtk4");
        assert_eq!(cfg.js_engine, "jsc");
        assert_eq!(cfg.output_path, "dist/ng-linux-counter");
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
    }
}

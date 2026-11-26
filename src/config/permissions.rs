use crate::desktop::json::{JsonValue, parse_json};

/// Granular OS capability manifest for an Angular Desktop Engine application.
///
/// Generated statically at build time from TypeScript AST imports (`permissions.manifest.json`)
/// or declared explicitly in workspace configuration (`angular.json` / `project.json`), ensuring
/// OS installers only request the minimum permissions required instead of full host access.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct AppPermissions {
    pub network: bool,
    pub sqlite: bool,
    pub filesystem_xdg: bool,
    pub filesystem_dialogs: bool,
    pub filesystem_drop: bool,
    pub clipboard: bool,
    pub notifications: bool,
    pub compute_threads: bool,
    /// Explicit escape hatch ONLY when an application legitimately requires unrestricted host trust.
    /// Defaults to `false` so installers never warn about requesting all system resources.
    pub full_system_access: bool,
}

impl Default for AppPermissions {
    fn default() -> Self {
        Self::minimal()
    }
}

impl AppPermissions {
    /// Returns a zero-privilege permission set where every sensitive OS capability is disabled.
    #[must_use]
    pub const fn minimal() -> Self {
        Self {
            network: false,
            sqlite: false,
            filesystem_xdg: false,
            filesystem_dialogs: false,
            filesystem_drop: false,
            clipboard: false,
            notifications: false,
            compute_threads: false,
            full_system_access: false,
        }
    }

    /// Returns all sandboxed desktop capabilities enabled while keeping `full_system_access` off.
    #[must_use]
    pub const fn allow_all_sandboxed() -> Self {
        Self {
            network: true,
            sqlite: true,
            filesystem_xdg: true,
            filesystem_dialogs: true,
            filesystem_drop: true,
            clipboard: true,
            notifications: true,
            compute_threads: true,
            full_system_access: false,
        }
    }

    /// Returns `true` when any filesystem capability (`xdg`, `dialogs`, or `drop`) is granted.
    #[must_use]
    pub const fn has_any_filesystem(&self) -> bool {
        self.filesystem_xdg || self.filesystem_dialogs || self.filesystem_drop
    }

    /// Returns `true` when zero permissions are active.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        !self.network
            && !self.sqlite
            && !self.has_any_filesystem()
            && !self.clipboard
            && !self.notifications
            && !self.compute_threads
            && !self.full_system_access
    }

    /// Returns deterministic, human-readable names for all enabled capabilities.
    #[must_use]
    pub fn summary_labels(&self) -> Vec<&'static str> {
        let mut labels = Vec::new();
        let checks = [
            (self.network, "network"),
            (self.sqlite, "sqlite"),
            (self.filesystem_xdg, "filesystem:xdg"),
            (self.filesystem_dialogs, "filesystem:dialogs"),
            (self.filesystem_drop, "filesystem:drop"),
            (self.clipboard, "clipboard"),
            (self.notifications, "notifications"),
            (self.compute_threads, "compute-threads"),
            (self.full_system_access, "full-system-access"),
        ];
        for (enabled, label) in checks {
            if enabled {
                labels.push(label);
            }
        }
        labels
    }

    /// Enables or disables a capability from a canonical token string.
    pub fn apply_token(&mut self, token: &str, value: bool) {
        let normalized = token.trim().to_ascii_lowercase().replace('_', "-");
        match normalized.as_str() {
            "network" | "http" => self.network = value,
            "sqlite" | "database" => self.sqlite = value,
            "filesystem" | "fs" => {
                self.filesystem_xdg = value;
                self.filesystem_dialogs = value;
                self.filesystem_drop = value;
            }
            "filesystem:xdg" | "filesystem-xdg" | "filesystemxdg" | "xdg" => {
                self.filesystem_xdg = value;
            }
            "filesystem:dialogs" | "filesystem-dialogs" | "filesystemdialogs" | "dialogs" => {
                self.filesystem_dialogs = value;
            }
            "filesystem:drop" | "filesystem-drop" | "filesystemdrop" | "drop" => {
                self.filesystem_drop = value;
            }
            "clipboard" => self.clipboard = value,
            "notifications" | "notification" => self.notifications = value,
            "compute-threads" | "computethreads" | "compute" | "threads" => {
                self.compute_threads = value;
            }
            "full-system-access" | "fullsystemaccess" => self.full_system_access = value,
            _ => {}
        }
    }

    /// Parses permissions from a pre-parsed [`JsonValue`] (`permissions` block, `capabilities` block, or flat object).
    #[must_use]
    pub(crate) fn from_json_value(value: &JsonValue) -> Self {
        let mut perms = Self::minimal();
        let perm_block = value.get_object("permissions").unwrap_or(value);
        let cap_obj = perm_block.get_object("capabilities").unwrap_or(perm_block);

        if let Some(val) = cap_obj.get_bool("network") {
            perms.network = val;
        }
        if let Some(val) = cap_obj.get_bool("sqlite") {
            perms.sqlite = val;
        }
        if let Some(val) = cap_obj.get_bool("clipboard") {
            perms.clipboard = val;
        }
        if let Some(val) = cap_obj.get_bool("notifications") {
            perms.notifications = val;
        }
        if let Some(val) = cap_obj
            .get_bool("computeThreads")
            .or_else(|| cap_obj.get_bool("compute_threads"))
        {
            perms.compute_threads = val;
        }
        if let Some(val) = cap_obj
            .get_bool("fullSystemAccess")
            .or_else(|| cap_obj.get_bool("full_system_access"))
        {
            perms.full_system_access = val;
        }

        if let Some(fs_obj) = cap_obj.get_object("filesystem") {
            if let Some(val) = fs_obj.get_bool("xdg") {
                perms.filesystem_xdg = val;
            }
            if let Some(val) = fs_obj.get_bool("dialogs") {
                perms.filesystem_dialogs = val;
            }
            if let Some(val) = fs_obj.get_bool("drop") {
                perms.filesystem_drop = val;
            }
        }
        if let Some(val) = cap_obj
            .get_bool("filesystemXdg")
            .or_else(|| cap_obj.get_bool("filesystem_xdg"))
        {
            perms.filesystem_xdg = val;
        }
        if let Some(val) = cap_obj
            .get_bool("filesystemDialogs")
            .or_else(|| cap_obj.get_bool("filesystem_dialogs"))
        {
            perms.filesystem_dialogs = val;
        }
        if let Some(val) = cap_obj
            .get_bool("filesystemDrop")
            .or_else(|| cap_obj.get_bool("filesystem_drop"))
        {
            perms.filesystem_drop = val;
        }

        if let Some(allow_items) = perm_block.get_array("allow") {
            for item in allow_items {
                if let Some(token) = item.as_str() {
                    perms.apply_token(token, true);
                }
            }
        }
        if let Some(deny_items) = perm_block.get_array("deny") {
            for item in deny_items {
                if let Some(token) = item.as_str() {
                    perms.apply_token(token, false);
                }
            }
        }

        perms
    }

    /// Parses permissions from a manifest, `angular.json`, `project.json`, or runtime config string.
    #[must_use]
    pub fn from_json_str(content: &str) -> Self {
        parse_json(content).map_or_else(|_| Self::minimal(), |val| Self::from_json_value(&val))
    }

    /// Serializes this permission manifest as formatted JSON (`permissions.manifest.json` schema).
    #[must_use]
    pub fn to_json(&self) -> String {
        format!(
            concat!(
                "{{\n",
                "  \"schemaVersion\": 1,\n",
                "  \"capabilities\": {{\n",
                "    \"network\": {},\n",
                "    \"sqlite\": {},\n",
                "    \"filesystem\": {{\n",
                "      \"xdg\": {},\n",
                "      \"dialogs\": {},\n",
                "      \"drop\": {}\n",
                "    }},\n",
                "    \"clipboard\": {},\n",
                "    \"notifications\": {},\n",
                "    \"computeThreads\": {},\n",
                "    \"fullSystemAccess\": {}\n",
                "  }}\n",
                "}}"
            ),
            self.network,
            self.sqlite,
            self.filesystem_xdg,
            self.filesystem_dialogs,
            self.filesystem_drop,
            self.clipboard,
            self.notifications,
            self.compute_threads,
            self.full_system_access
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_permissions_default_to_zero_privilege() {
        let perms = AppPermissions::minimal();
        assert!(perms.is_empty());
        assert!(!perms.has_any_filesystem());
        assert!(!perms.full_system_access);
        assert_eq!(perms.summary_labels(), [] as [&str; 0]);
    }

    #[test]
    fn allow_all_sandboxed_keeps_full_system_access_disabled() {
        let perms = AppPermissions::allow_all_sandboxed();
        assert!(perms.network);
        assert!(perms.sqlite);
        assert!(perms.has_any_filesystem());
        assert!(perms.clipboard);
        assert!(perms.notifications);
        assert!(perms.compute_threads);
        assert!(!perms.full_system_access);
    }

    #[test]
    fn parses_nested_manifest_and_allow_deny_overrides_and_roundtrips() {
        let json = r#"{
            "permissions": {
                "capabilities": {
                    "network": true,
                    "sqlite": false,
                    "filesystem": {
                        "xdg": true,
                        "dialogs": true,
                        "drop": false
                    },
                    "clipboard": true,
                    "notifications": false,
                    "computeThreads": true,
                    "fullSystemAccess": false
                },
                "allow": ["notifications", "sqlite"],
                "deny": ["clipboard"]
            }
        }"#;
        let perms = AppPermissions::from_json_str(json);
        assert!(perms.network);
        assert!(perms.sqlite);
        assert!(perms.filesystem_xdg);
        assert!(perms.filesystem_dialogs);
        assert!(!perms.filesystem_drop);
        assert!(!perms.clipboard);
        assert!(perms.notifications);
        assert!(perms.compute_threads);
        assert!(!perms.full_system_access);

        let serialized = perms.to_json();
        let reparsed = AppPermissions::from_json_str(&serialized);
        assert_eq!(reparsed, perms);
    }

    #[test]
    fn returns_minimal_when_angular_json_has_no_permissions_block() {
        let json = r#"{ "projects": { "demo": { "sourceRoot": "src" } } }"#;
        let perms = AppPermissions::from_json_str(json);
        assert_eq!(perms, AppPermissions::minimal());
    }

    #[test]
    fn ignores_permission_like_substrings_inside_unrelated_string_values() {
        let json = r#"{
            "description": "Docs example: {\"network\": true, \"fullSystemAccess\": true}",
            "note": "\"sqlite\": true",
            "permissions": {
                "capabilities": {
                    "clipboard": true
                }
            }
        }"#;
        let perms = AppPermissions::from_json_str(json);
        assert!(perms.clipboard);
        assert!(!perms.network);
        assert!(!perms.sqlite);
        assert!(!perms.full_system_access);
    }
}

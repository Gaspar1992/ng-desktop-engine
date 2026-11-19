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

    /// Parses permissions from a manifest, `angular.json`, `project.json`, or runtime config string.
    #[must_use]
    pub fn from_json_str(content: &str) -> Self {
        let mut perms = Self::minimal();

        let outer_slice = extract_json_object_slice(content, "permissions").or_else(|| {
            if extract_json_object_slice(content, "capabilities").is_some()
                || has_any_permission_key(content)
            {
                Some(content)
            } else {
                None
            }
        });

        let Some(perm_block) = outer_slice else {
            return perms;
        };

        let cap_slice = extract_json_object_slice(perm_block, "capabilities").unwrap_or(perm_block);

        if let Some(val) = extract_json_bool(cap_slice, "network") {
            perms.network = val;
        }
        if let Some(val) = extract_json_bool(cap_slice, "sqlite") {
            perms.sqlite = val;
        }
        if let Some(val) = extract_json_bool(cap_slice, "clipboard") {
            perms.clipboard = val;
        }
        if let Some(val) = extract_json_bool(cap_slice, "notifications") {
            perms.notifications = val;
        }
        if let Some(val) = extract_json_bool(cap_slice, "computeThreads")
            .or_else(|| extract_json_bool(cap_slice, "compute_threads"))
        {
            perms.compute_threads = val;
        }
        if let Some(val) = extract_json_bool(cap_slice, "fullSystemAccess")
            .or_else(|| extract_json_bool(cap_slice, "full_system_access"))
        {
            perms.full_system_access = val;
        }

        if let Some(fs_slice) = extract_json_object_slice(cap_slice, "filesystem") {
            if let Some(val) = extract_json_bool(fs_slice, "xdg") {
                perms.filesystem_xdg = val;
            }
            if let Some(val) = extract_json_bool(fs_slice, "dialogs") {
                perms.filesystem_dialogs = val;
            }
            if let Some(val) = extract_json_bool(fs_slice, "drop") {
                perms.filesystem_drop = val;
            }
        }
        if let Some(val) = extract_json_bool(cap_slice, "filesystemXdg")
            .or_else(|| extract_json_bool(cap_slice, "filesystem_xdg"))
        {
            perms.filesystem_xdg = val;
        }
        if let Some(val) = extract_json_bool(cap_slice, "filesystemDialogs")
            .or_else(|| extract_json_bool(cap_slice, "filesystem_dialogs"))
        {
            perms.filesystem_dialogs = val;
        }
        if let Some(val) = extract_json_bool(cap_slice, "filesystemDrop")
            .or_else(|| extract_json_bool(cap_slice, "filesystem_drop"))
        {
            perms.filesystem_drop = val;
        }

        for token in extract_json_string_array(perm_block, "allow") {
            perms.apply_token(&token, true);
        }
        for token in extract_json_string_array(perm_block, "deny") {
            perms.apply_token(&token, false);
        }

        perms
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

fn has_any_permission_key(content: &str) -> bool {
    [
        "\"network\"",
        "\"sqlite\"",
        "\"filesystem\"",
        "\"filesystemXdg\"",
        "\"filesystem_xdg\"",
        "\"filesystemDialogs\"",
        "\"filesystem_dialogs\"",
        "\"filesystemDrop\"",
        "\"filesystem_drop\"",
        "\"clipboard\"",
        "\"notifications\"",
        "\"computeThreads\"",
        "\"compute_threads\"",
        "\"fullSystemAccess\"",
        "\"full_system_access\"",
    ]
    .iter()
    .any(|key| content.contains(key))
}

fn find_json_key_value_start<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let pattern = format!("\"{key}\"");
    let mut offset = 0;
    while let Some(rel_idx) = json[offset..].find(&pattern) {
        let key_end = offset + rel_idx + pattern.len();
        let after_key = json[key_end..].trim_start();
        if let Some(after_colon) = after_key.strip_prefix(':') {
            return Some(after_colon.trim_start());
        }
        offset = key_end;
    }
    None
}

fn extract_json_bool(json: &str, key: &str) -> Option<bool> {
    let after_colon = find_json_key_value_start(json, key)?;
    if after_colon.starts_with("true") {
        Some(true)
    } else if after_colon.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn extract_json_object_slice<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let after_colon = find_json_key_value_start(json, key)?;
    if !after_colon.starts_with('{') {
        return None;
    }
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;
    for (idx, ch) in after_colon.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && in_string {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(&after_colon[..=idx]);
            }
        }
    }
    None
}

fn extract_json_string_array(json: &str, key: &str) -> Vec<String> {
    let Some(after_colon) = find_json_key_value_start(json, key) else {
        return Vec::new();
    };
    let Some(rest) = after_colon.strip_prefix('[') else {
        return Vec::new();
    };
    let Some(end_bracket) = rest.find(']') else {
        return Vec::new();
    };
    let inside = &rest[..end_bracket];
    let mut items = Vec::new();
    let mut current = inside;
    while let Some(open_q) = current.find('"') {
        let after_open = &current[open_q + 1..];
        let Some(close_q) = after_open.find('"') else {
            break;
        };
        items.push(after_open[..close_q].to_string());
        current = &after_open[close_q + 1..];
    }
    items
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
}

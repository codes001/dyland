use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub schema_version: u32,
    pub name: String,
    pub version: String,
    pub author: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    pub entry_point: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub ui_components: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub config_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub severity: IssueSeverity,
    pub field_path: String,
    pub reason: String,
}

impl ValidationIssue {
    pub fn error(field_path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            severity: IssueSeverity::Error,
            field_path: field_path.into(),
            reason: reason.into(),
        }
    }

    pub fn warning(field_path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            severity: IssueSeverity::Warning,
            field_path: field_path.into(),
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
    pub manifest: Option<PluginManifest>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

impl std::fmt::Display for ValidationReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for e in &self.errors {
            writeln!(f, "error: {}: {}", e.field_path, e.reason)?;
        }
        for w in &self.warnings {
            writeln!(f, "warning: {}: {}", w.field_path, w.reason)?;
        }
        Ok(())
    }
}

const REQUIRED_FIELDS: &[(&str, bool)] = &[
    ("schema_version", true),
    ("name", true),
    ("version", true),
    ("author", true),
    ("entry_point", true),
];

pub fn validate_manifest(json: &str) -> ValidationReport {
    let value: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => {
            return ValidationReport {
                errors: vec![ValidationIssue::error("$", format!("invalid JSON: {e}"))],
                warnings: Vec::new(),
                manifest: None,
            };
        }
    };

    let mut report = ValidationReport {
        errors: Vec::new(),
        warnings: Vec::new(),
        manifest: None,
    };

    for (field, required) in REQUIRED_FIELDS {
        let key = *field;
        if value.get(key).is_none() && *required {
            report
                .errors
                .push(ValidationIssue::error(key, "missing required field"));
        }
    }

    if let Some(sv) = value.get("schema_version") {
        if !sv.is_u64() {
            report.errors.push(ValidationIssue::error(
                "schema_version",
                "must be an integer",
            ));
        }
    }

    for field in ["name", "version", "author", "entry_point"] {
        if let Some(v) = value.get(field) {
            if !v.is_string() {
                report
                    .errors
                    .push(ValidationIssue::error(field, "must be a string"));
            }
        }
    }

    if let Some(p) = value.get("permissions") {
        if !p.is_array() {
            report.errors.push(ValidationIssue::error(
                "permissions",
                "must be an array of strings",
            ));
        } else if let Some(arr) = p.as_array() {
            for (i, perm) in arr.iter().enumerate() {
                if !perm.is_string() {
                    report.errors.push(ValidationIssue::error(
                        format!("permissions[{i}]"),
                        "must be a string",
                    ));
                    continue;
                }
                let perm_str = perm.as_str().unwrap_or_default();
                if !is_known_permission(perm_str) {
                    report.warnings.push(ValidationIssue::warning(
                        format!("permissions[{i}]"),
                        format!("unknown permission '{perm_str}' ignored (forward-compatible)"),
                    ));
                }
            }
        }
    }

    if let Some(ui) = value.get("ui_components") {
        if !ui.is_array() {
            report.errors.push(ValidationIssue::error(
                "ui_components",
                "must be an array of strings",
            ));
        } else if let Some(arr) = ui.as_array() {
            for (i, comp) in arr.iter().enumerate() {
                if !comp.is_string() {
                    report.errors.push(ValidationIssue::error(
                        format!("ui_components[{i}]"),
                        "must be a string",
                    ));
                }
            }
        }
    }

    if let Some(version) = value.get("version") {
        if let Some(v) = version.as_str() {
            let is_strict = is_strict_semver(v);
            let best_effort = parse_semver_best_effort(v).is_some();
            if !is_strict && best_effort {
                report.warnings.push(ValidationIssue::warning(
                    "version",
                    format!("'{v}' is not strict semver (expected X.Y.Z); parsed best-effort"),
                ));
            } else if !is_strict {
                report.warnings.push(ValidationIssue::warning(
                    "version",
                    format!("'{v}' is not valid semver; parsed best-effort"),
                ));
            }
        }
    }

    if report.errors.is_empty() {
        match serde_json::from_value::<PluginManifest>(value) {
            Ok(manifest) => {
                report.manifest = Some(manifest);
                report
            }
            Err(e) => ValidationReport {
                errors: vec![ValidationIssue::error("$", format!("schema mismatch: {e}"))],
                warnings: report.warnings,
                manifest: None,
            },
        }
    } else {
        report
    }
}

fn is_known_permission(perm: &str) -> bool {
    let (kind, _rest) = match perm.split_once(':') {
        Some(pair) => pair,
        None => return true,
    };
    matches!(
        kind,
        "dbus" | "http" | "https" | "unix_socket" | "system" | "api" | "notification"
    )
}

pub fn is_strict_semver(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts
        .iter()
        .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

pub fn parse_semver_best_effort(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    let patch = parts
        .next()
        .unwrap_or("0")
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse::<u64>()
        .ok()?;
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest_json() -> String {
        serde_json::json!({
            "schema_version": 1,
            "name": "docker-status",
            "version": "0.1.0",
            "author": "dyland",
            "description": "Show Docker container status in Dynamic Island",
            "entry_point": "plugin.wasm",
            "permissions": ["dbus:org.freedesktop.DBus", "http:localhost:2375"],
            "ui_components": ["expanded"],
            "config_schema": {
                "type": "object",
                "properties": {
                    "docker_host": { "type": "string", "default": "unix:///var/run/docker.sock" }
                }
            }
        })
        .to_string()
    }

    #[test]
    fn parses_valid_full_manifest_cleanly() {
        let report = validate_manifest(&valid_manifest_json());
        assert!(report.is_valid(), "valid manifest should have no errors");
        assert_eq!(report.warnings.len(), 0);
        let manifest = report.manifest.expect("manifest present on success");
        assert_eq!(manifest.name, "docker-status");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.author, "dyland");
        assert_eq!(manifest.entry_point, "plugin.wasm");
        assert_eq!(manifest.permissions.len(), 2);
        assert_eq!(manifest.ui_components, vec!["expanded".to_string()]);
        assert!(manifest.config_schema.is_some());
        assert!(manifest.description.is_some());
    }

    #[test]
    fn missing_required_field_errors_with_field_name() {
        let json = serde_json::json!({
            "schema_version": 1,
            "name": "docker-status",
            "version": "0.1.0",
            "author": "dyland",
            "entry_point": "plugin.wasm"
        });
        let mut missing_author = json.clone();
        missing_author.as_object_mut().unwrap().remove("author");
        let report = validate_manifest(&missing_author.to_string());
        assert!(!report.is_valid());
        assert!(report
            .errors
            .iter()
            .any(|e| e.field_path == "author" && e.reason.contains("missing")));

        let mut missing_name = json.clone();
        missing_name.as_object_mut().unwrap().remove("name");
        let report = validate_manifest(&missing_name.to_string());
        assert!(report.errors.iter().any(|e| e.field_path == "name"));
    }

    #[test]
    fn unknown_permission_warns_but_not_errors() {
        let mut value: serde_json::Value = serde_json::from_str(&valid_manifest_json()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("permissions".into(), serde_json::json!(["future:service"]));
        let report = validate_manifest(&value.to_string());
        assert_eq!(report.errors.len(), 0);
        assert!(report
            .warnings
            .iter()
            .any(|w| w.field_path == "permissions[0]" && w.reason.contains("unknown permission")));
    }

    #[test]
    fn non_semver_version_warns_but_parses() {
        let mut value: serde_json::Value = serde_json::from_str(&valid_manifest_json()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("version".into(), serde_json::json!("1.2"));
        let report = validate_manifest(&value.to_string());
        assert_eq!(report.errors.len(), 0);
        assert!(report.warnings.iter().any(|w| w.field_path == "version"));
        assert!(report.manifest.is_some());
    }

    #[test]
    fn invalid_json_fails_parse_with_reason() {
        let report = validate_manifest("{not json");
        assert!(report
            .errors
            .iter()
            .any(|e| e.field_path == "$" && e.reason.contains("invalid JSON")));
    }

    #[test]
    fn config_schema_passthrough_preserved() {
        let report = validate_manifest(&valid_manifest_json());
        let manifest = report.manifest.expect("manifest present on success");
        let schema = manifest.config_schema.unwrap();
        assert_eq!(
            schema["properties"]["docker_host"]["default"],
            "unix:///var/run/docker.sock"
        );
    }

    #[test]
    fn wrong_field_type_errors() {
        let mut value: serde_json::Value = serde_json::from_str(&valid_manifest_json()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("name".into(), serde_json::json!(123));
        let report = validate_manifest(&value.to_string());
        assert!(report
            .errors
            .iter()
            .any(|e| e.field_path == "name" && e.reason.contains("must be a string")));
    }

    #[test]
    fn semver_parser_handles_edge_cases() {
        assert_eq!(parse_semver_best_effort("0.1.0"), Some((0, 1, 0)));
        assert_eq!(parse_semver_best_effort("1.2"), Some((1, 2, 0)));
        assert_eq!(parse_semver_best_effort("2.0.0-beta.1"), Some((2, 0, 0)));
        assert_eq!(parse_semver_best_effort("not-a-version"), None);
    }
}

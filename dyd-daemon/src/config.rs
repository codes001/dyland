//! Persistent application configuration (Epic 7, story 7-1).
//!
//! [`ConfigStore`] owns the on-disk `config.json` and the in-memory view of it.
//! The value is stored as a raw [`serde_json::Value`] that is deep-merged over
//! [`AppConfig::default`], so:
//!   * every known key is always present (a missing/empty file still hydrates a
//!     complete config), and
//!   * unknown fields written by a newer build are preserved across writes by an
//!     older one.
//!
//! Updates are validated by round-tripping the merged value through
//! [`AppConfig`] (a bad type is rejected without mutating state), fanned out to
//! all connected clients via a [`broadcast`] channel, and flushed to disk with a
//! 500 ms debounce using an atomic temp-file + rename.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;
use tokio::sync::broadcast;

use dyd_types::{AppConfig, ConfigChangedPayload};

/// Debounce window before an in-memory update is flushed to disk.
const WRITE_DEBOUNCE: Duration = Duration::from_millis(500);

pub struct ConfigStore {
    /// Absolute path to `config.json`.
    path: PathBuf,
    /// Current fully-hydrated config, stored as a raw value so unknown fields
    /// survive round-trips.
    inner: Mutex<Value>,
    /// Fan-out of full-snapshot changes to every open connection.
    tx: broadcast::Sender<ConfigChangedPayload>,
    /// Monotonic write generation; the debounced writer only flushes if its
    /// captured generation is still current (coalesces bursts of updates).
    write_gen: AtomicU64,
    /// Debounce window (overridable in tests).
    debounce: Duration,
}

impl ConfigStore {
    /// Resolve the config file path. `$DYD_CONFIG_DIR` overrides everything (for
    /// tests); otherwise `$XDG_CONFIG_HOME/dyd` → `$HOME/.config/dyd`.
    pub fn config_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("DYD_CONFIG_DIR") {
            return PathBuf::from(dir);
        }
        let base = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                PathBuf::from(home).join(".config")
            });
        base.join("dyd")
    }

    fn config_path() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    /// Load the store from disk, hydrating a full config over the defaults. A
    /// missing file yields defaults; a corrupt or type-invalid file logs a
    /// warning and falls back to defaults (the bad file is left untouched until
    /// the next successful write overwrites it).
    pub fn load() -> Arc<Self> {
        let path = Self::config_path();
        let value = Self::load_value(&path);
        let (tx, _rx) = broadcast::channel(16);
        Arc::new(Self {
            path,
            inner: Mutex::new(value),
            tx,
            write_gen: AtomicU64::new(0),
            debounce: WRITE_DEBOUNCE,
        })
    }

    /// Read + merge the config value at `path`, always returning a valid,
    /// fully-hydrated value.
    fn load_value(path: &PathBuf) -> Value {
        let defaults = serde_json::to_value(AppConfig::default())
            .expect("AppConfig::default must serialize");

        let on_disk = match std::fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str::<Value>(&text) {
                Ok(v) => Some(v),
                Err(e) => {
                    tracing::warn!(error = %e, path = %path.display(), "config.json is not valid JSON; using defaults");
                    None
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => {
                tracing::warn!(error = %e, path = %path.display(), "failed to read config.json; using defaults");
                None
            }
        };

        let mut merged = defaults.clone();
        if let Some(disk) = on_disk {
            deep_merge(&mut merged, &disk);
        }

        // Validate: the merged value must deserialize into AppConfig. If a
        // hand-edited file has a wrong type, discard it and use defaults.
        if serde_json::from_value::<AppConfig>(merged.clone()).is_err() {
            tracing::warn!(path = %path.display(), "config.json has invalid field types; using defaults");
            return defaults;
        }
        merged
    }

    /// A subscriber for the config broadcast, added to `broadcast_receivers()`.
    pub fn subscribe(&self) -> broadcast::Receiver<ConfigChangedPayload> {
        self.tx.subscribe()
    }

    /// The broadcast sender (used to build a receiver in `SourceContext`).
    pub fn sender(&self) -> broadcast::Sender<ConfigChangedPayload> {
        self.tx.clone()
    }

    /// The current full config value.
    pub fn snapshot(&self) -> Value {
        self.inner.lock().expect("config lock poisoned").clone()
    }

    /// Apply a partial patch: deep-merge `changes` over the current value,
    /// validate the result, store it, broadcast the full snapshot, and schedule
    /// a debounced write. Returns the new full snapshot, or an error string if
    /// the merged result is not a valid [`AppConfig`] (in which case state is
    /// left unchanged).
    pub fn update(self: &Arc<Self>, changes: Value) -> Result<Value, String> {
        let snapshot = {
            let mut guard = self.inner.lock().expect("config lock poisoned");
            let mut candidate = guard.clone();
            deep_merge(&mut candidate, &changes);
            // Reject invalid patches before committing.
            serde_json::from_value::<AppConfig>(candidate.clone())
                .map_err(|e| format!("invalid config: {e}"))?;
            *guard = candidate.clone();
            candidate
        };

        // Fan out the full snapshot; ignore "no receivers".
        let _ = self.tx.send(ConfigChangedPayload {
            changes: snapshot.clone(),
        });

        self.schedule_write();
        Ok(snapshot)
    }

    /// Bump the write generation and spawn a debounced writer. Only the newest
    /// scheduled write actually flushes, coalescing rapid successive updates.
    fn schedule_write(self: &Arc<Self>) {
        let generation = self.write_gen.fetch_add(1, Ordering::SeqCst) + 1;
        let store = Arc::clone(self);
        let debounce = self.debounce;
        tokio::spawn(async move {
            tokio::time::sleep(debounce).await;
            if store.write_gen.load(Ordering::SeqCst) != generation {
                return; // superseded by a newer update
            }
            if let Err(e) = store.persist_now() {
                tracing::error!(error = %e, path = %store.path.display(), "failed to persist config.json");
            }
        });
    }

    /// Write the current value to disk atomically (temp file + rename). Public
    /// so tests can force a synchronous flush without waiting on the debounce.
    pub fn persist_now(&self) -> std::io::Result<()> {
        let value = self.snapshot();
        let text = serde_json::to_string_pretty(&value)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, text.as_bytes())?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

/// Recursively merge `patch` into `base`. Objects merge key-by-key; every other
/// JSON kind (including arrays) replaces wholesale. `null` in `patch` overwrites
/// the base value with `null` (callers omit keys they don't intend to change).
fn deep_merge(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(base_map), Value::Object(patch_map)) => {
            for (k, v) in patch_map {
                deep_merge(base_map.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (base_slot, patch_val) => {
            *base_slot = patch_val.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a store rooted at a unique temp dir, with an immediate (0 ms)
    /// debounce so scheduled writes flush promptly in async tests.
    fn temp_store(tag: &str) -> (Arc<ConfigStore>, PathBuf) {
        let dir = std::env::temp_dir().join(format!("dyd-cfg-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let (tx, _rx) = broadcast::channel(16);
        let value = ConfigStore::load_value(&path);
        let store = Arc::new(ConfigStore {
            path,
            inner: Mutex::new(value),
            tx,
            write_gen: AtomicU64::new(0),
            debounce: Duration::from_millis(0),
        });
        (store, dir)
    }

    #[test]
    fn missing_file_yields_full_defaults() {
        let (store, dir) = temp_store("missing");
        let snap = store.snapshot();
        assert_eq!(snap["appearance"]["theme"], "dark");
        assert_eq!(snap["features"]["battery"], true);
        assert_eq!(snap["general"]["start_on_login"], false);
        assert!(snap["last_category"].is_null());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn deep_merge_patches_single_leaf_only() {
        let mut base = serde_json::json!({
            "appearance": {"theme": "dark"},
            "features": {"battery": true, "wifi": true}
        });
        deep_merge(&mut base, &serde_json::json!({"features": {"wifi": false}}));
        assert_eq!(base["appearance"]["theme"], "dark");
        assert_eq!(base["features"]["battery"], true);
        assert_eq!(base["features"]["wifi"], false);
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults() {
        let (_store, dir) = temp_store("corrupt");
        let path = dir.join("config.json");
        std::fs::write(&path, "{ not valid json ").unwrap();
        let value = ConfigStore::load_value(&path);
        assert_eq!(value["appearance"]["theme"], "dark");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn unknown_fields_are_preserved() {
        let (_store, dir) = temp_store("unknown");
        let path = dir.join("config.json");
        std::fs::write(&path, r#"{"future_key": {"nested": 1}, "appearance": {"theme": "light"}}"#)
            .unwrap();
        let value = ConfigStore::load_value(&path);
        assert_eq!(value["future_key"]["nested"], 1);
        assert_eq!(value["appearance"]["theme"], "light");
        // known defaults still hydrated alongside the unknown field
        assert_eq!(value["features"]["battery"], true);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn invalid_field_type_in_file_uses_defaults() {
        let (_store, dir) = temp_store("badtype");
        let path = dir.join("config.json");
        std::fs::write(&path, r#"{"appearance": {"theme": 123}}"#).unwrap();
        let value = ConfigStore::load_value(&path);
        // theme must be a string; the whole file is discarded → defaults
        assert_eq!(value["appearance"]["theme"], "dark");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn update_rejects_bad_type_without_mutating() {
        let (store, dir) = temp_store("reject");
        let before = store.snapshot();
        let err = store
            .update(serde_json::json!({"appearance": {"theme": 42}}))
            .unwrap_err();
        assert!(err.contains("invalid config"));
        assert_eq!(store.snapshot(), before);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn update_persists_atomically_and_reloads() {
        let (store, dir) = temp_store("persist");
        store
            .update(serde_json::json!({"appearance": {"theme": "light"}}))
            .unwrap();
        // force synchronous flush (debounce is 0 but avoid task-timing races)
        store.persist_now().unwrap();

        let reloaded = ConfigStore::load_value(&store.path);
        assert_eq!(reloaded["appearance"]["theme"], "light");
        assert_eq!(reloaded["features"]["battery"], true);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn update_broadcasts_full_snapshot() {
        let (store, dir) = temp_store("broadcast");
        let mut rx = store.subscribe();
        store
            .update(serde_json::json!({"features": {"wifi": false}}))
            .unwrap();
        let msg = rx.recv().await.unwrap();
        assert_eq!(msg.changes["features"]["wifi"], false);
        // full snapshot, not just the patch
        assert_eq!(msg.changes["appearance"]["theme"], "dark");
        let _ = std::fs::remove_dir_all(dir);
    }
}

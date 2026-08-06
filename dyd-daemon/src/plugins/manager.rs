use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use dyd_types::{IpcEnvelope, IpcEvent, PluginHealthPayload};
use tokio::sync::{broadcast, RwLock};

use super::handle::{health_payload, HealthSnapshot, PluginHandle, PluginState};
use super::host::{PluginExecutor, TickOutcome};
use super::runtime::HostContext;

const START_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub enum PluginError {
    NotFound(String),
    Duplicate(String),
    InvalidManifest(String),
    Operation(String),
    Timeout(String),
    Disabled(String),
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginError::NotFound(n) => write!(f, "plugin not found: {n}"),
            PluginError::Duplicate(n) => write!(f, "plugin already registered: {n}"),
            PluginError::InvalidManifest(m) => write!(f, "invalid manifest: {m}"),
            PluginError::Operation(o) => write!(f, "plugin operation failed: {o}"),
            PluginError::Timeout(n) => write!(f, "plugin timed out: {n}"),
            PluginError::Disabled(n) => write!(f, "plugin is disabled: {n}"),
        }
    }
}

impl std::error::Error for PluginError {}

pub struct PluginManager {
    registry: RwLock<HashMap<String, PluginHandle>>,
    event_broadcast: broadcast::Sender<IpcEnvelope>,
    executor: Arc<dyn PluginExecutor>,
    ctx: Option<Arc<HostContext>>,
}

impl PluginManager {
    pub fn new(executor: Arc<dyn PluginExecutor>) -> Self {
        let (event_broadcast, _) = broadcast::channel(64);
        Self {
            registry: RwLock::new(HashMap::new()),
            event_broadcast,
            executor,
            ctx: None,
        }
    }

    /// Construct a manager wired to a shared [`HostContext`] and a broadcast
    /// sender that is also used by the IPC server. Used in production (story
    /// 6-3); `new` remains for tests that use the stub executor.
    pub fn from_parts(
        executor: Arc<dyn PluginExecutor>,
        event_broadcast: broadcast::Sender<IpcEnvelope>,
        ctx: Arc<HostContext>,
    ) -> Self {
        Self {
            registry: RwLock::new(HashMap::new()),
            event_broadcast,
            executor,
            ctx: Some(ctx),
        }
    }

    pub fn event_broadcast(&self) -> broadcast::Sender<IpcEnvelope> {
        self.event_broadcast.clone()
    }

    pub fn plugins_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("DYD_PLUGINS_DIR") {
            return PathBuf::from(dir);
        }
        let base = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                PathBuf::from(home).join(".local").join("share")
            });
        base.join("dyd").join("plugins")
    }

    pub async fn scan_plugins(&self) -> Vec<String> {
        let dir = Self::plugins_dir();
        if !dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&dir) {
                tracing::warn!(error = %e, path = %dir.display(), "failed to create plugins dir");
            }
            return Vec::new();
        }
        let mut loaded = Vec::new();
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(error = %e, path = %dir.display(), "failed to scan plugins dir");
                return Vec::new();
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            match self.load_plugin_from_dir(&path).await {
                Ok(name) => loaded.push(name),
                Err(e) => {
                    tracing::warn!(error = %e, path = %path.display(), "skipping invalid plugin");
                }
            }
        }
        loaded
    }

    async fn load_plugin_from_dir(&self, dir: &Path) -> Result<String, PluginError> {
        let manifest_path = dir.join("manifest.json");
        let manifest_str = std::fs::read_to_string(&manifest_path)
            .map_err(|e| PluginError::InvalidManifest(format!("cannot read manifest.json: {e}")))?;
        let report = dyd_types::plugin::validate_manifest(&manifest_str);
        if !report.is_valid() {
            return Err(PluginError::InvalidManifest(format!("{report}")));
        }
        for w in &report.warnings {
            tracing::warn!(plugin = %dir.display(), warning = %w.reason, "manifest warning");
        }
        let manifest = report.manifest.ok_or_else(|| {
            PluginError::InvalidManifest("validation passed but manifest missing".to_string())
        })?;
        self.register(manifest, dir.to_path_buf(), false).await
    }

    pub async fn register(
        &self,
        manifest: dyd_types::PluginManifest,
        directory: PathBuf,
        is_core: bool,
    ) -> Result<String, PluginError> {
        let name = manifest.name.clone();
        let mut handle = PluginHandle::new(manifest, directory, is_core);
        // Seed the shared config map before init so guest `get_config` works
        // during the `init` hook.
        if let Some(ctx) = &self.ctx {
            ctx.set_config(&name, handle.config.clone());
        }
        {
            let mut registry = self.registry.write().await;
            if registry.contains_key(&name) {
                return Err(PluginError::Duplicate(name));
            }
            if let Err(e) = self
                .executor
                .init(&name, &handle.manifest, &handle.directory)
            {
                handle.record_error(format!("init failed: {e}"));
                registry.insert(name.clone(), handle);
                tracing::warn!(plugin = %name, error = %e, "plugin registered but init failed");
            } else {
                handle.state = PluginState::Initialized;
                registry.insert(name.clone(), handle);
                tracing::info!(plugin = %name, "plugin registered and initialized");
            }
        }
        let payload = self
            .health_for(&name)
            .await
            .unwrap_or_else(|| PluginHealthPayload {
                name: name.clone(),
                version: String::new(),
                state: "loaded".to_string(),
                last_error: None,
                error_count: 0,
                uptime_secs: 0,
                memory_usage: None,
                disabled: false,
                is_core,
                permissions: Vec::new(),
                ui_components: Vec::new(),
                config_schema: None,
                config: Some(serde_json::json!({})),
            });
        let _ = self
            .event_broadcast
            .send(IpcEnvelope::event(IpcEvent::PluginChanged(payload)));
        Ok(name)
    }

    pub async fn list(&self) -> Vec<PluginHandle> {
        let registry = self.registry.read().await;
        registry.values().cloned().collect()
    }

    pub async fn get(&self, name: &str) -> Option<PluginHandle> {
        let registry = self.registry.read().await;
        registry.get(name).cloned()
    }

    pub async fn start_plugin(&self, name: &str) -> Result<(), PluginError> {
        let result = {
            let mut registry = self.registry.write().await;
            let handle = registry
                .get_mut(name)
                .ok_or_else(|| PluginError::NotFound(name.to_string()))?;
            if handle.disabled {
                return Err(PluginError::Disabled(name.to_string()));
            }
            let n = name.to_string();
            let executor = self.executor.clone();
            match run_with_timeout(move || executor.start(&n)).await {
                Ok(Ok(())) => {
                    handle.state = PluginState::Running;
                    tracing::info!(plugin = %name, "plugin running");
                    Ok(())
                }
                Ok(Err(e)) => {
                    handle.state = PluginState::Error;
                    handle.record_crash(format!("start failed: {e}"));
                    tracing::warn!(plugin = %name, error = %e, "plugin start failed");
                    Err(PluginError::Operation(format!("start: {e}")))
                }
                Err(_) => {
                    handle.state = PluginState::Stopped;
                    handle.record_error("start timed out after 5s");
                    tracing::warn!(plugin = %name, "plugin start timed out; force-stopped");
                    Err(PluginError::Timeout(name.to_string()))
                }
            }
        };
        self.broadcast_health_if_exists(name).await;
        result
    }

    pub async fn start_all(&self) -> Vec<Result<(), PluginError>> {
        let names: Vec<String> = self.list().await.into_iter().map(|h| h.name).collect();
        let mut results = Vec::new();
        for name in names {
            results.push(self.start_plugin(&name).await);
        }
        results
    }

    pub async fn stop_plugin(&self, name: &str) -> Result<(), PluginError> {
        let result = {
            let mut registry = self.registry.write().await;
            let handle = registry
                .get_mut(name)
                .ok_or_else(|| PluginError::NotFound(name.to_string()))?;
            let n = name.to_string();
            let executor = self.executor.clone();
            match run_with_timeout(move || executor.stop(&n)).await {
                Ok(Ok(())) => {
                    handle.state = PluginState::Stopped;
                    tracing::info!(plugin = %name, "plugin stopped");
                    Ok(())
                }
                Ok(Err(e)) => {
                    handle.record_error(format!("stop failed: {e}"));
                    tracing::warn!(plugin = %name, error = %e, "plugin stop failed");
                    Ok(())
                }
                Err(_) => {
                    handle.record_error("stop timed out; force stopped");
                    handle.state = PluginState::Stopped;
                    tracing::warn!(plugin = %name, "plugin stop timed out; force-stopped");
                    Ok(())
                }
            }
        };
        self.broadcast_health_if_exists(name).await;
        result
    }

    pub async fn unload_plugin(&self, name: &str) -> Result<(), PluginError> {
        {
            let mut registry = self.registry.write().await;
            let handle = registry
                .get_mut(name)
                .ok_or_else(|| PluginError::NotFound(name.to_string()))?;
            let n = name.to_string();
            let executor = self.executor.clone();
            let _ = run_with_timeout(move || executor.unload(&n)).await;
            handle.state = PluginState::Unloaded;
            tracing::info!(plugin = %name, "plugin unloaded");
            registry.remove(name);
        }
        Ok(())
    }

    pub async fn set_enabled(&self, name: &str, enabled: bool) -> Result<(), PluginError> {
        {
            let mut registry = self.registry.write().await;
            let handle = registry
                .get_mut(name)
                .ok_or_else(|| PluginError::NotFound(name.to_string()))?;
            if enabled {
                handle.disabled = false;
                handle.reset_errors();
                if handle.state == PluginState::Error {
                    handle.state = PluginState::Loaded;
                }
                tracing::info!(plugin = %name, "plugin enabled");
            } else {
                handle.disabled = true;
                let n = name.to_string();
                let executor = self.executor.clone();
                let _ = run_with_timeout(move || executor.stop(&n)).await;
                handle.state = PluginState::Stopped;
                tracing::info!(plugin = %name, "plugin disabled");
            }
        }
        self.broadcast_health_if_exists(name).await;
        Ok(())
    }

    pub async fn set_config(
        &self,
        name: &str,
        config: serde_json::Value,
    ) -> Result<(), PluginError> {
        {
            let mut registry = self.registry.write().await;
            let handle = registry
                .get_mut(name)
                .ok_or_else(|| PluginError::NotFound(name.to_string()))?;
            handle.config = config.clone();
            tracing::info!(plugin = %name, "plugin config updated");
        }
        // Write through to the shared map so running instances see the update.
        if let Some(ctx) = &self.ctx {
            ctx.set_config(name, config);
        }
        self.broadcast_health_if_exists(name).await;
        Ok(())
    }

    pub async fn report_crash(&self, name: &str) {
        let mut disabled = false;
        {
            let mut registry = self.registry.write().await;
            let Some(handle) = registry.get_mut(name) else {
                return;
            };
            handle.record_crash("crash detected at runtime");
            handle.state = PluginState::Error;
            tracing::error!(plugin = %name, count = %handle.error_count, "plugin crashed");
            if handle.should_auto_disable() {
                handle.disabled = true;
                disabled = true;
                tracing::error!(plugin = %name, "plugin auto-disabled after repeated crashes");
            }
        }
        self.broadcast_health_if_exists(name).await;
        if disabled {
            self.broadcast_health_if_exists(name).await;
        }
    }

    /// Tick every running, non-disabled plugin once. Batches the whole set into
    /// a single `spawn_blocking` hop (avoiding per-plugin thread churn), then
    /// applies outcomes back on the async side: crashes route through
    /// [`report_crash`](Self::report_crash) (feeding 6-2 auto-disable), pauses
    /// are logged, and memory usage is refreshed for the health payload.
    pub async fn tick_all(&self) {
        let names: Vec<String> = {
            let registry = self.registry.read().await;
            registry
                .values()
                .filter(|h| h.state == PluginState::Running && !h.disabled)
                .map(|h| h.name.clone())
                .collect()
        };
        if names.is_empty() {
            return;
        }
        let executor = self.executor.clone();
        let outcomes = tokio::task::spawn_blocking(move || {
            names
                .into_iter()
                .map(|name| {
                    let outcome = executor.tick(&name);
                    let mem = executor.memory_usage(&name);
                    (name, outcome, mem)
                })
                .collect::<Vec<_>>()
        })
        .await;
        let outcomes = match outcomes {
            Ok(o) => o,
            Err(e) => {
                tracing::warn!(error = %e, "plugin tick batch panicked");
                return;
            }
        };
        for (name, outcome, mem) in outcomes {
            match outcome {
                TickOutcome::Crashed(reason) => {
                    tracing::warn!(plugin = %name, %reason, "plugin crashed during tick");
                    self.report_crash(&name).await;
                }
                TickOutcome::Paused => {
                    tracing::debug!(plugin = %name, "plugin paused (out of tick fuel)");
                }
                TickOutcome::Ran | TickOutcome::Idle => {}
            }
            if let Some(mem) = mem {
                let mut registry = self.registry.write().await;
                if let Some(handle) = registry.get_mut(&name) {
                    handle.memory_usage = Some(mem);
                }
            }
        }
    }

    pub async fn emit_activity(&self, source: String, content: serde_json::Value) {
        let envelope = IpcEnvelope::event(IpcEvent::PluginActivity(dyd_types::ActivityPayload {
            activity_id: uuid::Uuid::new_v4().to_string(),
            source,
            content,
        }));
        let _ = self.event_broadcast.send(envelope);
    }

    pub async fn health_snapshot(&self, name: &str) -> Option<HealthSnapshot> {
        let registry = self.registry.read().await;
        registry.get(name).map(HealthSnapshot::from)
    }

    async fn health_for(&self, name: &str) -> Option<PluginHealthPayload> {
        let registry = self.registry.read().await;
        registry.get(name).map(health_payload)
    }

    async fn broadcast_health_if_exists(&self, name: &str) {
        if let Some(payload) = self.health_for(name).await {
            let _ = self
                .event_broadcast
                .send(IpcEnvelope::event(IpcEvent::PluginChanged(payload)));
        }
    }

    pub async fn install_plugin(&self, path: &Path) -> Result<String, PluginError> {
        let manifest_str = std::fs::read_to_string(path.join("manifest.json"))
            .map_err(|e| PluginError::InvalidManifest(format!("cannot read manifest.json: {e}")))?;
        let report = dyd_types::plugin::validate_manifest(&manifest_str);
        if !report.is_valid() {
            return Err(PluginError::InvalidManifest(format!("{report}")));
        }
        let manifest = report.manifest.ok_or_else(|| {
            PluginError::InvalidManifest("validation passed but manifest missing".to_string())
        })?;
        let name = manifest.name.clone();
        let dest = Self::plugins_dir().join(&name);
        if dest.exists() {
            std::fs::remove_dir_all(&dest).map_err(|e| {
                PluginError::Operation(format!("failed to replace existing plugin dir: {e}"))
            })?;
        }
        std::fs::create_dir_all(&dest)
            .map_err(|e| PluginError::Operation(format!("failed to create plugin dir: {e}")))?;
        for entry in std::fs::read_dir(path)
            .map_err(|e| PluginError::Operation(format!("failed to read source plugin dir: {e}")))?
        {
            let entry = entry
                .map_err(|e| PluginError::Operation(format!("failed to read source entry: {e}")))?;
            let src = entry.path();
            let target = dest.join(entry.file_name());
            if src.is_dir() {
                copy_dir(&src, &target)
                    .map_err(|e| PluginError::Operation(format!("failed to copy dir: {e}")))?;
            } else {
                std::fs::copy(&src, &target)
                    .map_err(|e| PluginError::Operation(format!("failed to copy file: {e}")))?;
            }
        }
        self.register(manifest, dest, false).await
    }

    pub async fn uninstall_plugin(&self, name: &str) -> Result<(), PluginError> {
        self.unload_plugin(name).await?;
        let dir = Self::plugins_dir().join(name);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|e| {
                PluginError::Operation(format!("failed to remove plugin files: {e}"))
            })?;
        }
        Ok(())
    }
}

async fn run_with_timeout<F>(f: F) -> Result<Result<(), String>, ()>
where
    F: FnOnce() -> Result<(), String> + Send + 'static,
{
    let handle = tokio::task::spawn_blocking(f);
    match tokio::time::timeout(START_TIMEOUT, handle).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(join_err)) => {
            tracing::warn!(error = %join_err, "plugin executor task panicked");
            Err(())
        }
        Err(_) => Err(()),
    }
}

fn copy_dir(src: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let s = entry.path();
        let d = dest.join(entry.file_name());
        if s.is_dir() {
            copy_dir(&s, &d)?;
        } else {
            std::fs::copy(&s, &d)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::host::StubExecutor;
    use super::*;
    use std::sync::Arc;

    fn test_manifest(name: &str) -> dyd_types::PluginManifest {
        dyd_types::PluginManifest {
            schema_version: 1,
            name: name.to_string(),
            version: "0.1.0".to_string(),
            author: "test".to_string(),
            description: None,
            homepage: None,
            license: None,
            entry_point: "plugin.wasm".to_string(),
            permissions: Vec::new(),
            ui_components: Vec::new(),
            dependencies: Vec::new(),
            config_schema: None,
        }
    }

    #[tokio::test]
    async fn registry_registers_and_lists() {
        let manager = PluginManager::new(Arc::new(StubExecutor));
        let name = manager
            .register(test_manifest("alpha"), PathBuf::from("/tmp/alpha"), false)
            .await
            .expect("register should succeed");
        assert_eq!(name, "alpha");
        let list = manager.list().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "alpha");
        assert_eq!(list[0].state, PluginState::Initialized);
    }

    #[tokio::test]
    async fn registry_rejects_duplicate_names() {
        let manager = PluginManager::new(Arc::new(StubExecutor));
        manager
            .register(test_manifest("alpha"), PathBuf::from("/tmp/alpha"), false)
            .await
            .expect("first register ok");
        let err = manager
            .register(test_manifest("alpha"), PathBuf::from("/tmp/alpha2"), false)
            .await
            .unwrap_err();
        assert!(matches!(err, PluginError::Duplicate(_)));
    }

    #[tokio::test]
    async fn lifecycle_transitions_loaded_initialized_running() {
        let manager = PluginManager::new(Arc::new(StubExecutor));
        let name = manager
            .register(test_manifest("alpha"), PathBuf::from("/tmp/alpha"), false)
            .await
            .expect("register ok");
        manager.start_plugin(&name).await.expect("start ok");
        let h = manager.get(&name).await.unwrap();
        assert_eq!(h.state, PluginState::Running);
        manager.stop_plugin(&name).await.expect("stop ok");
        let h = manager.get(&name).await.unwrap();
        assert_eq!(h.state, PluginState::Stopped);
    }

    #[tokio::test]
    async fn unload_removes_from_registry() {
        let manager = PluginManager::new(Arc::new(StubExecutor));
        let name = manager
            .register(test_manifest("alpha"), PathBuf::from("/tmp/alpha"), false)
            .await
            .expect("register ok");
        manager.unload_plugin(&name).await.expect("unload ok");
        assert!(manager.get(&name).await.is_none());
    }

    #[tokio::test]
    async fn disabled_plugin_rejects_start() {
        let manager = PluginManager::new(Arc::new(StubExecutor));
        let name = manager
            .register(test_manifest("alpha"), PathBuf::from("/tmp/alpha"), false)
            .await
            .expect("register ok");
        manager.set_enabled(&name, false).await.expect("disable ok");
        let err = manager.start_plugin(&name).await.unwrap_err();
        assert!(matches!(err, PluginError::Disabled(_)));
        let h = manager.get(&name).await.unwrap();
        assert!(h.disabled);
    }

    #[tokio::test]
    async fn discovery_scans_dir_and_skips_invalid_manifests() {
        let temp = std::env::temp_dir().join(format!("dyd-test-{}", uuid::Uuid::new_v4()));
        let good = temp.join("good");
        let bad = temp.join("bad");
        std::fs::create_dir_all(&good).unwrap();
        std::fs::create_dir_all(&bad).unwrap();
        std::fs::write(
            good.join("manifest.json"),
            serde_json::json!({
                "schema_version": 1,
                "name": "good",
                "version": "0.1.0",
                "author": "t",
                "entry_point": "plugin.wasm"
            })
            .to_string(),
        )
        .unwrap();
        std::fs::write(bad.join("manifest.json"), "not json").unwrap();
        std::fs::write(good.join("plugin.wasm"), "wasm").unwrap();

        std::env::set_var("DYD_PLUGINS_DIR", &temp);
        let manager = PluginManager::new(Arc::new(StubExecutor));
        let loaded = manager.scan_plugins().await;
        std::env::remove_var("DYD_PLUGINS_DIR");
        let _ = std::fs::remove_dir_all(&temp);
        assert!(loaded.contains(&"good".to_string()));
        assert!(!loaded.contains(&"bad".to_string()));
    }

    #[tokio::test]
    async fn plugins_dir_falls_back_to_xdg() {
        std::env::remove_var("DYD_PLUGINS_DIR");
        let dir = PluginManager::plugins_dir();
        assert!(dir.ends_with("plugins"));
    }

    #[tokio::test]
    async fn auto_disable_after_repeated_crashes() {
        let manager = PluginManager::new(Arc::new(StubExecutor));
        let name = manager
            .register(test_manifest("alpha"), PathBuf::from("/tmp/alpha"), false)
            .await
            .expect("register ok");
        for _ in 0..4 {
            manager.report_crash(&name).await;
        }
        let h = manager.get(&name).await.unwrap();
        assert!(h.disabled, "plugin should be auto-disabled");
        assert_eq!(h.state, PluginState::Error);
    }
}

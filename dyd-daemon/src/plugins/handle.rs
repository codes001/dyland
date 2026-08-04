use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;

use dyd_types::PluginManifest;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginState {
    Loaded,
    Initialized,
    Running,
    Stopped,
    Unloaded,
    Error,
}

impl PluginState {
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginState::Loaded => "loaded",
            PluginState::Initialized => "initialized",
            PluginState::Running => "running",
            PluginState::Stopped => "stopped",
            PluginState::Unloaded => "unloaded",
            PluginState::Error => "error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: &'static str,
    pub message: String,
    pub timestamp: Instant,
}

pub const ERROR_BUFFER_SIZE: usize = 100;
const CRASH_LIMIT: u32 = 3;
const CRASH_WINDOW_SECS: u64 = 300;

#[derive(Debug, Clone)]
pub struct PluginHandle {
    pub name: String,
    pub manifest: PluginManifest,
    pub version: String,
    pub state: PluginState,
    pub is_core: bool,
    pub disabled: bool,
    pub directory: PathBuf,
    pub config: serde_json::Value,
    pub last_error: Option<String>,
    pub error_count: u32,
    pub loaded_at: Instant,
    pub crash_times: Vec<Instant>,
    pub log_buffer: VecDeque<LogEntry>,
    pub memory_usage: Option<u64>,
}

impl PluginHandle {
    pub fn new(manifest: PluginManifest, directory: PathBuf, is_core: bool) -> Self {
        let config = manifest.config_schema.clone().map(|_| {
            serde_json::json!({})
        }).unwrap_or_else(|| serde_json::json!({}));
        Self {
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            manifest,
            state: PluginState::Loaded,
            is_core,
            disabled: false,
            directory,
            config,
            last_error: None,
            error_count: 0,
            loaded_at: Instant::now(),
            crash_times: Vec::new(),
            log_buffer: VecDeque::with_capacity(ERROR_BUFFER_SIZE),
            memory_usage: None,
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        self.loaded_at.elapsed().as_secs()
    }

    pub fn log(&mut self, level: &'static str, message: impl Into<String>) {
        if self.log_buffer.len() >= ERROR_BUFFER_SIZE {
            self.log_buffer.pop_front();
        }
        self.log_buffer.push_back(LogEntry {
            level,
            message: message.into(),
            timestamp: Instant::now(),
        });
    }

    pub fn record_error(&mut self, message: impl Into<String>) {
        let msg = message.into();
        self.last_error = Some(msg.clone());
        self.error_count += 1;
        self.log("error", msg);
    }

    pub fn record_crash(&mut self, message: impl Into<String>) {
        let msg = message.into();
        self.last_error = Some(msg.clone());
        self.error_count += 1;
        self.crash_times.push(Instant::now());
        self.log("error", msg);
    }

    pub fn crashes_in_window(&self, window_secs: u64) -> u32 {
        let cutoff = Instant::now() - std::time::Duration::from_secs(window_secs);
        self.crash_times
            .iter()
            .filter(|t| **t >= cutoff)
            .count() as u32
    }

    pub fn should_auto_disable(&self) -> bool {
        self.crashes_in_window(CRASH_WINDOW_SECS) > CRASH_LIMIT
    }

    pub fn reset_errors(&mut self) {
        self.last_error = None;
        self.error_count = 0;
        self.crash_times.clear();
    }

    pub fn recent_logs(&self, limit: usize) -> Vec<LogEntry> {
        self.log_buffer.iter().rev().take(limit).cloned().collect()
    }
}

#[derive(Debug, Clone)]
pub struct HealthSnapshot {
    pub state: String,
    pub last_error: Option<String>,
    pub error_count: u32,
    pub uptime_secs: u64,
    pub memory_usage: Option<u64>,
    pub disabled: bool,
}

impl From<&PluginHandle> for HealthSnapshot {
    fn from(h: &PluginHandle) -> Self {
        Self {
            state: h.state.as_str().to_string(),
            last_error: h.last_error.clone(),
            error_count: h.error_count,
            uptime_secs: h.uptime_secs(),
            memory_usage: h.memory_usage,
            disabled: h.disabled,
        }
    }
}

pub fn health_payload(h: &PluginHandle) -> dyd_types::PluginHealthPayload {
    dyd_types::PluginHealthPayload {
        name: h.name.clone(),
        version: h.version.clone(),
        state: h.state.as_str().to_string(),
        last_error: h.last_error.clone(),
        error_count: h.error_count,
        uptime_secs: h.uptime_secs(),
        memory_usage: h.memory_usage,
        disabled: h.disabled,
        is_core: h.is_core,
        permissions: h.manifest.permissions.clone(),
        ui_components: h.manifest.ui_components.clone(),
        config_schema: h.manifest.config_schema.clone(),
        config: Some(h.config.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn handle_state_transitions_tracked() {
        let mut h = PluginHandle::new(test_manifest("demo"), PathBuf::from("/tmp/demo"), false);
        assert_eq!(h.state, PluginState::Loaded);
        h.state = PluginState::Initialized;
        assert_eq!(h.state.as_str(), "initialized");
        h.state = PluginState::Running;
        assert_eq!(h.state.as_str(), "running");
        h.state = PluginState::Stopped;
        h.state = PluginState::Unloaded;
        assert_eq!(h.state.as_str(), "unloaded");
    }

    #[test]
    fn log_buffer_keeps_last_100() {
        let mut h = PluginHandle::new(test_manifest("demo"), PathBuf::from("/tmp/demo"), false);
        for i in 0..150 {
            h.log("info", format!("entry {i}"));
        }
        assert_eq!(h.log_buffer.len(), ERROR_BUFFER_SIZE);
        let logs = h.recent_logs(200);
        assert_eq!(logs.len(), ERROR_BUFFER_SIZE);
        assert_eq!(logs[0].message, "entry 149");
    }

    #[test]
    fn crash_auto_disable_after_threshold() {
        let mut h = PluginHandle::new(test_manifest("demo"), PathBuf::from("/tmp/demo"), false);
        for _ in 0..4 {
            h.record_crash("boom");
        }
        assert_eq!(h.error_count, 4);
        assert!(h.should_auto_disable());
    }

    #[test]
    fn crashes_before_threshold_do_not_disable() {
        let mut h = PluginHandle::new(test_manifest("demo"), PathBuf::from("/tmp/demo"), false);
        for _ in 0..3 {
            h.record_crash("boom");
        }
        assert!(!h.should_auto_disable());
    }

    #[test]
    fn reset_errors_clears_crash_history() {
        let mut h = PluginHandle::new(test_manifest("demo"), PathBuf::from("/tmp/demo"), false);
        for _ in 0..5 {
            h.record_crash("boom");
        }
        assert!(h.should_auto_disable());
        h.reset_errors();
        assert_eq!(h.error_count, 0);
        assert!(!h.should_auto_disable());
        assert!(h.last_error.is_none());
    }
}

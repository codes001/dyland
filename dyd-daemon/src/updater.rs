//! Automatic background update checker (Epic 9, Feature 9.4).
//!
//! Periodically checks GitHub Releases API for newer semver tags and pushes an
//! `UpdateAvailable` notification into the daemon's activity queue.

use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::queue::ActivityQueue;

const CHECK_INTERVAL: Duration = Duration::from_secs(86400); // 24 hours
const GITHUB_API_URL: &str = "https://api.github.com/repos/dyland-org/dyland/releases/latest";

#[derive(Debug, Clone, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    #[allow(dead_code)]
    body: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub url: String,
}

pub struct UpdateChecker {
    current_version: String,
    queue: Arc<Mutex<ActivityQueue>>,
}

impl UpdateChecker {
    pub fn new(current_version: &str, queue: Arc<Mutex<ActivityQueue>>) -> Self {
        Self {
            current_version: current_version.to_string(),
            queue,
        }
    }

    pub fn start_periodic_check(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(CHECK_INTERVAL);
            loop {
                interval.tick().await;
                if let Some(update) = self.check_once().await {
                    info!(version = %update.version, url = %update.url, "new update available");
                    let activity = crate::queue::activity::Activity::new(
                        "updater".to_string(),
                        crate::queue::activity::Priority::Medium,
                        serde_json::json!({
                            "title": format!("dyland v{} Available", update.version),
                            "subtitle": "Click to view release details & install update",
                            "version": update.version,
                            "url": update.url,
                        }),
                    );
                    self.queue.lock().await.push(activity);
                }
            }
        });
    }

    pub async fn check_once(&self) -> Option<UpdateInfo> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent("dyland-daemon-updater/0.1.0")
            .build()
            .ok()?;

        let resp = match client.get(GITHUB_API_URL).send().await {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, "update check HTTP request failed (offline or network error)");
                return None;
            }
        };

        if !resp.status().is_success() {
            return None;
        }

        let release: GithubRelease = resp.json().await.ok()?;
        let latest_tag = release.tag_name.trim_start_matches('v');

        if is_newer_semver(&self.current_version, latest_tag) {
            Some(UpdateInfo {
                version: latest_tag.to_string(),
                url: release.html_url,
            })
        } else {
            None
        }
    }
}

fn is_newer_semver(current: &str, latest: &str) -> bool {
    let parse = |s: &str| -> Option<(u32, u32, u32)> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() < 3 {
            return None;
        }
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = parts[2].split('-').next()?.parse().ok()?;
        Some((major, minor, patch))
    };

    match (parse(current), parse(latest)) {
        (Some(c), Some(l)) => l > c,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_comparison() {
        assert!(is_newer_semver("0.1.0", "0.2.0"));
        assert!(is_newer_semver("0.1.0", "1.0.0"));
        assert!(is_newer_semver("0.1.0", "0.1.1"));
        assert!(!is_newer_semver("0.1.0", "0.1.0"));
        assert!(!is_newer_semver("0.2.0", "0.1.0"));
    }
}

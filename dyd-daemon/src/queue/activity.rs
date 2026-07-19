use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::time::Instant;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl Priority {
    pub fn rank(&self) -> u8 {
        match self {
            Priority::Critical => 5,
            Priority::High => 4,
            Priority::Medium => 3,
            Priority::Low => 2,
            Priority::Info => 1,
        }
    }
}

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Priority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank().cmp(&other.rank())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: Uuid,
    pub source: String,
    #[serde(skip)]
    pub priority: Priority,
    #[serde(skip)]
    pub timestamp: Instant,
    pub content: serde_json::Value,
    pub ttl_secs: Option<u64>,
}

impl Activity {
    pub fn new(source: String, priority: Priority, content: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            source,
            priority,
            timestamp: Instant::now(),
            content,
            ttl_secs: None,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.ttl_secs
            .map(|ttl| self.timestamp.elapsed().as_secs() > ttl)
            .unwrap_or(false)
    }
}

impl Eq for Activity {}

impl PartialEq for Activity {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for Activity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Activity {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| self.timestamp.cmp(&other.timestamp))
    }
}

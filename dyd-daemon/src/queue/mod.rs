pub mod activity;
pub mod priority;

use activity::{Activity, Priority};
use std::collections::BinaryHeap;
use std::path::PathBuf;
use uuid::Uuid;

const MAX_CAPACITY: usize = 20;

#[derive(Debug)]
pub struct ActivityQueue {
    heap: BinaryHeap<Activity>,
}

impl ActivityQueue {
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
        }
    }

    pub fn push(&mut self, activity: Activity) {
        if self.heap.len() >= MAX_CAPACITY {
            if let Some(lowest) = self.heap.iter().min_by_key(|a| (a.priority.rank(), a.timestamp))
            {
                if lowest.priority.rank() <= activity.priority.rank() {
                    let lowest_id = lowest.id;
                    self.remove_by_id(lowest_id);
                    tracing::warn!(id = %lowest_id, "queue full, evicted lowest priority activity");
                } else {
                    tracing::warn!(id = %activity.id, "queue full, dropped new activity");
                    return;
                }
            }
        }

        if self.heap.iter().any(|a| a.id == activity.id) {
            tracing::warn!(id = %activity.id, "duplicate activity id, ignoring");
            return;
        }

        self.heap.push(activity);
    }

    pub fn pop(&mut self) -> Option<Activity> {
        self.heap.pop()
    }

    pub fn peek(&self) -> Option<&Activity> {
        self.heap.peek()
    }

    pub fn remove_by_id(&mut self, id: Uuid) -> Option<Activity> {
        let mut removed = None;
        self.heap = self.heap.drain().filter(|a| {
            if a.id == id {
                removed = Some(a.clone());
                false
            } else {
                true
            }
        }).collect();
        removed
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Activity> {
        self.heap.iter()
    }

    pub fn remove_expired(&mut self) {
        self.heap = self.heap.drain().filter(|a| !a.is_expired()).collect();
    }

    pub fn clear(&mut self) {
        self.heap.clear();
    }
}

impl Default for ActivityQueue {
    fn default() -> Self {
        Self::new()
    }
}

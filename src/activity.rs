//! Bounded, process-local audit history shared by HTTP REST and MCP transports.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use serde::Serialize;

const DEFAULT_CAPACITY: usize = 100;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ActivityEvent {
    pub sequence: u64,
    pub timestamp: String,
    pub transport: String,
    pub action: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct ActivityLog {
    state: Arc<Mutex<ActivityState>>,
    capacity: usize,
}

struct ActivityState {
    events: VecDeque<ActivityEvent>,
    next_sequence: u64,
}

impl Default for ActivityLog {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }
}

impl ActivityLog {
    pub fn new(capacity: usize) -> Self {
        Self {
            state: Arc::new(Mutex::new(ActivityState {
                events: VecDeque::with_capacity(capacity),
                next_sequence: 1,
            })),
            capacity: capacity.max(1),
        }
    }

    pub fn record(&self, transport: &str, action: &str, ok: bool, error: Option<&str>) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let event = ActivityEvent {
            sequence: state.next_sequence,
            timestamp: chrono::Utc::now().to_rfc3339(),
            transport: transport.to_owned(),
            action: action.to_owned(),
            ok,
            error: error.map(sanitize_error_category),
        };
        state.next_sequence = state.next_sequence.wrapping_add(1);
        while state.events.len() >= self.capacity {
            state.events.pop_front();
        }
        state.events.push_back(event);
    }

    pub fn snapshot(&self) -> Vec<ActivityEvent> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .events
            .iter()
            .rev()
            .cloned()
            .collect()
    }
}

fn sanitize_error_category(error: &str) -> String {
    let normalized = error.to_ascii_lowercase();
    if normalized.contains("confirm") || normalized.contains("declin") {
        "confirmation_denied".into()
    } else if normalized.contains("forbidden")
        || normalized.contains("scope")
        || normalized.contains("auth")
    {
        "forbidden".into()
    } else if normalized.contains("invalid")
        || normalized.contains("unknown")
        || normalized.contains("required")
    {
        "invalid_request".into()
    } else {
        "execution_failed".into()
    }
}

#[cfg(test)]
#[path = "activity_tests.rs"]
mod tests;

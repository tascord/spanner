use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use tracing::Level;

use crate::event::Event;

/// Type for event callback functions
pub type EventCallback = Arc<dyn Fn(&Event) + Send + Sync>;

/// Global event manager to capture all tracing events
static GLOBAL_EVENT_MANAGER: Mutex<Option<EventManager>> = Mutex::new(None);
/// Global list of event callbacks
static GLOBAL_EVENT_CALLBACKS: Mutex<Vec<EventCallback>> = Mutex::new(Vec::new());

#[derive(Default)]
pub struct EventManager(VecDeque<Event>);

impl EventManager {
    pub fn new() -> Self {
        Self(VecDeque::new())
    }

    pub fn push(&mut self, event: Event) {
        self.0.push_front(event);
        if self.0.len() > 12_000 {
            let _ = self.0.pop_back();
        }
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get events by level
    pub fn get_by_level(&self, level: Level) -> Vec<&Event> {
        self.0
            .iter()
            .filter(|event| event.event_data.level == level)
            .collect()
    }

    /// Get events by target (module/crate)
    pub fn get_by_target(&self, target: &str) -> Vec<&Event> {
        self.0
            .iter()
            .filter(|event| event.event_data.target.contains(target))
            .collect()
    }

    /// Get events within a specific span
    pub fn get_by_span(&self, span_name: &str) -> Vec<&Event> {
        self.0
            .iter()
            .filter(|event| {
                event
                    .span_stack
                    .iter()
                    .chain(event.current_span.iter())
                    .any(|span| span.name.contains(span_name))
            })
            .collect()
    }

    /// Get events by thread
    pub fn get_by_thread(&self, thread_id: &str) -> Vec<&Event> {
        self.0
            .iter()
            .filter(|event| {
                event
                    .thread_id
                    .as_ref()
                    .is_some_and(|id| id == thread_id)
            })
            .collect()
    }

    /// Get events with specific correlation ID
    pub fn get_by_correlation_id(&self, correlation_id: &str) -> Vec<&Event> {
        self.0
            .iter()
            .filter(|event| {
                event
                    .correlation_id
                    .as_ref()
                    .is_some_and(|id| id == correlation_id)
            })
            .collect()
    }

    /// Advanced search with multiple criteria
    pub fn search(
        &self,
        level_filter: Option<Level>,
        target_filter: Option<&str>,
        message_contains: Option<&str>,
        span_name_contains: Option<&str>,
    ) -> Vec<&Event> {
        self.0
            .iter()
            .filter(|event| {
                event.matches_criteria(
                    level_filter,
                    target_filter,
                    message_contains,
                    span_name_contains,
                )
            })
            .collect()
    }

    /// Get the most recent N events
    pub fn get_recent(&self, count: usize) -> Vec<&Event> {
        self.0.iter().take(count).collect()
    }
}

/// Initialize the global event manager
pub fn init_global_event_manager() {
    let mut global = GLOBAL_EVENT_MANAGER.lock().unwrap();
    *global = Some(EventManager::new());
}

/// Add an event to the global manager
pub fn add_global_event(event: Event) {
    // Notify all registered callbacks first
    if let Ok(callbacks) = GLOBAL_EVENT_CALLBACKS.lock() {
        for callback in callbacks.iter() {
            callback(&event);
        }
    }

    // Then add to the manager
    if let Ok(mut global) = GLOBAL_EVENT_MANAGER.lock()
        && let Some(ref mut manager) = *global
    {
        manager.push(event);
    }
}

/// Register a callback to be called when new events are added
pub fn register_event_callback(callback: EventCallback) {
    if let Ok(mut callbacks) = GLOBAL_EVENT_CALLBACKS.lock() {
        callbacks.push(callback);
    }
}

/// Clear all registered callbacks
pub fn clear_event_callbacks() {
    if let Ok(mut callbacks) = GLOBAL_EVENT_CALLBACKS.lock() {
        callbacks.clear();
    }
}

/// Get a copy of all events from the global manager
pub fn get_global_events() -> Vec<Event> {
    if let Ok(global) = GLOBAL_EVENT_MANAGER.lock()
        && let Some(ref manager) = *global
    {
        return manager.0.iter().cloned().collect();
    }
    Vec::new()
}

/// Get the number of events in the global manager
pub fn get_global_event_count() -> usize {
    if let Ok(global) = GLOBAL_EVENT_MANAGER.lock()
        && let Some(ref manager) = *global
    {
        return manager.len();
    }
    0
}

/// Clear all events from the global manager
pub fn clear_global_events() {
    if let Ok(mut global) = GLOBAL_EVENT_MANAGER.lock()
        && let Some(ref mut manager) = *global
    {
        manager.0.clear();
    }
}

/// Get debug information about captured events
pub fn get_event_summary() -> String {
    if let Ok(global) = GLOBAL_EVENT_MANAGER.lock()
        && let Some(ref manager) = *global
    {
        let total = manager.len();
        let by_level = [
            (Level::ERROR, manager.get_by_level(Level::ERROR).len()),
            (Level::WARN, manager.get_by_level(Level::WARN).len()),
            (Level::INFO, manager.get_by_level(Level::INFO).len()),
            (Level::DEBUG, manager.get_by_level(Level::DEBUG).len()),
            (Level::TRACE, manager.get_by_level(Level::TRACE).len()),
        ];

        let mut summary = format!("Event Summary: {} total events\n", total);
        for (level, count) in by_level {
            if count > 0 {
                summary.push_str(&format!("  {}: {}\n", level, count));
            }
        }

        return summary;
    }
    "No events captured".to_string()
}
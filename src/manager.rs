use {
    crate::{event::Event, events::EventTarget},
    chrono::{DateTime, Utc},
    serde::{Deserialize, Serialize},
    std::{
        collections::{BTreeMap, VecDeque},
        fs::File,
        io::{self, Write},
        ops::Deref,
        path::Path,
        sync::{Arc, OnceLock, RwLock},
    },
    tracing::Level,
};

static GLOBAL_EVENT_MANAGER: OnceLock<Arc<RwLock<EventManager>>> = OnceLock::new();

#[derive(Default)]
pub struct EventManager {
    inner: VecDeque<Event>,
    target: EventTarget<Event>,
    max_events: usize,
}

impl Deref for EventManager {
    type Target = EventTarget<Event>;

    fn deref(&self) -> &Self::Target { &self.target }
}

impl EventManager {
    pub fn new(max_events: Option<usize>) -> Self {
        Self { inner: Default::default(), target: Default::default(), max_events: max_events.unwrap_or(12_000) }
    }

    pub fn push(&mut self, event: Event) {
        self.inner.push_front(event);
        if self.inner.len() > self.max_events {
            let _ = self.inner.pop_back();
        }
    }

    pub fn len(&self) -> usize { self.inner.len() }

    pub fn is_empty(&self) -> bool { self.inner.is_empty() }

    /// Get events by level
    pub fn get_by_level(&self, level: Level) -> Vec<&Event> {
        self.inner.iter().filter(|event| event.event_data.level == level).collect()
    }

    /// Get events by target (module/crate)
    pub fn get_by_target(&self, target: &str) -> Vec<&Event> {
        self.inner.iter().filter(|event| event.event_data.target.contains(target)).collect()
    }

    /// Get events within a specific span
    pub fn get_by_span(&self, span_name: &str) -> Vec<&Event> {
        self.inner
            .iter()
            .filter(|event| {
                event.span_stack.iter().chain(event.current_span.iter()).any(|span| span.name.contains(span_name))
            })
            .collect()
    }

    /// Get events by thread
    pub fn get_by_thread(&self, thread_id: &str) -> Vec<&Event> {
        self.inner.iter().filter(|event| event.thread_id.as_ref().is_some_and(|id| id == thread_id)).collect()
    }

    /// Get events with specific correlation ID
    pub fn get_by_correlation_id(&self, correlation_id: &str) -> Vec<&Event> {
        self.inner.iter().filter(|event| event.correlation_id.as_ref().is_some_and(|id| id == correlation_id)).collect()
    }

    /// Advanced search with multiple criteria
    pub fn search(
        &self,
        level_filter: Option<Level>,
        target_filter: Option<&str>,
        message_contains: Option<&str>,
        span_name_contains: Option<&str>,
    ) -> Vec<&Event> {
        self.inner
            .iter()
            .filter(|event| event.matches_criteria(level_filter, target_filter, message_contains, span_name_contains))
            .collect()
    }

    /// Get the most recent N events
    pub fn get_recent(&self, count: usize) -> Vec<&Event> { self.inner.iter().take(count).collect() }
}

/// Initialize the global event manager
pub fn init_global_event_manager() { let _ = GLOBAL_EVENT_MANAGER.set(Arc::new(RwLock::new(EventManager::new(None)))); }

/// Initialize the global event manager with max event count
pub fn init_global_event_manager_with_count(max_events: usize) {
    let _ = GLOBAL_EVENT_MANAGER.set(Arc::new(RwLock::new(EventManager::new(Some(max_events)))));
}

/// Get a copy of all events from the global manager
pub fn get_global_events() -> Option<Vec<Event>> { Some(GLOBAL_EVENT_MANAGER.get()?.read().ok()?.inner.clone().into()) }

/// Get the number of events in the global manager
pub fn get_global_event_count() -> usize {
    GLOBAL_EVENT_MANAGER.get().and_then(|v| v.read().map(|v| v.inner.len()).ok()).unwrap_or(0)
}

pub(crate) fn emit(event: Event) -> Option<()> {
    GLOBAL_EVENT_MANAGER.get()?.read().ok()?.emit(event);
    Some(())
}

/// Get access to the global event target for emitting events
pub fn events() -> Option<EventTarget<Event>> { Some(GLOBAL_EVENT_MANAGER.get()?.read().ok()?.target.clone()) }

/// Clear all events from the global manager
pub fn clear_global_events() {
    if let Some(mut global) = GLOBAL_EVENT_MANAGER.get().and_then(|v| v.write().ok()) {
        global.inner.clear();
    }
}

/// Export format for binary files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMetadata {
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub total_events: usize,
    pub level_counts: BTreeMap<String, usize>,
    pub description: Option<String>,
}

/// Container for exported data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub metadata: ExportMetadata,
    pub events: Vec<Event>,
}

/// Export all events to a binary file
pub fn export_to_bin_file<P: AsRef<Path>>(path: P) -> io::Result<usize> {
    let events = get_global_events().unwrap_or_default();
    let export_data = create_export_data(events, None);

    let encoded = serde_json::to_vec(&export_data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let mut file = File::create(path)?;
    file.write_all(&encoded)?;
    file.flush()?;

    Ok(export_data.events.len())
}

/// Export events with filtering to a binary file
pub fn export_filtered_to_bin_file<P: AsRef<Path>>(
    path: P,
    level_filter: Option<Level>,
    target_filter: Option<&str>,
    message_contains: Option<&str>,
    span_name_contains: Option<&str>,
    description: Option<String>,
) -> io::Result<usize> {
    let all_events = get_global_events().unwrap_or_default();
    let filtered_events: Vec<Event> = all_events
        .into_iter()
        .filter(|event| event.matches_criteria(level_filter, target_filter, message_contains, span_name_contains))
        .collect();

    let export_data = create_export_data(filtered_events, description);

    let encoded = serde_json::to_vec(&export_data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let mut file = File::create(path)?;
    file.write_all(&encoded)?;
    file.flush()?;

    Ok(export_data.events.len())
}

/// Get binary data for export without writing to file
pub fn export_to_bin_data() -> Result<Vec<u8>, serde_json::Error> {
    let events = get_global_events().unwrap_or_default();
    let export_data = create_export_data(events, None);
    serde_json::to_vec(&export_data)
}

/// Import events from a binary file and return a new EventManager
pub fn import_from_bin_file<P: AsRef<Path>>(path: P) -> io::Result<EventManager> {
    let data = std::fs::read(path)?;
    let export_data: ExportData =
        serde_json::from_slice(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // Create a new EventManager with the imported events
    let mut manager = EventManager::new(None);
    for event in export_data.events {
        manager.push(event);
    }

    Ok(manager)
}

/// Import events from a binary file and add to global manager
pub fn import_and_merge_from_bin_file<P: AsRef<Path>>(path: P) -> io::Result<(ExportData, usize)> {
    let data = std::fs::read(path)?;
    let export_data: ExportData =
        serde_json::from_slice(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // Add imported events to the global manager
    if let Some(global) = GLOBAL_EVENT_MANAGER.get().and_then(|v| v.write().ok()) {
        let mut manager = global;
        for event in &export_data.events {
            manager.push(event.clone());
        }
    }

    let imported_count = export_data.events.len();
    Ok((export_data, imported_count))
}

/// Create export data structure with metadata
fn create_export_data(events: Vec<Event>, description: Option<String>) -> ExportData {
    let total_events = events.len();
    let mut level_counts = BTreeMap::new();

    for event in &events {
        let level_str = format!("{}", event.event_data.level);
        *level_counts.entry(level_str).or_insert(0) += 1;
    }

    let metadata = ExportMetadata {
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now(),
        total_events,
        level_counts,
        description,
    };

    ExportData { metadata, events }
}

/// Get summary of events without exporting
pub fn get_event_summary() -> String {
    if let Some(global) = GLOBAL_EVENT_MANAGER.get().and_then(|v| v.read().ok()) {
        let total = global.len();
        let by_level = [
            (Level::ERROR, global.get_by_level(Level::ERROR).len()),
            (Level::WARN, global.get_by_level(Level::WARN).len()),
            (Level::INFO, global.get_by_level(Level::INFO).len()),
            (Level::DEBUG, global.get_by_level(Level::DEBUG).len()),
            (Level::TRACE, global.get_by_level(Level::TRACE).len()),
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

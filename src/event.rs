use {
    crate::{event_data::EventData, span::SpanInfo},
    serde::{Deserialize, Serialize},
    std::{
        collections::HashMap,
        sync::Arc,
        time::{SystemTime, UNIX_EPOCH},
    },
    tracing::Level,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    #[serde(skip)]
    pub parent: Option<Arc<Event>>,
    pub event_data: EventData,
    pub span_stack: Vec<SpanInfo>,
    pub current_span: Option<SpanInfo>,
    pub thread_id: Option<String>,
    pub thread_name: Option<String>,
    pub process_id: Option<u32>,
    pub correlation_id: Option<String>,
    pub custom_metadata: HashMap<String, String>,
}

impl Event {
    pub fn new(event_data: EventData) -> Self {
        Self {
            parent: None,
            event_data,
            span_stack: Vec::new(),
            current_span: None,
            thread_id: None,
            thread_name: None,
            process_id: None,
            correlation_id: None,
            custom_metadata: HashMap::new(),
        }
    }

    pub fn with_parent(mut self, parent: Arc<Event>) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn with_span_stack(mut self, spans: Vec<SpanInfo>) -> Self {
        self.span_stack = spans;
        self
    }

    pub fn with_current_span(mut self, span: SpanInfo) -> Self {
        self.current_span = Some(span);
        self
    }

    pub fn with_thread_info(mut self, thread_id: String, thread_name: Option<String>) -> Self {
        self.thread_id = Some(thread_id);
        self.thread_name = thread_name;
        self
    }

    pub fn with_process_id(mut self, pid: u32) -> Self {
        self.process_id = Some(pid);
        self
    }

    pub fn with_correlation_id(mut self, correlation_id: String) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    pub fn add_metadata(&mut self, key: String, value: String) { self.custom_metadata.insert(key, value); }

    /// Get the full span hierarchy as a formatted tree string
    pub fn get_span_tree(&self) -> String {
        let mut tree = String::new();

        if let Some(ref current) = self.current_span {
            tree.push_str(&format!("Current Span: {} ({})\n", current.name, current.level()));
        }

        if !self.span_stack.is_empty() {
            tree.push_str("Span Stack:\n");
            for (depth, span) in self.span_stack.iter().enumerate() {
                let indent = "  ".repeat(depth);
                let duration_str =
                    span.get_duration().map(|d| format!(" [{:.2?}]", d)).unwrap_or_else(|| " [active]".to_string());

                tree.push_str(&format!("{}├─ {} ({}){}", indent, span.name, span.level(), duration_str));

                if !span.fields.is_empty() {
                    tree.push_str(" {");
                    for (k, v) in &span.fields {
                        tree.push_str(&format!(" {}={}", k, v));
                    }
                    tree.push_str(" }");
                }
                tree.push('\n');

                // Add children recursively
                for child in &span.children {
                    Self::format_span_child(child, depth + 1, &mut tree);
                }
            }
        }

        tree
    }

    fn format_span_child(span: &SpanInfo, depth: usize, tree: &mut String) {
        let indent = "  ".repeat(depth);
        let duration_str = span.get_duration().map(|d| format!(" [{:.2?}]", d)).unwrap_or_else(|| " [active]".to_string());

        tree.push_str(&format!("{}├─ {} ({}){}", indent, span.name, span.level(), duration_str));

        if !span.fields.is_empty() {
            tree.push_str(" {");
            for (k, v) in &span.fields {
                tree.push_str(&format!(" {}={}", k, v));
            }
            tree.push_str(" }");
        }
        tree.push('\n');

        for child in &span.children {
            Self::format_span_child(child, depth + 1, tree);
        }
    }

    /// Get all context information as a formatted string for debugging
    pub fn get_full_context(&self) -> String {
        let mut context = String::new();

        context.push_str(&format!("Event: {} ({})\n", self.event_data.message, self.event_data.level()));
        context.push_str(&format!("Target: {}\n", self.event_data.target));
        context.push_str(&format!("Timestamp: {:?}\n", self.event_data.timestamp));

        if let Some(ref file) = self.event_data.file {
            context.push_str(&format!("Location: {}:{}\n", file, self.event_data.line.unwrap_or(0)));
        }

        if let Some(ref thread_id) = self.thread_id {
            context.push_str(&format!("Thread: {}", thread_id));
            if let Some(ref name) = self.thread_name {
                context.push_str(&format!(" ({})", name));
            }
            context.push('\n');
        }

        if let Some(pid) = self.process_id {
            context.push_str(&format!("Process ID: {}\n", pid));
        }

        if let Some(ref correlation_id) = self.correlation_id {
            context.push_str(&format!("Correlation ID: {}\n", correlation_id));
        }

        if !self.event_data.fields.is_empty() {
            context.push_str("Event Fields:\n");
            for (k, v) in &self.event_data.fields {
                context.push_str(&format!("  {}: {}\n", k, v));
            }
        }

        if !self.custom_metadata.is_empty() {
            context.push_str("Metadata:\n");
            for (k, v) in &self.custom_metadata {
                context.push_str(&format!("  {}: {}\n", k, v));
            }
        }

        context.push('\n');
        context.push_str(&self.get_span_tree());

        if let Some(ref parent) = self.parent {
            context.push_str("\n--- Parent Event ---\n");
            context.push_str(&parent.get_full_context());
        }

        context
    }

    /// Search for events by various criteria
    pub fn matches_criteria(
        &self,
        level_filter: Option<Level>,
        target_filter: Option<&str>,
        message_contains: Option<&str>,
        span_name_contains: Option<&str>,
    ) -> bool {
        if let Some(level) = level_filter
            && self.event_data.level() != level
        {
            return false;
        }

        if let Some(target) = target_filter
            && !self.event_data.target.contains(target)
        {
            return false;
        }

        if let Some(message) = message_contains
            && !self.event_data.message.contains(message)
        {
            return false;
        }

        if let Some(span_name) = span_name_contains {
            let has_matching_span =
                self.span_stack.iter().chain(self.current_span.iter()).any(|span| span.name.contains(span_name));
            if !has_matching_span {
                return false;
            }
        }

        true
    }

    /// Create an Event from tracing subscriber data
    pub fn from_tracing_event(
        message: String,
        level: Level,
        target: String,
        metadata: Option<(String, u32, String)>, // (file, line, module_path)
        fields: HashMap<String, String>,
    ) -> Self {
        let mut event_data = EventData::new(message, level, target);
        event_data.fields = fields;

        if let Some((file, line, module_path)) = metadata {
            event_data.file = Some(file);
            event_data.line = Some(line);
            event_data.module_path = Some(module_path);
        }

        Event::new(event_data)
    }

    /// Create an Event with full context from current tracing state
    pub fn capture_current_context(message: String, level: Level, target: String) -> Self {
        // In a real implementation, you would extract this from the tracing subscriber
        // This is a template showing what information should be captured

        let mut event = Event::from_tracing_event(message, level, target, None, HashMap::new());

        // Add thread information
        event = event
            .with_thread_info(format!("{:?}", std::thread::current().id()), std::thread::current().name().map(String::from));

        // Add process ID
        event = event.with_process_id(std::process::id());

        // Add correlation ID (could be from context or generated)
        event = event.with_correlation_id(format!("corr-{}", generate_uuid_like_string()));

        event
    }
}

/// Helper function to generate a simple UUID-like string
fn generate_uuid_like_string() -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{:x}-{:x}", now.as_secs(), now.subsec_nanos())
}

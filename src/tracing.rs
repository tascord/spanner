use {
    crate::{
        event::Event,
        event_data::EventData,
        manager::{emit, init_global_event_manager},
    },
    std::collections::HashMap,
    tracing::Subscriber,
    tracing_subscriber::{Layer, layer::Context, Registry, prelude::*},
};

/// Initialize tracing with Spanner layer only (use with existing subscriber)
pub fn init_layer_only() -> Result<(), Box<dyn std::error::Error>> {
    init_global_event_manager();
    Ok(())
}

/// Add Spanner layer to an existing subscriber
pub fn add_to_subscriber<S>(subscriber: S) -> impl Subscriber + Send + Sync
where
    S: Subscriber + Send + Sync + 'static,
{
    init_global_event_manager();
    subscriber.with(SpannerLayer)
}

/// Initialize with custom subscriber
pub fn init_with_subscriber<S>(subscriber: S) -> Result<(), Box<dyn std::error::Error>>
where
    S: Subscriber + Send + Sync + 'static,
{
    init_global_event_manager();
    let subscriber_with_spanner = subscriber.with(SpannerLayer);
    tracing::subscriber::set_global_default(subscriber_with_spanner)?;
    tracing::info!("Spanner initialized with custom subscriber");
    Ok(())
}

/// Initialize the complete tracing system with event capture
/// This sets up both the global event manager and the tracing subscriber
pub fn init_tracing_capture() -> Result<(), Box<dyn std::error::Error>> {
    use tracing_subscriber::prelude::*;

    // Initialize global event manager
    init_global_event_manager();

    // Set up tracing subscriber with our custom layer
    let subscriber = Registry::default()
        .with(SpannerLayer)
        .with(tracing_subscriber::fmt::layer());

    tracing::subscriber::set_global_default(subscriber)?;
    tracing::info!("Spanner tracing capture initialized");
    Ok(())
}

/// Custom tracing layer that captures events and spans
pub struct SpannerLayer;

impl<S> Layer<S> for SpannerLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut fields = HashMap::new();
        let mut message = String::new();

        // Capture event fields using a visitor
        struct FieldVisitor<'a> {
            fields: &'a mut HashMap<String, String>,
            message: &'a mut String,
        }

        impl<'a> tracing::field::Visit for FieldVisitor<'a> {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                let value_str = format!("{:?}", value);
                if field.name() == "message" {
                    *self.message = value_str.trim_matches('"').to_string();
                } else {
                    self.fields.insert(field.name().to_string(), value_str);
                }
            }
        }

        let mut visitor = FieldVisitor { fields: &mut fields, message: &mut message };

        event.record(&mut visitor);

        // Create event data
        let metadata = event.metadata();
        let mut event_data = EventData::new(message, *metadata.level(), metadata.target().to_string());

        event_data.fields = fields;
        event_data.file = metadata.file().map(String::from);
        event_data.line = metadata.line();
        event_data.module_path = metadata.module_path().map(String::from);

        // Create the event with thread context
        let captured_event = Event::new(event_data)
            .with_thread_info(format!("{:?}", std::thread::current().id()), std::thread::current().name().map(String::from))
            .with_process_id(std::process::id())
            .with_correlation_id(format!("corr-{}", generate_uuid_like_string()));

        emit(captured_event);
    }

    fn on_new_span(&self, _attrs: &tracing::span::Attributes<'_>, _id: &tracing::span::Id, _ctx: Context<'_, S>) {
        // Could implement span tracking here for even richer context
    }

    fn on_enter(&self, _id: &tracing::span::Id, _ctx: Context<'_, S>) {
        // Could track span entry
    }

    fn on_exit(&self, _id: &tracing::span::Id, _ctx: Context<'_, S>) {
        // Could track span exit
    }
}

/// Helper function to generate a simple UUID-like string
fn generate_uuid_like_string() -> String {
    use chrono::Utc;
    let now = Utc::now();
    format!("{:x}-{:x}", now.timestamp(), now.timestamp_subsec_nanos())
}

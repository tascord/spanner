use {
    crate::{
        event::Event,
        event_data::EventData,
        manager::{emit, init_global_event_manager},
    },
    std::collections::HashMap,
    tracing::Subscriber,
    tracing_subscriber::{Layer, layer::Context},
};

/// Initialize the complete tracing system with event capture
/// This sets up both the global event manager and the tracing subscriber
pub fn init_tracing_capture() -> Result<(), Box<dyn std::error::Error>> {
    use tracing_subscriber::{Registry, prelude::*};

    // Initialize global event manager
    init_global_event_manager();

    // Set up tracing subscriber with our custom layer
    let subscriber = Registry::default().with(SpannerLayer).with(tracing_subscriber::fmt::layer()); // Also include formatted output

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
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{:x}-{:x}", now.as_secs(), now.subsec_nanos())
}

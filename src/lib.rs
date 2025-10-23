// Modular structure for better code organization
mod event;
mod event_data;
mod span;
mod manager;
mod tracing;

// Keep the existing async event system
pub mod events;

// Re-export main types and functions for public API
pub use event::Event;
pub use event_data::EventData;
pub use span::SpanInfo;
pub use manager::{
    EventCallback, EventManager,
    init_global_event_manager, add_global_event, register_event_callback,
    clear_event_callbacks, get_global_events, get_global_event_count,
    clear_global_events, get_event_summary,
};
pub use tracing::{init_tracing_capture, SpannerLayer};

/// Main initialization function - sets up the complete tracing system
pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    tracing::init_tracing_capture()
}

/// Example usage and test functions
#[cfg(test)]
mod tests {
    use {super::*, ::tracing::Level};

    #[test]
    fn test_tracing_integration() {
        use tracing_subscriber::{Registry, prelude::*};

        // Initialize global event manager
        init_global_event_manager();
        clear_global_events();

        // Set up tracing subscriber with our custom layer
        let subscriber = Registry::default().with(SpannerLayer);

        // Set the subscriber for this test
        ::tracing::subscriber::with_default(subscriber, || {
            // Generate some tracing events
            ::tracing::info!("This is an info message");
            ::tracing::warn!(field1 = "value1", field2 = 42, "Warning with fields");
            ::tracing::error!("An error occurred");

            // Add a small delay to ensure events are processed
            std::thread::sleep(std::time::Duration::from_millis(10));
        });

        // Check that events were captured
        let events = get_global_events();
        assert!(events.len() >= 3, "Expected at least 3 events, got {}", events.len());

        // Check that we captured the info message
        let info_events: Vec<_> = events.iter().filter(|e| e.event_data.message.contains("info message")).collect();
        assert_eq!(info_events.len(), 1, "Should have captured the info message");

        // Check that we captured the warning with fields
        let warn_events: Vec<_> = events.iter().filter(|e| e.event_data.message.contains("Warning with fields")).collect();
        assert_eq!(warn_events.len(), 1, "Should have captured the warning");

        // Verify fields were captured
        let warn_event = warn_events[0];
        assert!(warn_event.event_data.fields.contains_key("field1"));
        assert!(warn_event.event_data.fields.contains_key("field2"));

        // Check that we captured the error
        let error_events: Vec<_> = events.iter().filter(|e| e.event_data.level == Level::ERROR).collect();
        assert_eq!(error_events.len(), 1, "Should have captured the error");

        // Verify thread information was captured
        for event in &events {
            assert!(event.thread_id.is_some(), "Thread ID should be captured");
            assert!(event.process_id.is_some(), "Process ID should be captured");
            assert!(event.correlation_id.is_some(), "Correlation ID should be generated");
        }

        println!("Captured {} events successfully", events.len());
        for event in &events {
            println!("Event: {} ({})", event.event_data.message, event.event_data.level);
        }
    }

    #[test]
    fn test_event_callbacks() {
        use std::sync::{Arc, Mutex};

        // Clear any existing callbacks and events
        clear_event_callbacks();
        clear_global_events();
        init_global_event_manager();

        // Create a shared vector to capture callback events
        let callback_events = Arc::new(Mutex::new(Vec::<Event>::new()));

        // Register a callback
        {
            let callback_events_clone = callback_events.clone();
            register_event_callback(Arc::new(move |event: &Event| {
                callback_events_clone.lock().unwrap().push(event.clone());
            }));
        }

        // Add some events
        let event1 = Event::new(EventData::new("Test callback event 1".to_string(), Level::INFO, "test".to_string()));

        let event2 = Event::new(EventData::new("Test callback event 2".to_string(), Level::ERROR, "test".to_string()));

        add_global_event(event1);
        add_global_event(event2);

        // Verify callbacks were triggered
        let captured_events = callback_events.lock().unwrap();
        assert_eq!(captured_events.len(), 2);
        assert_eq!(captured_events[0].event_data.message, "Test callback event 1");
        assert_eq!(captured_events[1].event_data.message, "Test callback event 2");

        // Verify events were also added to global manager
        assert_eq!(get_global_event_count(), 2);

        println!("Successfully tested event callbacks!");
    }
}
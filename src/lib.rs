// Modular structure for better code organization
mod event;
mod event_data;
mod manager;
mod span;
mod tracing;

// Keep the existing async event system
pub mod events;

// Re-export main types and functions for public API
pub use {
    event::Event,
    event_data::EventData,
    manager::{
        EventManager, ExportData, ExportMetadata, clear_global_events, events, export_filtered_to_bin_file,
        export_to_bin_data, export_to_bin_file, get_event_summary, get_global_event_count, get_global_events,
        import_and_merge_from_bin_file, import_from_bin_file, init_global_event_manager,
        init_global_event_manager_with_count,
    },
    span::SpanInfo,
    tracing::{SpannerLayer, init_tracing_capture, init_layer_only, add_to_subscriber, init_with_subscriber},
};

/// Main initialization function - sets up the complete tracing system
pub fn init() -> Result<(), Box<dyn std::error::Error>> { tracing::init_tracing_capture() }

/// Example usage functions for testing the binary export functionality
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_export() {
        // Initialize the tracing system
        init_global_event_manager();
        clear_global_events();

        // Create some test events
        let event1 = Event::new(EventData::new("Test event 1".to_string(), ::tracing::Level::INFO, "test".to_string()));

        let event2 = Event::new(EventData::new("Test event 2".to_string(), ::tracing::Level::ERROR, "test".to_string()));

        // Add events to manager (using the internal emit function)
        if let Some(events_target) = events() {
            events_target.emit(event1);
            events_target.emit(event2);
        }

        // Test binary export to data
        match export_to_bin_data() {
            Ok(data) => {
                assert!(!data.is_empty(), "Binary data should not be empty");
                println!("Successfully exported {} bytes", data.len());
            }
            Err(e) => panic!("Failed to export binary data: {}", e),
        }

        // Test getting event summary
        let summary = get_event_summary();
        println!("Event summary:\n{}", summary);
    }

    #[test]
    fn test_filtered_export() {
        use std::{fs, path::Path};

        // Initialize the tracing system
        init_global_event_manager();
        clear_global_events();

        // Create test events with different levels
        let events_target = events().expect("Events target should be available");

        events_target.emit(Event::new(EventData::new(
            "Info message".to_string(),
            ::tracing::Level::INFO,
            "test".to_string(),
        )));

        events_target.emit(Event::new(EventData::new(
            "Error message".to_string(),
            ::tracing::Level::ERROR,
            "test".to_string(),
        )));

        events_target.emit(Event::new(EventData::new(
            "Debug message".to_string(),
            ::tracing::Level::DEBUG,
            "test".to_string(),
        )));

        // Test export to file
        let temp_file = "/tmp/test_events.bin";
        match export_to_bin_file(temp_file) {
            Ok(count) => {
                println!("Successfully exported {} events to {}", count, temp_file);

                // Check that file exists and has content
                assert!(Path::new(temp_file).exists(), "Export file should exist");
                let file_size = fs::metadata(temp_file).unwrap().len();
                assert!(file_size > 0, "Export file should not be empty");

                // Test import
                match import_from_bin_file(temp_file) {
                    Ok(imported_manager) => {
                        let imported_count = imported_manager.len();
                        println!("Successfully imported {} events", imported_count);
                        assert_eq!(count, imported_count, "Import count should match export count");

                        // Verify we can access the events
                        let imported_events: Vec<_> = imported_manager.get_recent(10);
                        println!("Imported events: {:?}", imported_events.len());
                    }
                    Err(e) => panic!("Failed to import from file: {}", e),
                }

                // Clean up
                let _ = fs::remove_file(temp_file);
            }
            Err(e) => panic!("Failed to export to file: {}", e),
        }
    }
}

# 🔍 Tracing Spanner

**Span introspection library for `tracing` with event capture and export.**

## Quick Start

```rust
use tracing_spanner::init;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with event capture
    init()?;
    
    tracing::info!("Hello world");
    tracing::error!("Something failed");
    
    Ok(())
}
```
## Initialization Options

```rust
// Simple: Full setup with default subscriber
tracing_spanner::init()?;

// Advanced: Add to existing subscriber
let subscriber = tracing_subscriber::Registry::default()
    .with(tracing_subscriber::fmt::layer());
tracing_spanner::init_with_subscriber(subscriber)?;

// Manual: Just add the layer
let enhanced = tracing_spanner::add_to_subscriber(my_subscriber);
tracing::subscriber::set_global_default(enhanced)?;
```

## Export Events

```rust
use tracing_spanner::*;

// Export all events
export_to_bin_file("events.json")?;

// Export filtered events  
export_filtered_to_bin_file(
    "errors.json",
    Some(tracing::Level::ERROR), // Only errors
    None, None, None,
    Some("Error analysis".to_string())
)?;

// Import for analysis
let manager = import_from_bin_file("events.json")?;
let errors = manager.get_by_level(tracing::Level::ERROR);
```

## Query Events

```rust
// Get global events
let events = get_global_events().unwrap_or_default();

// Access event target for reactive programming
if let Some(target) = events() {
    target.on(|event| {
        println!("New event: {}", event.event_data.message);
    });
}
```

## License

MIT
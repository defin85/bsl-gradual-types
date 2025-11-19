// TODO: Implement event handling in Stage 8
// Events: stopped, output, terminated, etc.

use serde_json::Value;

pub struct EventHandler {
    // Placeholder
}

impl EventHandler {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn handle_event(&self, _event: Value) {
        // TODO: Process DAP events
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}

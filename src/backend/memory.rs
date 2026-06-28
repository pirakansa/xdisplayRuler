use std::io;

use crate::{DisplayBackend, DisplayEvent};

#[derive(Clone, Debug, Default)]
pub struct InMemoryBackend {
    pending_events: Vec<DisplayEvent>,
}

impl InMemoryBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_events(events: impl IntoIterator<Item = DisplayEvent>) -> Self {
        Self {
            pending_events: events.into_iter().collect(),
        }
    }
}

impl DisplayBackend for InMemoryBackend {
    fn name(&self) -> &'static str {
        "in-memory"
    }

    fn poll_events(&mut self) -> io::Result<Vec<DisplayEvent>> {
        Ok(std::mem::take(&mut self.pending_events))
    }
}

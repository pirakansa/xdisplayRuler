use std::io;

use crate::{DisplayBackend, DisplayState};

#[derive(Clone, Debug)]
pub struct DisplayMonitor<B> {
    backend: B,
    state: DisplayState,
}

impl<B> DisplayMonitor<B>
where
    B: DisplayBackend,
{
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            state: DisplayState::new(),
        }
    }

    pub fn state(&self) -> &DisplayState {
        &self.state
    }

    pub fn refresh_once(&mut self) -> io::Result<usize> {
        let events = self.backend.poll_events()?;
        let event_count = events.len();

        for event in events {
            self.state.apply(event);
        }

        Ok(event_count)
    }

    pub fn status_report(&self) -> String {
        self.state.status_report_for_backend(self.backend.name())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        DisplayEvent, DisplayMonitor, DisplayOutput, InMemoryBackend, Rect, WindowId, WindowInfo,
    };

    #[test]
    fn applies_backend_events_once() {
        let backend = InMemoryBackend::with_events([
            DisplayEvent::OutputConnected(DisplayOutput::connected(
                "HDMI-1",
                Rect::new(0, 0, 1920, 1080),
                true,
            )),
            DisplayEvent::WindowMapped(WindowInfo::mapped(
                WindowId(0x42),
                Rect::new(10, 20, 800, 600),
            )),
        ]);
        let mut monitor = DisplayMonitor::new(backend);

        let event_count = monitor.refresh_once().expect("refresh should run");

        assert_eq!(event_count, 2);
        assert_eq!(monitor.state().outputs().len(), 1);
        assert_eq!(monitor.state().windows().len(), 1);
        assert_eq!(monitor.state().top_window(), Some(WindowId(0x42)));
    }
}

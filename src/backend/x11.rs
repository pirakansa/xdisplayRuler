use std::io;

use crate::{DisplayBackend, DisplayEvent};

#[derive(Clone, Debug, Default)]
pub struct X11Backend;

impl X11Backend {
    pub fn connect() -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "x11 backend requires an X11 client implementation that is not available in this build",
        ))
    }
}

impl DisplayBackend for X11Backend {
    fn name(&self) -> &'static str {
        "x11"
    }

    fn poll_events(&mut self) -> io::Result<Vec<DisplayEvent>> {
        Ok(Vec::new())
    }
}

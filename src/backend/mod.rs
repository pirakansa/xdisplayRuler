mod memory;
mod x11;

use std::io;

use crate::DisplayEvent;

pub use memory::InMemoryBackend;
pub use x11::X11Backend;

#[derive(Clone, Debug)]
pub enum ConfiguredBackend {
    InMemory(InMemoryBackend),
    X11(X11Backend),
}

impl ConfiguredBackend {
    pub fn from_name(name: &str) -> Result<Self, BackendError> {
        match name {
            "in-memory" => Ok(Self::InMemory(InMemoryBackend::new())),
            "x11" | "xorg" => X11Backend::connect()
                .map(Self::X11)
                .map_err(BackendError::Io),
            _ => Err(BackendError::UnsupportedName(name.to_string())),
        }
    }
}

impl DisplayBackend for ConfiguredBackend {
    fn name(&self) -> &'static str {
        match self {
            Self::InMemory(backend) => backend.name(),
            Self::X11(backend) => backend.name(),
        }
    }

    fn poll_events(&mut self) -> io::Result<Vec<DisplayEvent>> {
        match self {
            Self::InMemory(backend) => backend.poll_events(),
            Self::X11(backend) => backend.poll_events(),
        }
    }
}

#[derive(Debug)]
pub enum BackendError {
    Io(io::Error),
    UnsupportedName(String),
}

pub trait DisplayBackend {
    fn name(&self) -> &'static str;

    fn poll_events(&mut self) -> io::Result<Vec<DisplayEvent>>;
}

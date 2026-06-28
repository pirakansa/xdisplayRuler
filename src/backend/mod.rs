mod memory;
mod x11;

use std::io;

use crate::{DisplayEvent, WindowId};

pub use memory::InMemoryBackend;
pub use x11::X11Backend;

#[derive(Debug)]
pub enum ConfiguredBackend {
    InMemory(InMemoryBackend),
    X11(Box<X11Backend>),
}

impl ConfiguredBackend {
    pub fn from_name(name: &str) -> Result<Self, BackendError> {
        match name {
            "in-memory" => Ok(Self::InMemory(InMemoryBackend::new())),
            "x11" | "xorg" => X11Backend::connect()
                .map(Box::new)
                .map(Self::X11)
                .map_err(BackendError::Io),
            _ => Err(BackendError::UnsupportedName(name.to_string())),
        }
    }

    pub fn raise_window(&self, id: WindowId) -> io::Result<()> {
        match self {
            Self::InMemory(_) => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "in-memory backend cannot change X11 window stacking",
            )),
            Self::X11(backend) => backend.raise_window(id),
        }
    }

    pub fn lower_window(&self, id: WindowId) -> io::Result<()> {
        match self {
            Self::InMemory(_) => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "in-memory backend cannot change X11 window stacking",
            )),
            Self::X11(backend) => backend.lower_window(id),
        }
    }

    pub fn place_window_fullscreen(&self, id: WindowId, output_name: &str) -> io::Result<()> {
        match self {
            Self::InMemory(_) => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "in-memory backend cannot place X11 windows",
            )),
            Self::X11(backend) => backend.place_window_fullscreen(id, output_name),
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

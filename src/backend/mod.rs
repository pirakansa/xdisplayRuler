mod memory;
mod x11;

use std::io;

use crate::{DisplayEvent, WindowId, WindowInfo};

pub use memory::InMemoryBackend;
pub use x11::X11Backend;

#[derive(Debug)]
pub enum ConfiguredBackend {
    InMemory(InMemoryBackend),
    X11(Box<X11Backend>),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WindowGeometryChange {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputMode {
    pub name: String,
    pub width: u16,
    pub height: u16,
    pub refresh_millihertz: Option<u32>,
    pub preferred: bool,
    pub current: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputModeSelection {
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub refresh_millihertz: Option<u32>,
    pub rotation: Option<OutputRotation>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputRotation {
    Normal,
    Left,
    Right,
    Inverted,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OutputModeChange {
    pub warnings: Vec<String>,
}

impl WindowGeometryChange {
    pub fn is_empty(&self) -> bool {
        self.x.is_none() && self.y.is_none() && self.width.is_none() && self.height.is_none()
    }
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

    pub fn stack_window_above(&self, id: WindowId, sibling: WindowId) -> io::Result<()> {
        match self {
            Self::InMemory(_) => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "in-memory backend cannot change X11 window stacking",
            )),
            Self::X11(backend) => backend.stack_window_above(id, sibling),
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

    pub fn configure_window(&self, id: WindowId, change: &WindowGeometryChange) -> io::Result<()> {
        match self {
            Self::InMemory(_) => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "in-memory backend cannot configure X11 windows",
            )),
            Self::X11(backend) => backend.configure_window(id, change),
        }
    }

    pub fn windows(&self) -> io::Result<Vec<WindowInfo>> {
        match self {
            Self::InMemory(_) => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "in-memory backend cannot resolve X11 windows",
            )),
            Self::X11(backend) => backend.windows(),
        }
    }

    pub fn output_modes(&self, output_name: &str) -> io::Result<Vec<OutputMode>> {
        match self {
            Self::InMemory(_) => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "in-memory backend cannot list X11 output modes",
            )),
            Self::X11(backend) => backend.output_modes(output_name),
        }
    }

    pub fn set_output_mode(
        &self,
        output_name: &str,
        selection: &OutputModeSelection,
    ) -> io::Result<OutputModeChange> {
        match self {
            Self::InMemory(_) => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "in-memory backend cannot change X11 output modes",
            )),
            Self::X11(backend) => backend.set_output_mode(output_name, selection),
        }
    }

    pub fn snapshot_events(&mut self) -> io::Result<Vec<DisplayEvent>> {
        match self {
            Self::InMemory(backend) => backend.poll_events(),
            Self::X11(backend) => backend.snapshot_events(),
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

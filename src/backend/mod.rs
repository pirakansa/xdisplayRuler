mod memory;

use std::io;

use crate::DisplayEvent;

pub use memory::InMemoryBackend;

pub trait DisplayBackend {
    fn name(&self) -> &'static str;

    fn poll_events(&mut self) -> io::Result<Vec<DisplayEvent>>;
}

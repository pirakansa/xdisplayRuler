use std::fmt;

use super::Rect;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WindowId(pub u64);

impl fmt::Display for WindowId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "0x{:x}", self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowInfo {
    pub id: WindowId,
    pub title: Option<String>,
    pub app_id: Option<String>,
    pub class_name: Option<String>,
    pub instance_name: Option<String>,
    pub geometry: Rect,
    pub mapped: bool,
}

impl WindowInfo {
    pub fn mapped(id: WindowId, geometry: Rect) -> Self {
        Self {
            id,
            title: None,
            app_id: None,
            class_name: None,
            instance_name: None,
            geometry,
            mapped: true,
        }
    }
}

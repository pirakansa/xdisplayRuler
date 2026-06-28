use super::Rect;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayOutput {
    pub name: String,
    pub geometry: Rect,
    pub primary: bool,
    pub connected: bool,
}

impl DisplayOutput {
    pub fn connected(name: impl Into<String>, geometry: Rect, primary: bool) -> Self {
        Self {
            name: name.into(),
            geometry,
            primary,
            connected: true,
        }
    }
}

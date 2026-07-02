use crate::{DisplayEvent, DisplayOutput, Rect, WindowId, WindowInfo};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct X11Snapshot {
    pub(super) outputs: Vec<X11OutputSnapshot>,
    pub(super) windows: Vec<X11WindowSnapshot>,
    pub(super) focused_window: Option<WindowId>,
}

impl X11Snapshot {
    pub(super) fn into_events(self) -> Vec<DisplayEvent> {
        let mut events = vec![DisplayEvent::Reset];

        events.extend(self.outputs.into_iter().map(X11OutputSnapshot::into_event));
        events.extend(
            self.windows
                .into_iter()
                .map(|window| DisplayEvent::WindowMapped(window.into_window_info())),
        );
        events.push(DisplayEvent::FocusChanged(self.focused_window));

        events
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct X11OutputSnapshot {
    pub(super) name: String,
    pub(super) geometry: Option<Rect>,
    pub(super) primary: bool,
}

impl X11OutputSnapshot {
    pub(super) fn connected(name: impl Into<String>, geometry: Rect, primary: bool) -> Self {
        Self {
            name: name.into(),
            geometry: Some(geometry),
            primary,
        }
    }

    pub(super) fn disconnected(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            geometry: None,
            primary: false,
        }
    }

    fn into_event(self) -> DisplayEvent {
        match self.geometry {
            Some(geometry) => DisplayEvent::OutputConnected(DisplayOutput::connected(
                self.name,
                geometry,
                self.primary,
            )),
            None => DisplayEvent::OutputDisconnected { name: self.name },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct X11WindowSnapshot {
    pub(super) id: WindowId,
    pub(super) title: Option<String>,
    pub(super) class_name: Option<String>,
    pub(super) instance_name: Option<String>,
    pub(super) geometry: Rect,
}

impl X11WindowSnapshot {
    pub(super) fn into_window_info(self) -> WindowInfo {
        let mut window = WindowInfo::mapped(self.id, self.geometry);
        window.title = self.title;
        window.class_name = self.class_name;
        window.instance_name = self.instance_name;
        window
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct X11WindowClass {
    pub(super) instance_name: String,
    pub(super) class_name: String,
}

#[cfg(test)]
mod tests {
    use super::{X11OutputSnapshot, X11Snapshot, X11WindowSnapshot};
    use crate::{DisplayEvent, DisplayOutput, Rect, WindowId, WindowInfo};

    #[test]
    fn converts_snapshot_to_reset_and_current_state_events() {
        let snapshot = X11Snapshot {
            outputs: vec![
                X11OutputSnapshot::connected("HDMI-1", Rect::new(0, 0, 1920, 1080), true),
                X11OutputSnapshot::disconnected("DP-1"),
            ],
            windows: vec![
                X11WindowSnapshot {
                    id: WindowId(0x10),
                    title: Some("first".to_string()),
                    class_name: Some("Code".to_string()),
                    instance_name: Some("code".to_string()),
                    geometry: Rect::new(0, 0, 800, 600),
                },
                X11WindowSnapshot {
                    id: WindowId(0x20),
                    title: None,
                    class_name: None,
                    instance_name: None,
                    geometry: Rect::new(800, 0, 640, 480),
                },
            ],
            focused_window: Some(WindowId(0x20)),
        };

        let events = snapshot.into_events();

        assert_eq!(events[0], DisplayEvent::Reset);
        assert_eq!(
            events[1],
            DisplayEvent::OutputConnected(DisplayOutput::connected(
                "HDMI-1",
                Rect::new(0, 0, 1920, 1080),
                true,
            ))
        );
        assert_eq!(
            events[2],
            DisplayEvent::OutputDisconnected {
                name: "DP-1".to_string(),
            }
        );
        assert_eq!(
            events[3],
            DisplayEvent::WindowMapped(WindowInfo {
                id: WindowId(0x10),
                title: Some("first".to_string()),
                app_id: None,
                class_name: Some("Code".to_string()),
                instance_name: Some("code".to_string()),
                geometry: Rect::new(0, 0, 800, 600),
                mapped: true,
            })
        );
        assert_eq!(events[5], DisplayEvent::FocusChanged(Some(WindowId(0x20))));
    }

    #[test]
    fn disconnected_outputs_ignore_primary_flag() {
        let event = X11OutputSnapshot::disconnected("HDMI-2").into_event();

        assert_eq!(
            event,
            DisplayEvent::OutputDisconnected {
                name: "HDMI-2".to_string(),
            }
        );
    }
}

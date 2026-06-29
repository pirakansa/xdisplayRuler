use crate::{DisplayEvent, DisplayOutput, DisplayState, Rect, WindowId, WindowInfo};

#[test]
fn tracks_outputs_and_primary_display() {
    let mut state = DisplayState::new();

    state.apply(DisplayEvent::OutputConnected(DisplayOutput::connected(
        "HDMI-1",
        Rect::new(0, 0, 1920, 1080),
        true,
    )));
    state.apply(DisplayEvent::OutputConnected(DisplayOutput::connected(
        "DP-1",
        Rect::new(1920, 0, 1280, 720),
        true,
    )));
    state.apply(DisplayEvent::OutputDisconnected {
        name: "HDMI-1".to_string(),
    });

    assert_eq!(state.outputs().len(), 2);
    assert!(!state.outputs()[0].connected);
    assert!(!state.outputs()[0].primary);
    assert!(state.outputs()[1].primary);
}

#[test]
fn tracks_window_stacking_focus_and_unmap() {
    let mut state = DisplayState::new();
    let first = WindowId(0x20);
    let second = WindowId(0x30);

    state.apply(DisplayEvent::WindowMapped(WindowInfo::mapped(
        first,
        Rect::new(0, 0, 800, 600),
    )));
    state.apply(DisplayEvent::WindowMapped(WindowInfo::mapped(
        second,
        Rect::new(20, 20, 1024, 768),
    )));
    state.apply(DisplayEvent::WindowRaised(first));
    state.apply(DisplayEvent::FocusChanged(Some(first)));

    assert_eq!(state.stacking_order(), &[second, first]);
    assert_eq!(state.top_window(), Some(first));
    assert_eq!(state.focused_window(), Some(first));

    state.apply(DisplayEvent::WindowUnmapped(first));

    assert_eq!(state.stacking_order(), &[second]);
    assert_eq!(state.top_window(), Some(second));
    assert_eq!(state.focused_window(), None);
}

#[test]
fn status_report_includes_escaped_window_properties_when_present() {
    let mut state = DisplayState::new();
    let mut window = WindowInfo::mapped(WindowId(0x20), Rect::new(0, 0, 800, 600));
    window.title = Some("hello \"display\"\n".to_string());
    window.class_name = Some("Code".to_string());
    window.instance_name = Some("code".to_string());

    state.apply(DisplayEvent::WindowMapped(window));

    assert!(state.status_report().contains(
        "- 0x20: mapped 800x600+0+0 title=\"hello \\\"display\\\"\\n\" class=\"Code\" instance=\"code\""
    ));
}

#[test]
fn reset_clears_outputs_windows_stacking_and_focus() {
    let mut state = DisplayState::new();

    state.apply(DisplayEvent::OutputConnected(DisplayOutput::connected(
        "HDMI-1",
        Rect::new(0, 0, 1920, 1080),
        true,
    )));
    state.apply(DisplayEvent::WindowMapped(WindowInfo::mapped(
        WindowId(0x40),
        Rect::new(0, 0, 800, 600),
    )));
    state.apply(DisplayEvent::FocusChanged(Some(WindowId(0x40))));

    state.apply(DisplayEvent::Reset);

    assert!(state.outputs().is_empty());
    assert!(state.windows().is_empty());
    assert!(state.stacking_order().is_empty());
    assert_eq!(state.focused_window(), None);
}

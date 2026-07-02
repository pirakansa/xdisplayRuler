use std::{
    cell::RefCell,
    fs, io,
    path::PathBuf,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    backend::{WindowGeometryChange, WindowLayoutBackend},
    DisplayEvent, DisplayOutput, Rect, WindowId, WindowInfo,
};

use super::{runner::run_with_layout_backend, EnforceOptions};

#[derive(Clone, Debug)]
struct LayoutOnlyBackend {
    events: Vec<DisplayEvent>,
    configured_windows: Rc<RefCell<Vec<(WindowId, WindowGeometryChange)>>>,
}

impl WindowLayoutBackend for LayoutOnlyBackend {
    fn snapshot_events(&mut self) -> io::Result<Vec<DisplayEvent>> {
        Ok(std::mem::take(&mut self.events))
    }

    fn configure_window(&self, id: WindowId, change: &WindowGeometryChange) -> io::Result<()> {
        self.configured_windows
            .borrow_mut()
            .push((id, change.clone()));
        Ok(())
    }

    fn raise_window(&self, _id: WindowId) -> io::Result<()> {
        Ok(())
    }

    fn stack_window_above(&self, _id: WindowId, _sibling: WindowId) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn enforce_runs_with_layout_only_backend() {
    let layout_path = write_temp_layout(
        r#"{
                "schema_version": 1,
                "windows": [
                    { "selector": { "app_id": "Player" }, "output": "HDMI-2" }
                ]
            }"#,
    );
    let configured_windows = Rc::new(RefCell::new(Vec::new()));
    let backend = LayoutOnlyBackend {
        events: vec![
            DisplayEvent::OutputConnected(DisplayOutput::connected(
                "HDMI-2",
                Rect::new(100, 50, 1920, 1080),
                true,
            )),
            DisplayEvent::WindowMapped(test_window()),
        ],
        configured_windows: Rc::clone(&configured_windows),
    };
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    run_with_layout_backend(
        EnforceOptions {
            backend_name: "layout-only".to_string(),
            layout_path: layout_path.to_string_lossy().into_owned(),
            once: true,
            dry_run: false,
            interval_millis: 1_000,
        },
        &mut stdout,
        &mut stderr,
        |_| Ok(backend.clone()),
    )
    .expect("enforce should run with layout-only backend");

    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
    assert_eq!(
        *configured_windows.borrow(),
        vec![(
            WindowId(0x10),
            WindowGeometryChange {
                x: Some(100),
                y: Some(50),
                width: Some(1920),
                height: Some(1080),
            },
        )]
    );

    fs::remove_file(layout_path).expect("temp layout should be removable");
}

fn test_window() -> WindowInfo {
    let mut window = WindowInfo::mapped(WindowId(0x10), Rect::new(0, 0, 800, 600));
    window.class_name = Some("Player".to_string());
    window
}

fn write_temp_layout(content: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after UNIX epoch")
        .as_nanos();
    path.push(format!(
        "xdisplay-ruler-enforce-test-{}-{unique}.json",
        std::process::id()
    ));
    fs::write(&path, content).expect("temp layout should be writable");
    path
}

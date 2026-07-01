use std::{collections::HashSet, io::Write, thread, time::Duration};

use crate::{backend::WindowLayoutBackend, ConfiguredBackend};

mod executor;
mod planner;
mod report;

use executor::apply_plan;
use planner::EnforcementSession;
use report::{write_dry_run_report, write_new_warnings, write_warnings};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EnforceOptions {
    pub(crate) backend_name: String,
    pub(crate) layout_path: String,
    pub(crate) once: bool,
    pub(crate) dry_run: bool,
    pub(crate) interval_millis: usize,
}

impl EnforceOptions {
    fn single_cycle_mode(&self) -> Option<EnforceCycleMode> {
        if self.dry_run {
            Some(EnforceCycleMode::DryRun)
        } else if self.once {
            Some(EnforceCycleMode::ApplyOnce)
        } else {
            None
        }
    }

    fn loop_interval(&self) -> Duration {
        Duration::from_millis(self.interval_millis as u64)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EnforceCycleMode {
    DryRun,
    ApplyOnce,
}

pub(crate) fn run(
    options: EnforceOptions,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
    build_backend: impl Fn(&str) -> Result<ConfiguredBackend, String>,
) -> Result<(), String> {
    run_with_layout_backend(options, stdout, stderr, build_backend)
}

fn run_with_layout_backend<B>(
    options: EnforceOptions,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
    build_backend: impl Fn(&str) -> Result<B, String>,
) -> Result<(), String>
where
    B: WindowLayoutBackend,
{
    let single_cycle_mode = options.single_cycle_mode();
    let loop_interval = options.loop_interval();
    let mut session = EnforcementSession::new(&options, build_backend)?;

    if let Some(mode) = single_cycle_mode {
        run_single_cycle(&mut session, mode, stdout, stderr)
    } else {
        run_daemon(&mut session, loop_interval, stderr)
    }
}

fn run_single_cycle(
    session: &mut EnforcementSession<impl WindowLayoutBackend>,
    mode: EnforceCycleMode,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<(), String> {
    let plan = match mode {
        EnforceCycleMode::DryRun => session.build_recoverable_plan()?,
        EnforceCycleMode::ApplyOnce => session.build_recoverable_plan()?,
    };
    write_warnings(&plan, stderr)?;

    match mode {
        EnforceCycleMode::DryRun => write_dry_run_report(&plan, stdout),
        EnforceCycleMode::ApplyOnce => apply_plan(session.backend(), &plan),
    }
}

fn run_daemon(
    session: &mut EnforcementSession<impl WindowLayoutBackend>,
    interval: Duration,
    stderr: &mut impl Write,
) -> Result<(), String> {
    let mut previous_warnings = HashSet::new();
    loop {
        let plan = session.build_recoverable_plan()?;
        write_new_warnings(&plan, stderr, &mut previous_warnings)?;
        apply_plan(session.backend(), &plan)?;
        thread::sleep(interval);
    }
}

#[cfg(test)]
mod tests {
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

    use super::{run_with_layout_backend, EnforceOptions};

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
}

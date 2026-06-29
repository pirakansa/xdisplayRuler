use crate::{
    report::{escape_value, format_millihertz},
    OutputMode,
};

pub(super) fn modes_report(output_name: &str, modes: &[OutputMode]) -> String {
    let mut report = format!(
        "xdisplay-ruler\noutput: {output_name}\nmodes: {}\n",
        modes.len()
    );

    for mode in modes {
        let refresh = mode
            .refresh_millihertz
            .map(format_millihertz)
            .unwrap_or_else(|| "unknown-rate".to_string());
        let current = if mode.current { " current" } else { "" };
        let preferred = if mode.preferred { " preferred" } else { "" };

        report.push_str(&format!(
            "- {}x{} {} name=\"{}\"{}{}\n",
            mode.width,
            mode.height,
            refresh,
            escape_value(&mode.name),
            current,
            preferred
        ));
    }

    report
}

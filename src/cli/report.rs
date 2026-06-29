use crate::OutputMode;

pub(super) fn modes_report(output_name: &str, modes: &[OutputMode]) -> String {
    let mut report = format!(
        "xdisplay-ruler\noutput: {output_name}\nmodes: {}\n",
        modes.len()
    );

    for mode in modes {
        let refresh = mode
            .refresh_millihertz
            .map(format_refresh_millihertz)
            .unwrap_or_else(|| "unknown-rate".to_string());
        let current = if mode.current { " current" } else { "" };
        let preferred = if mode.preferred { " preferred" } else { "" };

        report.push_str(&format!(
            "- {}x{} {} name=\"{}\"{}{}\n",
            mode.width,
            mode.height,
            refresh,
            escape_report_value(&mode.name),
            current,
            preferred
        ));
    }

    report
}

fn format_refresh_millihertz(refresh_millihertz: u32) -> String {
    let hz = refresh_millihertz / 1000;
    let fraction = refresh_millihertz % 1000;

    if fraction == 0 {
        format!("{hz}Hz")
    } else {
        let mut fraction = format!("{fraction:03}");
        while fraction.ends_with('0') {
            fraction.pop();
        }
        format!("{hz}.{fraction}Hz")
    }
}

fn escape_report_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

use super::DisplayState;

impl DisplayState {
    pub fn status_report(&self) -> String {
        self.status_report_for_backend("in-memory")
    }

    pub fn status_report_for_backend(&self, backend_name: &str) -> String {
        let mut report = format!("xdisplay-ruler\nbackend: {backend_name}\n");
        report.push_str(&format!("outputs: {}\n", self.outputs().len()));
        report.push_str(&self.output_report());
        report.push_str(&format!("windows: {}\n", self.windows().len()));
        report.push_str(&self.window_report());
        report.push_str(&format!("focused: {}\n", self.focused_window_label()));
        report.push_str(&format!("top: {}\n", self.top_window_label()));
        report
    }

    fn output_report(&self) -> String {
        self.outputs()
            .iter()
            .map(|output| {
                let primary = if output.primary { " primary" } else { "" };
                let status = if output.connected {
                    "connected"
                } else {
                    "disconnected"
                };

                format!(
                    "- {}: {} {}{}\n",
                    output.name, status, output.geometry, primary
                )
            })
            .collect()
    }

    fn window_report(&self) -> String {
        self.windows()
            .iter()
            .map(|window| {
                let mapped = if window.mapped { "mapped" } else { "unmapped" };
                let title = window
                    .title
                    .as_deref()
                    .map(|title| window_property_report("title", title))
                    .unwrap_or_default();
                let class = window
                    .class_name
                    .as_deref()
                    .map(|class| window_property_report("class", class))
                    .unwrap_or_default();
                let instance = window
                    .instance_name
                    .as_deref()
                    .map(|instance| window_property_report("instance", instance))
                    .unwrap_or_default();
                format!(
                    "- {}: {} {}{}{}{}\n",
                    window.id, mapped, window.geometry, title, class, instance
                )
            })
            .collect()
    }

    fn focused_window_label(&self) -> String {
        self.focused_window()
            .map_or_else(|| "none".to_string(), |id| id.to_string())
    }

    fn top_window_label(&self) -> String {
        self.top_window()
            .map_or_else(|| "none".to_string(), |id| id.to_string())
    }
}

fn window_property_report(name: &str, value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }

    format!(" {name}=\"{}\"", escape_report_value(value))
}

fn escape_report_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

use std::{fmt, fs, path::Path};

use serde::Deserialize;

use crate::WindowId;

const SUPPORTED_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutPolicy {
    pub schema_version: u32,
    #[serde(default)]
    pub unmanaged_windows: UnmanagedWindowsPolicy,
    pub windows: Vec<ManagedWindowRule>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedWindowRule {
    pub selector: WindowSelector,
    pub output: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WindowSelector {
    Id(WindowId),
    Title(String),
    AppId(String),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnmanagedWindowsPolicy {
    #[default]
    AllowAbove,
    KeepBelowManaged,
}

impl LayoutPolicy {
    pub fn read_from_path(path: impl AsRef<Path>) -> Result<Self, LayoutError> {
        let content = fs::read_to_string(path).map_err(LayoutError::Read)?;
        Self::from_json_str(&content)
    }

    pub fn from_json_str(content: &str) -> Result<Self, LayoutError> {
        let policy = serde_json::from_str::<Self>(content).map_err(LayoutError::Json)?;
        policy.validate()?;
        Ok(policy)
    }

    pub fn validate(&self) -> Result<(), LayoutError> {
        if self.schema_version != SUPPORTED_SCHEMA_VERSION {
            return Err(LayoutError::UnsupportedSchemaVersion(self.schema_version));
        }

        Ok(())
    }
}

impl fmt::Display for WindowSelector {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(id) => write!(formatter, "id:{id}"),
            Self::Title(title) => write!(formatter, "title:\"{}\"", escape_report_value(title)),
            Self::AppId(app_id) => write!(formatter, "app_id:\"{}\"", escape_report_value(app_id)),
        }
    }
}

impl<'de> Deserialize<'de> for WindowSelector {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct SelectorFields {
            id: Option<String>,
            title: Option<String>,
            app_id: Option<String>,
        }

        let fields = SelectorFields::deserialize(deserializer)?;
        let set_count = usize::from(fields.id.is_some())
            + usize::from(fields.title.is_some())
            + usize::from(fields.app_id.is_some());

        if set_count != 1 {
            return Err(serde::de::Error::custom(
                "selector must contain exactly one of: id, title, app_id",
            ));
        }

        if let Some(id) = fields.id {
            return parse_window_id(&id)
                .map(Self::Id)
                .map_err(serde::de::Error::custom);
        }
        if let Some(title) = fields.title {
            return Ok(Self::Title(title));
        }
        if let Some(app_id) = fields.app_id {
            return Ok(Self::AppId(app_id));
        }

        unreachable!("selector count was checked above")
    }
}

#[derive(Debug)]
pub enum LayoutError {
    Read(std::io::Error),
    Json(serde_json::Error),
    UnsupportedSchemaVersion(u32),
    SelectorNotFound(WindowSelector),
    SelectorAmbiguous {
        selector: WindowSelector,
        matches: Vec<WindowId>,
    },
    OutputNotFound(String),
    OutputDisconnected(String),
}

impl fmt::Display for LayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(formatter, "layout read failed: {error}"),
            Self::Json(error) => write!(formatter, "layout JSON is invalid: {error}"),
            Self::UnsupportedSchemaVersion(version) => {
                write!(formatter, "unsupported layout schema_version: {version}")
            }
            Self::SelectorNotFound(selector) => write!(formatter, "window not found: {selector}"),
            Self::SelectorAmbiguous { selector, matches } => {
                write!(formatter, "window selector is ambiguous: {selector}")?;
                for id in matches {
                    write!(formatter, "\n- {id}")?;
                }
                Ok(())
            }
            Self::OutputNotFound(output) => write!(formatter, "output not found: {output}"),
            Self::OutputDisconnected(output) => {
                write!(formatter, "output is disconnected: {output}")
            }
        }
    }
}

impl std::error::Error for LayoutError {}

fn parse_window_id(value: &str) -> Result<WindowId, String> {
    let normalized = value.trim();
    let parsed = normalized
        .strip_prefix("0x")
        .or_else(|| normalized.strip_prefix("0X"))
        .map_or_else(
            || normalized.parse::<u64>(),
            |hex| u64::from_str_radix(hex, 16),
        )
        .map_err(|_| format!("id must be an X11 window id, got: {value}"))?;

    Ok(WindowId(parsed))
}

fn escape_report_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

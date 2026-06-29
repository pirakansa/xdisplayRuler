use std::io;

use crate::WindowId;

use super::types::X11WindowClass;

pub(super) fn text_property_value(value: &[u8]) -> Option<String> {
    if value.is_empty() {
        return None;
    }

    let text = String::from_utf8_lossy(value)
        .trim_end_matches('\0')
        .to_string();

    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

pub(super) fn window_class_value(value: &[u8]) -> Option<X11WindowClass> {
    let mut parts = value
        .split(|byte| *byte == b'\0')
        .filter(|part| !part.is_empty())
        .map(|part| String::from_utf8_lossy(part).into_owned());
    let instance_name = parts.next()?;
    let class_name = parts.next()?;

    Some(X11WindowClass {
        instance_name,
        class_name,
    })
}

pub(super) fn x11_window_id(id: WindowId) -> io::Result<u32> {
    u32::try_from(id.0).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("window id {id} does not fit in an X11 window id"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{text_property_value, window_class_value, x11_window_id};
    use crate::WindowId;

    #[test]
    fn validates_x11_window_id_range() {
        assert_eq!(
            x11_window_id(WindowId(u64::from(u32::MAX))).expect("id should fit"),
            u32::MAX
        );
        assert!(x11_window_id(WindowId(u64::from(u32::MAX) + 1)).is_err());
    }

    #[test]
    fn normalizes_text_property_values() {
        assert_eq!(
            text_property_value(b"plain title"),
            Some("plain title".to_string())
        );
        assert_eq!(
            text_property_value(b"legacy title\0\0"),
            Some("legacy title".to_string())
        );
        assert_eq!(text_property_value(b""), None);
        assert_eq!(text_property_value(b"\0\0"), None);
    }

    #[test]
    fn parses_window_class_values() {
        let class = window_class_value(b"code\0Code\0").expect("class should parse");

        assert_eq!(class.instance_name, "code");
        assert_eq!(class.class_name, "Code");
        assert_eq!(window_class_value(b"code\0"), None);
        assert_eq!(window_class_value(b""), None);
    }
}

use x11rb::protocol::Event;

pub(super) fn is_relevant_event(event: &Event) -> bool {
    matches!(
        event,
        Event::RandrNotify(_)
            | Event::RandrScreenChangeNotify(_)
            | Event::ConfigureNotify(_)
            | Event::CreateNotify(_)
            | Event::DestroyNotify(_)
            | Event::MapNotify(_)
            | Event::UnmapNotify(_)
            | Event::ReparentNotify(_)
            | Event::PropertyNotify(_)
            | Event::FocusIn(_)
            | Event::FocusOut(_)
    )
}

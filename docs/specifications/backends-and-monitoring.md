# Backends and Monitoring

Backends collect display events. The monitor applies those events to
`DisplayState`.

## Backend Boundary

`DisplayBackend` exposes:

- `name()`: stable backend label for reports.
- `poll_events()`: returns pending `DisplayEvent` values.

The current implementation includes `InMemoryBackend`. It is used by the CLI and
test suite, and drains its configured events the first time it is polled.

## Monitor Flow

`DisplayMonitor` owns a backend and a `DisplayState`.

`refresh_once()`:

1. polls the backend
2. applies each returned event to the state
3. returns the number of applied events

`status_report()` renders the current state with the backend label.

## Planned Backend

The Xorg/XRandR backend should implement `DisplayBackend` and translate X11
output, window, stacking, and focus changes into `DisplayEvent` values.

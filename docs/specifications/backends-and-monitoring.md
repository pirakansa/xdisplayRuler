# Backends and Monitoring

Backends collect display events. The monitor applies those events to
`DisplayState`.

## Backend Boundary

`DisplayBackend` exposes:

- `name()`: stable backend label for reports.
- `poll_events()`: returns pending `DisplayEvent` values.

The current implementation includes `InMemoryBackend`. It is used by the test
suite and drains its configured events the first time it is polled.

`X11Backend` connects to the Xorg server through the pure Rust `x11rb` protocol
client. It does not call the `xrandr` command and does not link to `libXrandr`.
On connection, it verifies that the server exposes the RANDR extension.

The current X11 backend collects an initial snapshot:

- RANDR outputs and CRTC geometry
- root-level viewable windows
- root-level window geometry
- current input focus

The current X11 backend can also send low-level stacking requests:

- raise a window with X11 `ConfigureWindow` stack mode `Above`
- lower a window with X11 `ConfigureWindow` stack mode `Below`

## Monitor Flow

`DisplayMonitor` owns a backend and a `DisplayState`.

`refresh_once()`:

1. polls the backend
2. applies each returned event to the state
3. returns the number of applied events

`status_report()` renders the current state with the backend label.

The CLI uses this flow for both `snapshot` and `watch` commands. `snapshot`
refreshes once. `watch` refreshes repeatedly, optionally bounded by
`--iterations`.

## Planned Backend

The next X11/RandR step is event subscription. The backend should select RANDR
and window events on the root window, wait for X11 events, and translate output,
window, stacking, and focus changes into `DisplayEvent` values.

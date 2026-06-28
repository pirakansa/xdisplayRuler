# Backends and Monitoring

Backends collect display events. The monitor applies those events to
`DisplayState`.

## Backend Boundary

`DisplayBackend` exposes:

- `name()`: stable backend label for reports.
- `poll_events()`: returns pending `DisplayEvent` values.

The current implementation includes `InMemoryBackend`. It is used by the CLI and
test suite, and drains its configured events the first time it is polled.

`X11Backend` is present as the future integration point. Its `connect()` method
currently returns an unsupported error because no X11 client implementation is
included in this build.

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

The Xorg/XRandR backend should replace the current `X11Backend::connect()`
placeholder and translate X11 output, window, stacking, and focus changes into
`DisplayEvent` values.

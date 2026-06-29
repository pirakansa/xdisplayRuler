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
- root-level window titles from `_NET_WM_NAME`, falling back to `WM_NAME`
- root-level `WM_CLASS` instance and class names
- current input focus

After the initial snapshot, the X11 backend subscribes to RANDR and root-window
events. When a relevant event arrives, it refreshes the snapshot and emits a
state reset followed by the current output and window events.

The current X11 backend can list and switch output modes:

- list modes reported by RandR for a selected output
- mark the current CRTC mode and RandR preferred modes
- switch to an existing output mode with RandR `SetCrtcConfig`
- preserve the active CRTC position, rotation, and output list while switching
  mode, unless a new basic rotation is explicitly selected
- reuse the current active CRTC mode when only rotation is selected
- after a successful mode switch, refresh the selected output rectangle and
  update `Coordinate Transformation Matrix` on every enabled XInput device with
  a Touch class so touch coordinates follow the output's root-relative
  rectangle and basic rotation; if this touch remapping fails after RandR
  accepts the mode switch, the mode change remains successful and the backend
  returns a warning for the CLI to report

The current X11 backend can also send low-level window configuration requests:

- raise a window with X11 `ConfigureWindow` stack mode `Above`
- lower a window with X11 `ConfigureWindow` stack mode `Below`
- move or resize a window by sending provided X, Y, width, and height fields
  with X11 `ConfigureWindow`
- place a window fullscreen on a RandR output by configuring its X, Y, width,
  and height to the selected output geometry, then raising it

## Monitor Flow

`DisplayMonitor` owns a backend and a `DisplayState`.

`refresh_once()`:

1. polls the backend
2. applies each returned event to the state
3. returns the number of applied events

`status_report()` renders the current state with the backend label.

The CLI uses this flow for both `snapshot` and `watch` commands. `snapshot`
refreshes once. `watch` refreshes repeatedly until stopped, unless a test or
diagnostic run bounds it with `--iterations`.

## Planned Backend

The next X11/RandR step is richer event handling. The backend currently
refreshes a complete snapshot after relevant events instead of deriving a small
per-event delta.

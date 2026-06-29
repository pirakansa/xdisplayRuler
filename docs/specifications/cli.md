# CLI

The binary is named `xdisplay-ruler`.

## Arguments

- No arguments: run the default `snapshot` command with the X11 backend.
- `snapshot`: print the current display snapshot once.
- `watch`: keep refreshing and printing display snapshots.
- `modes`: list the modes RandR reports for an output.
- `mode`: change an output to an existing RandR mode.
- `place`: move and resize an X11 window onto an output.
- `configure`: move or resize an X11 window with explicit geometry values.
- `raise`: raise an X11 window above its siblings.
- `lower`: lower an X11 window below its siblings.
- `--backend x11` or `--backend xorg`: select the X11/RandR backend.
- `--backend in-memory`: select the deterministic in-memory backend for tests
  and diagnostics.
- `--iterations N`: stop `watch` mode after `N` snapshots. This is intended for
  tests, scripts, and diagnostics. The value must be a positive integer. When
  omitted, `watch` keeps running.
- `--output NAME`: select a RandR output for `modes`, `mode`, or `place`.
- `--rate HZ`: select a refresh rate for `mode`. Values such as `60`, `59.94`,
  and `59.940` are accepted. Values are interpreted as Hz and stored internally
  as millihertz.
- `--fullscreen`: place the selected window fullscreen on the selected output.
- `--window ID`: select an X11 window ID for `place`, `configure`, `raise`, or
  `lower`. Hex values such as `0x800003` and decimal values are accepted.
- `--window-title NAME`: select a window by exact X11 window title for `place`,
  `configure`, `raise`, or `lower`.
- `--window-class NAME`: select a window by exact `WM_CLASS` class name for
  `place`, `configure`, `raise`, or `lower`.
- `--window-instance NAME`: select a window by exact `WM_CLASS` instance name
  for `place`, `configure`, `raise`, or `lower`.
- `--x N`: set the selected window X position for `configure`. The value must
  be an integer.
- `--y N`: set the selected window Y position for `configure`. The value must
  be an integer.
- `--width N`: set the selected window width for `configure`. The value must be
  a positive integer.
- `--height N`: set the selected window height for `configure`. The value must
  be a positive integer.
- `--help` or `-h`: print command help.
- `--version` or `-V`: print the package version.
- Any unsupported command, argument, backend, or option value: print an error to
  standard error and exit with status `2`.

## Snapshot Output

The in-memory diagnostic snapshot is:

```text
xdisplay-ruler
backend: in-memory
outputs: 0
windows: 0
focused: none
top: none
```

The backend label is `x11` or `in-memory`.

Window rows include `title="..."`, `class="..."`, and `instance="..."` when the
backend reports the X11 window title or `WM_CLASS` values. Quotes, backslashes,
and control characters are escaped in these values.

## Modes Output

`modes --output NAME` prints:

```text
xdisplay-ruler
output: HDMI-2
modes: 2
- 1920x1080 60Hz name="1920x1080" current preferred
- 1280x720 59.94Hz name="1280x720"
```

Mode rows include width, height, refresh rate, RandR mode name, and optional
`current` and `preferred` markers.

## Backend Selection

The current build supports `x11`, `xorg`, and `in-memory`. `xorg` is an alias
for the X11 backend. `x11` is the default.

The X11 backend requires a reachable Xorg server through the usual `DISPLAY`
environment. It verifies that the server provides the RANDR extension before
collecting a snapshot.

`modes`, `mode`, `place`, `configure`, `raise`, and `lower` default to the X11
backend because they are real X11 or RandR operations. Selecting
`--backend in-memory` for those commands returns a usage error.

`mode` requires `--output`, `--width`, and `--height`. `--rate` is optional. It
selects from modes already reported by RandR for the output and sends
`SetCrtcConfig` to the output's active CRTC while preserving the CRTC position,
rotation, and output list. It does not create custom modelines.

`place` currently requires `--fullscreen`. It uses the selected output geometry,
configures the target window to that rectangle, and raises the window.

`place`, `configure`, `raise`, and `lower` require exactly one window selector:
`--window`, `--window-title`, `--window-class`, or `--window-instance`. Name
selectors match exactly and only consider mapped windows. If a selector matches
no windows, the command returns `window not found: NAME`. If a selector matches
multiple windows, the command returns `window selector is ambiguous: NAME` with
candidate IDs and window metadata. In ambiguous cases, rerun the command with
`--window ID` to select a specific candidate.

`configure` requires at least one of `--x`, `--y`, `--width`, or `--height`. It
only sends the geometry fields that were provided.

# CLI

The binary is named `xdisplay-ruler`.

## Arguments

- No arguments: run the default `snapshot` command with the X11 backend.
- `snapshot`: print the current display snapshot once.
- `watch`: keep refreshing and printing display snapshots.
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
- `--output NAME`: select a RandR output for `place`.
- `--fullscreen`: place the selected window fullscreen on the selected output.
- `--window ID`: select an X11 window ID for `place`, `configure`, `raise`, or
  `lower`. Hex values such as `0x800003` and decimal values are accepted.
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

Window rows include `title="..."` when the backend reports a window title.
Quotes, backslashes, and control characters are escaped in the title value.

## Backend Selection

The current build supports `x11`, `xorg`, and `in-memory`. `xorg` is an alias
for the X11 backend. `x11` is the default.

The X11 backend requires a reachable Xorg server through the usual `DISPLAY`
environment. It verifies that the server provides the RANDR extension before
collecting a snapshot.

`place`, `configure`, `raise`, and `lower` default to the X11 backend because
they are real X11 window operations. Selecting `--backend in-memory` for those
commands returns a usage error.

`place` currently requires `--fullscreen`. It uses the selected output geometry,
configures the target window to that rectangle, and raises the window.

`configure` requires `--window` and at least one of `--x`, `--y`, `--width`, or
`--height`. It only sends the geometry fields that were provided.

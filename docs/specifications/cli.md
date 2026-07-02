# CLI

The binary is named `xdisplay-ruler`.

## Command Groups

- Snapshot commands: inspect the current display and window state.
- Window control commands: raise, lower, move, resize, or place X11 windows.
- Layout command: enforce a JSON layout for managed kiosk windows.
- Other commands: print help or version information.

## Implementation Layout

- `src/cli/mod.rs`: top-level CLI flow, help/version fast paths, and exit mapping.
- `src/cli/help.rs`: the user-facing help document embedded in the binary.
- `src/cli/options.rs`: command selection, argument parsing, and scalar validation.
- `src/cli/command.rs`: backend construction, selector resolution, and side-effecting commands.

## Commands

- No arguments: run the default `snapshot` command with the X11 backend.
- `snapshot`: print the current display snapshot once.
- `watch`: keep refreshing and printing display snapshots.
- `enforce`: fit layout-defined windows to their outputs.
- `raise`: raise an X11 window above its siblings.
- `lower`: lower an X11 window below its siblings.
- `configure`: move or resize an X11 window with explicit geometry values.
- `place`: move and resize an X11 window onto an output.
- `--help` or `-h`: print command help.
- `--version` or `-V`: print the package version.

Any unsupported command, argument, backend, or option value prints an error to
standard error and exits with status `2`.

## Option Groups

### Global Options

- `--backend x11` or `--backend xorg`: select the X11/RandR backend.
- `--backend in-memory`: select the deterministic in-memory backend for tests
  and diagnostics.
- `--iterations N`: stop `watch` mode after `N` snapshots. This is intended for
  tests, scripts, and diagnostics. The value must be a positive integer. When
  omitted, `watch` keeps running.
- `--layout FILE`: select the layout JSON file for `enforce`.
- `--once`: make `enforce` apply once and exit.
- `--dry-run`: make `enforce` print one plan and exit without X11 changes.
- `--interval MS`: set the recurring `enforce` interval. The value must be a
  positive integer. The default is `1000`.

### Output Options

- `--output NAME`: select a RandR output for `place`.

### Window Selector Options

`place`, `configure`, `raise`, and `lower` require exactly one window selector:

- `--window ID`: select an X11 window ID. Hex values such as `0x800003` and
  decimal values are accepted.
- `--window-title NAME`: select a window by exact X11 window title.
- `--window-class NAME`: select a window by exact `WM_CLASS` class name.
- `--window-instance NAME`: select a window by exact `WM_CLASS` instance name.

Name selectors match exactly and only consider mapped windows. If a selector
matches no windows, the command returns `window not found: NAME`. If a selector
matches multiple windows, the command returns `window selector is ambiguous:
NAME` with candidate IDs and window metadata. In ambiguous cases, rerun the
command with `--window ID` to select a specific candidate.

### Window Options

- `--fullscreen`: place the selected window fullscreen on the selected output.

### Geometry Options

- `--x N`: set the selected window X position for `configure`. The value must
  be an integer.
- `--y N`: set the selected window Y position for `configure`. The value must
  be an integer.
- `--width N`: set the selected window width for `configure`. The value must be
  a positive integer.
- `--height N`: set the selected window height for `configure`. The value must
  be a positive integer.

## Quick Reference

### Snapshot

```text
xdisplay-ruler [snapshot] [--backend NAME]
xdisplay-ruler watch [--backend NAME] [--iterations N]
```

### Window Control

```text
xdisplay-ruler enforce --layout FILE [--once] [--dry-run] [--interval MS] [--backend x11]
xdisplay-ruler raise WINDOW_SELECTOR [--backend x11]
xdisplay-ruler lower WINDOW_SELECTOR [--backend x11]
xdisplay-ruler configure WINDOW_SELECTOR [--x N] [--y N] [--width N] [--height N] [--backend x11]
xdisplay-ruler place WINDOW_SELECTOR --output NAME --fullscreen [--backend x11]
```

### Other

```text
xdisplay-ruler --help
xdisplay-ruler --version
```

## Backend Selection

The current build supports `x11`, `xorg`, and `in-memory`. `xorg` is an alias
for the X11 backend. `x11` is the default.

The X11 backend requires a reachable Xorg server through the usual `DISPLAY`
environment. It verifies that the server provides the RANDR extension before
collecting a snapshot.

`enforce`, `place`, `configure`, `raise`, and `lower` default to the X11 backend
because they are real X11 operations. Selecting `--backend in-memory` for
`place`, `configure`, `raise`, or `lower` returns a usage error.
`enforce --dry-run --backend in-memory` can be used for deterministic planning
diagnostics, but it has no real outputs or windows unless the backend is
extended by tests.

## Command Requirements

- `place` currently requires `--fullscreen`. It uses the selected output
  geometry, configures the target window to that rectangle, and raises the
  window.
- `enforce` requires `--layout`. Without `--once` or `--dry-run`, it keeps
  running and reapplies the layout at `--interval`. `--dry-run` prints one
  operation plan and exits. See [Layout enforce](layout.md) for the JSON schema
  and selector/output error behavior.
- `configure` requires at least one of `--x`, `--y`, `--width`, or `--height`.
  It only sends the geometry fields that were provided.

## Related References

- [Output formats](output-formats.md): snapshot and dry-run reports.
- [Layout enforce](layout.md): layout JSON schema and enforce behavior.
- [Backends and monitoring](backends-and-monitoring.md): backend capabilities
  and monitor flow.

## Examples

Print one X11 snapshot:

```bash
xdisplay-ruler
```

Raise or lower a window:

```bash
xdisplay-ruler raise --window-class Gnome-terminal
xdisplay-ruler lower --window 0x800003
```

Move, resize, or place a window:

```bash
xdisplay-ruler configure --window-class Gnome-terminal --x 0 --y 0
xdisplay-ruler configure --window-class Gnome-terminal --width 480 --height 260
xdisplay-ruler place --window-class Gnome-terminal --output HDMI-2 --fullscreen
```

Dry-run a layout enforce plan:

```bash
xdisplay-ruler enforce --layout layout.json --dry-run
```

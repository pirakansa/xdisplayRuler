# Getting Started

`xdisplay-ruler` reads display and window state from a running Xorg server by
default.

## Print a Snapshot

```bash
xdisplay-ruler
```

The default command uses the X11 backend and requires a reachable Xorg server
through `DISPLAY`.

For development and diagnostics, the in-memory backend starts empty:

```text
xdisplay-ruler
backend: in-memory
outputs: 0
windows: 0
focused: none
top: none
```

The explicit snapshot command is equivalent:

```bash
xdisplay-ruler snapshot --backend x11
```

Use the in-memory backend only when you need a deterministic empty snapshot:

```bash
xdisplay-ruler snapshot --backend in-memory
```

## Watch Snapshots

Watch mode repeatedly refreshes the selected backend and prints a snapshot after
each refresh:

```bash
xdisplay-ruler watch
```

With the X11 backend, watch mode prints the initial snapshot and then waits for
RandR or root-window events before printing the next snapshot.

Use `--iterations N` only when a test, script, or diagnostic check needs watch
mode to stop after a fixed number of snapshots.

## Change an Output Mode

List the modes reported by RandR for an output:

```bash
xdisplay-ruler modes --output HDMI-2
```

The current mode is marked with `current`, and preferred modes are marked with
`preferred`.

Switch the output to one of the listed modes:

```bash
xdisplay-ruler mode --output HDMI-2 --width 1280 --height 720
```

When multiple modes share the same size, select a refresh rate:

```bash
xdisplay-ruler mode --output HDMI-2 --width 1920 --height 1080 --rate 60
```

`mode` selects from modes already reported by the Xorg RandR extension for the
output. It does not create custom modelines.

## Change Window Stacking

Use the window metadata from an X11 snapshot:

```bash
xdisplay-ruler snapshot --backend x11
```

Raise a window above its siblings:

```bash
xdisplay-ruler raise --window-class Gnome-terminal
```

Lower a window below its siblings:

```bash
xdisplay-ruler lower --window-class Gnome-terminal
```

`--window-class` uses the exact `WM_CLASS` class name printed as `class="..."`
in the snapshot. If several mapped windows have the same class, the command
prints the matching IDs so you can rerun it with `--window 0x...`.

These commands use low-level X11 stacking requests. They do not require a window
manager, but they require the target application window to accept normal X11
configuration requests.

## Move or Resize a Window

Move a window without changing its size:

```bash
xdisplay-ruler configure --window-class Gnome-terminal --x 0 --y 0
```

Resize a window without changing its position:

```bash
xdisplay-ruler configure --window-class Gnome-terminal --width 480 --height 260
```

Move and resize in one request:

```bash
xdisplay-ruler configure --window-class Gnome-terminal --x 0 --y 0 --width 480 --height 260
```

`configure` requires at least one geometry option. Width and height must be
positive integers.

## Place a Window

Use the output names from an X11 snapshot:

```bash
xdisplay-ruler snapshot
```

Place a window fullscreen on an output:

```bash
xdisplay-ruler place --window-class Gnome-terminal --output HDMI-2 --fullscreen
```

The current `place` command supports fullscreen placement only. It moves and
resizes the target window to the selected output geometry, then raises the
window.

## Command Help

```bash
xdisplay-ruler --help
```

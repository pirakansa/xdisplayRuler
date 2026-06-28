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

## Change Window Stacking

Use the window IDs from an X11 snapshot:

```bash
xdisplay-ruler snapshot --backend x11
```

Raise a window above its siblings:

```bash
xdisplay-ruler raise --window 0x800003
```

Lower a window below its siblings:

```bash
xdisplay-ruler lower --window 0x800003
```

These commands use low-level X11 stacking requests. They do not require a window
manager, but they require the target application window to accept normal X11
configuration requests.

## Place a Window

Use the output names from an X11 snapshot:

```bash
xdisplay-ruler snapshot
```

Place a window fullscreen on an output:

```bash
xdisplay-ruler place --window 0x800003 --output HDMI-2 --fullscreen
```

The current `place` command supports fullscreen placement only. It moves and
resizes the target window to the selected output geometry, then raises the
window.

## Command Help

```bash
xdisplay-ruler --help
```

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
xdisplay-ruler watch --iterations 3 --interval-ms 1000
```

With the X11 backend, watch mode prints the initial snapshot and then waits for
RandR or root-window events before printing the next snapshot.

Omit `--iterations` to keep watching until the process is stopped.

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

## Command Help

```bash
xdisplay-ruler --help
```

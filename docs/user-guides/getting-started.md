# Getting Started

`xdisplay-ruler` can print either the built-in in-memory snapshot or a snapshot
read from a running Xorg server.

## Print a Snapshot

```bash
xdisplay-ruler
```

The current default snapshot uses the in-memory backend, so it starts empty:

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
xdisplay-ruler snapshot --backend in-memory
```

Use the X11 backend to read outputs and root-level windows from the running Xorg
server:

```bash
xdisplay-ruler snapshot --backend x11
```

## Watch Snapshots

Watch mode repeatedly refreshes the selected backend and prints a snapshot after
each refresh:

```bash
xdisplay-ruler watch --iterations 3 --interval-ms 1000
```

For the current X11 backend, watch mode refreshes the initial snapshot once and
then reports no new events until X11 event subscription is implemented.

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

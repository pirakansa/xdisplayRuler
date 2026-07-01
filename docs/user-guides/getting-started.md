# Getting Started

`xdisplay-ruler` reads display and window state from a running Xorg server by
default.

## What You Can Do

- Print one display and window snapshot.
- Watch display and window changes.
- Raise, lower, move, resize, or place mapped X11 windows.
- Keep layout-defined kiosk windows fitted to output geometry.

## First Commands

Print the current X11 snapshot:

```bash
xdisplay-ruler
```

Watch display and window changes:

```bash
xdisplay-ruler watch
```

Use the in-memory backend only when you need a deterministic empty snapshot:

```bash
xdisplay-ruler snapshot --backend in-memory
```

## Command Map

- [Snapshots](snapshots.md): print, watch, and read display/window state.
- [Window control](window-control.md): select, raise, lower, move, resize, and
  place windows.
- [Layout enforcement](layout-enforcement.md): keep kiosk windows fitted to
  configured outputs.

For the exact CLI contract, see the [CLI specification](../specifications/cli.md).

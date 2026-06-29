# Window Control

Window control commands operate on mapped X11 windows selected by ID, title, or
`WM_CLASS` values.

## Window Selectors

Window control commands require exactly one selector:

```text
--window ID
--window-title NAME
--window-class NAME
--window-instance NAME
```

Use `--window-class` for common scripting cases because it uses the stable
`WM_CLASS` class name printed as `class="..."` in a snapshot. If several mapped
windows have the same selector value, the command prints the matching IDs so you
can rerun it with `--window 0x...`.

## Change Window Stacking

Raise a window above its siblings:

```bash
xdisplay-ruler raise --window-class Gnome-terminal
```

Lower a window below its siblings:

```bash
xdisplay-ruler lower --window-class Gnome-terminal
```

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

Place a window fullscreen on an output:

```bash
xdisplay-ruler place --window-class Gnome-terminal --output HDMI-2 --fullscreen
```

The current `place` command supports fullscreen placement only. It moves and
resizes the target window to the selected output geometry, then raises the
window.

For exact selector and command requirements, see the
[CLI specification](../specifications/cli.md).

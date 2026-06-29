# Getting Started

`xdisplay-ruler` reads display and window state from a running Xorg server by
default.

## What You Can Do

- Print one display and window snapshot.
- Watch display and window changes.
- List existing RandR output modes.
- Switch an output to one of its existing modes.
- Raise, lower, move, resize, or place mapped X11 windows.
- Keep layout-defined kiosk windows fitted to output geometry.

## Command Map

Snapshot commands:

```text
xdisplay-ruler
xdisplay-ruler snapshot
xdisplay-ruler watch
```

Output commands:

```text
xdisplay-ruler modes --output HDMI-2
xdisplay-ruler mode --output HDMI-2 --width 1920 --height 1080
```

Window commands:

```text
xdisplay-ruler enforce --layout layout.json
xdisplay-ruler enforce --layout layout.json --once --dry-run
xdisplay-ruler raise WINDOW_SELECTOR
xdisplay-ruler lower WINDOW_SELECTOR
xdisplay-ruler configure WINDOW_SELECTOR --x 0 --y 0
xdisplay-ruler place WINDOW_SELECTOR --output HDMI-2 --fullscreen
```

## Window Selector

Window control commands require exactly one `WINDOW_SELECTOR`:

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

## Print a Snapshot

```bash
xdisplay-ruler
```

The default command uses the X11 backend and requires a reachable Xorg server
through `DISPLAY`.

The explicit snapshot command is equivalent:

```bash
xdisplay-ruler snapshot --backend x11
```

Use the in-memory backend only when you need a deterministic empty snapshot:

```bash
xdisplay-ruler snapshot --backend in-memory
```

The in-memory backend starts empty:

```text
xdisplay-ruler
backend: in-memory
outputs: 0
windows: 0
focused: none
top: none
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

Rotate an output while keeping its current mode:

```bash
xdisplay-ruler mode --output HDMI-2 --rotate left
```

Change mode and rotation together:

```bash
xdisplay-ruler mode --output HDMI-2 --width 1920 --height 1080 --rate 60 --rotate inverted
```

For left or right rotation, pass the displayed size after rotation:

```bash
xdisplay-ruler mode --output HDMI-2 --width 1080 --height 1920 --rate 60 --rotate left
```

`mode` selects from modes already reported by the Xorg RandR extension for the
output. It does not create custom modelines. `--rotate` accepts `normal`,
`left`, `right`, or `inverted`.

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

## Enforce a Layout

Create a layout file that maps each managed app to a RandR output:

```json
{
  "schema_version": 1,
  "unmanaged_windows": "allow_above",
  "windows": [
    {
      "selector": { "app_id": "Player" },
      "output": "HDMI-2"
    },
    {
      "selector": { "app_id": "Overlay" },
      "output": "HDMI-2"
    }
  ]
}
```

Preview the planned operations:

```bash
xdisplay-ruler enforce --layout layout.json --dry-run
```

Apply once and exit:

```bash
xdisplay-ruler enforce --layout layout.json --once
```

Keep applying the layout:

```bash
xdisplay-ruler enforce --layout layout.json --interval 1000
```

Each managed window is moved and resized to the current geometry of its target
output. `app_id` matches the X11 `WM_CLASS` class name shown as `class="..."`
in snapshot output.

Use `unmanaged_windows: "allow_above"` when unknown apps are allowed to appear
above managed windows. This still keeps managed windows in layout order relative
to each other. Use `keep_below_managed` when the managed group should also be
raised above unknown windows on each enforce cycle.

## Reading Snapshot Output

Use a snapshot to discover output names and window selectors:

```bash
xdisplay-ruler snapshot --backend x11
```

Output rows show the names accepted by `--output`, such as `HDMI-2`.

Window rows can include:

- `title="..."`: value accepted by `--window-title`
- `class="..."`: value accepted by `--window-class`
- `instance="..."`: value accepted by `--window-instance`

Window IDs such as `0x800003` can always be used with `--window`. IDs are useful
when a title, class, or instance selector matches more than one mapped window.

## Command Help

```bash
xdisplay-ruler --help
```

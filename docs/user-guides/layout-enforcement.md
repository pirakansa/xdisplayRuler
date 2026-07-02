# Layout Enforcement

`xdisplay-ruler enforce` keeps layout-defined kiosk windows fitted to RandR
outputs.

## Create a Layout

Create a layout file that maps each managed app to a RandR output:

```json
{
  "schema_version": 1,
  "unmanaged_windows": "allow_above",
  "windows": [
    {
      "selector": { "class": "Player" },
      "output": "HDMI-2"
    },
    {
      "selector": { "class": "Overlay" },
      "output": "HDMI-2",
      "activate": true
    }
  ]
}
```

Each managed window is moved and resized to the current geometry of its target
output. Use `class` for the X11 `WM_CLASS` class name shown as `class="..."`
in snapshot output, or `instance` for the `WM_CLASS` instance name shown as
`instance="..."`.

Set `activate: true` on one managed window when that window should receive X11
input focus after each enforce cycle. In the example above, `Overlay` is the
active window.

## Preview and Apply

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

## Unmanaged Windows

Use `unmanaged_windows: "allow_above"` when unknown apps are allowed to appear
above managed windows. This still keeps managed windows in layout order relative
to each other.

Use `keep_below_managed` when the managed group should also be raised above
unknown windows on each enforce cycle.

For the exact schema and error behavior, see the
[Layout enforce specification](../specifications/layout.md).

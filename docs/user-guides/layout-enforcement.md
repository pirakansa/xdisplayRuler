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

Each managed window is moved and resized to the current geometry of its target
output. `app_id` matches the X11 `WM_CLASS` class name shown as `class="..."`
in snapshot output.

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

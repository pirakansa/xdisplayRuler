# Output Formats

These formats are human-readable CLI reports. They are stable enough for
diagnostics and examples, but they are not JSON APIs.

## Snapshot Output

The in-memory diagnostic snapshot is:

```text
xdisplay-ruler
backend: in-memory
outputs: 0
windows: 0
focused: none
top: none
```

The backend label is `x11` or `in-memory`.

Output rows include the output name, connection status, geometry, and an
optional `primary` marker.

Window rows include mapped state and geometry. They include `title="..."`,
`class="..."`, and `instance="..."` only when the backend reports the X11
window title or `WM_CLASS` values. Quotes, backslashes, and control characters
are escaped in these values.

## Modes Output

`modes --output NAME` prints:

```text
xdisplay-ruler
output: HDMI-2
modes: 2
- 1920x1080 60Hz name="1920x1080" current preferred
- 1280x720 59.94Hz name="1280x720"
```

Mode rows include width, height, refresh rate, RandR mode name, and optional
`current` and `preferred` markers.

## Dry-Run Enforce Output

`enforce --dry-run` prints:

```text
xdisplay-ruler enforce dry-run
operations: 1
- configure 0x20 selector=app_id:"Player" output="HDMI-2" geometry=1920x1080+0+0
```

Each operation line is the display form of a planned layout operation.

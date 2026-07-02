# Layout Enforce

`xdisplay-ruler enforce` applies a JSON layout to mapped X11 windows. It is
intended for kiosk-style Xorg environments where the tool, rather than a window
manager, keeps application windows fitted to display outputs.

## Commands

Apply once and exit:

```text
xdisplay-ruler enforce --layout layout.json --once
```

Print the planned operations without changing X11 state:

```text
xdisplay-ruler enforce --layout layout.json --dry-run
```

Keep applying the layout:

```text
xdisplay-ruler enforce --layout layout.json
```

`--interval MS` sets the recurring apply interval in milliseconds. The value
must be a positive integer. The default is `1000`.

`--dry-run` prints one plan and exits. If `--dry-run` and `--once` are both
provided, dry-run behavior takes precedence.

## Implementation Layout

- `src/layout/mod.rs`: public layout API surface and re-exports.
- `src/layout/policy.rs`: JSON schema types, selector parsing, and validation errors.
- `src/layout/planner.rs`: selector/output resolution and enforce operation planning.
- `src/enforce.rs`: enforce command orchestration for dry-run, once, and recurring modes.
- `src/enforce/planner.rs`: layout loading, backend snapshot refresh, and plan construction.
- `src/enforce/executor.rs`: planned operation execution against the selected backend.
- `src/enforce/report.rs`: warnings and dry-run report rendering.

## Layout Schema

The root object fields are:

- `schema_version`: required. The only supported value is `1`.
- `unmanaged_windows`: optional. Defaults to `allow_above`.
- `windows`: required array of managed window rules. Array order is bottom to
  top for managed stacking operations.

Unknown fields are rejected at the root, window rule, and selector levels.

Example:

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

Each window rule has:

- `selector`: required window selector.
- `output`: required RandR output name.
- `activate`: optional boolean. Defaults to `false`. The one rule with
  `activate: true` becomes the active X11 input focus target after geometry and
  stacking operations are planned.

Rules do not accept `geometry` or `placement`. A managed window is always fitted
to the current geometry of its target output.

At most one window rule can set `activate: true`.

## Selectors

A selector must contain exactly one of:

- `id`: X11 window ID string, such as `"0x800003"` or `"8388611"`.
- `title`: exact X11 window title.
- `class`: exact `WM_CLASS` class name.
- `instance`: exact `WM_CLASS` instance name.

Partial matches, regular expressions, prefixes, and multi-field selectors are
not supported.

## Unmanaged Windows

Unmanaged windows are mapped windows that do not match any layout rule.

Supported policies:

- `allow_above`: default. The enforce plan maintains managed window geometry
  and corrects the relative stacking order of managed windows with sibling
  stack operations. It does not raise the managed group over unknown windows.
- `keep_below_managed`: raises managed windows in layout array order after
  geometry changes, placing later rules above earlier rules.

The implementation does not kill, unmap, or hide unmanaged windows.

## Apply Flow

Each enforce cycle:

1. Refreshes the current display state from the selected backend.
2. Resolves each selector against mapped windows.
3. Resolves each rule output against connected outputs.
4. Plans a `ConfigureWindow` operation when the current window geometry differs
   from the output geometry.
5. For `allow_above`, plans sibling stack operations when managed windows are
   not in layout order.
6. For `keep_below_managed`, adds `RaiseWindow` operations in layout order.
7. Adds an `ActivateWindow` operation for the resolved rule with
   `activate: true`, if one is configured.
8. Applies the planned operations unless `--dry-run` was used.

## Error Handling

Layout read errors, invalid JSON, unsupported schema versions, missing required
fields, unknown fields, invalid selector shapes, multiple `activate: true`
rules, and invalid option values are usage errors.

In `--once`, `--dry-run`, and recurring mode:

- Unresolved selectors and outputs are warnings.
- Ambiguous selectors are warnings.
- The affected rule is skipped for that cycle and retried on the next cycle.

Recurring mode prints a warning when it first appears, then suppresses it while
it remains unchanged. If a warning disappears and later appears again, it is
printed again.

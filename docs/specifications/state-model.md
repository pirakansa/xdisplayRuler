# State Model

The state model is implemented in `src/models/` and `src/state.rs`.

## Geometry

`Rect` stores X11-style geometry:

- `x`
- `y`
- `width`
- `height`

The display format is `WIDTHxHEIGHT+X+Y`.

## Outputs

`DisplayOutput` stores:

- output name
- geometry
- primary flag
- connected flag

Connecting a primary output clears the primary flag from other outputs.
Disconnecting an output keeps the output in the state, marks it disconnected,
and clears its primary flag.

## Windows

`WindowInfo` stores:

- X11 window ID
- optional title
- optional application ID
- geometry
- mapped flag

Mapped windows can be raised into the stacking order. Unmapping a window removes
it from the stacking order and clears focus when that window was focused.

## Events

`DisplayEvent` is the input boundary for future backends. Implemented events
cover output connection changes, output geometry changes, window map and unmap,
window geometry changes, window raises, and focus changes.

Focus is accepted only for mapped windows.

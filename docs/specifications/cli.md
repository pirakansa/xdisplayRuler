# CLI

The binary is named `xdisplay-ruler`.

## Arguments

- No arguments: run the default `snapshot` command with the X11 backend.
- `snapshot`: print the current display snapshot once.
- `watch`: keep refreshing and printing display snapshots.
- `raise`: raise an X11 window above its siblings.
- `lower`: lower an X11 window below its siblings.
- `--backend x11` or `--backend xorg`: select the X11/RandR backend.
- `--backend in-memory`: select the deterministic in-memory backend for tests
  and diagnostics.
- `--interval-ms MS`: set the delay between `watch` refreshes. The value must
  be a positive integer. The default is `1000`.
- `--iterations N`: stop `watch` mode after `N` refreshes. The value must be a
  positive integer. When omitted, `watch` keeps running.
- `--window ID`: select an X11 window ID for `raise` or `lower`. Hex values such
  as `0x800003` and decimal values are accepted.
- `--help` or `-h`: print command help.
- `--version` or `-V`: print the package version.
- Any unsupported command, argument, backend, or option value: print an error to
  standard error and exit with status `2`.

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

## Backend Selection

The current build supports `x11`, `xorg`, and `in-memory`. `xorg` is an alias
for the X11 backend. `x11` is the default.

The X11 backend requires a reachable Xorg server through the usual `DISPLAY`
environment. It verifies that the server provides the RANDR extension before
collecting a snapshot.

`raise` and `lower` default to the X11 backend because they are real X11 window
operations. Selecting `--backend in-memory` for those commands returns a usage
error.

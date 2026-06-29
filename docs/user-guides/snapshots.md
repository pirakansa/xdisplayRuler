# Snapshots

Use snapshots to discover output names, window IDs, and selector values for
other commands.

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

## Read Snapshot Output

Output rows show the names accepted by `--output`, such as `HDMI-2`.

Window rows can include:

- `title="..."`: value accepted by `--window-title`
- `class="..."`: value accepted by `--window-class`
- `instance="..."`: value accepted by `--window-instance`

Window IDs such as `0x800003` can always be used with `--window`. IDs are useful
when a title, class, or instance selector matches more than one mapped window.

For the exact output contract, see
[Output formats](../specifications/output-formats.md).

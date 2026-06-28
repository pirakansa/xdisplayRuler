# CLI

The binary is named `display-ruler`.

## Arguments

- No arguments: run the default `snapshot` command.
- `snapshot`: print the current display snapshot once.
- `watch`: keep refreshing and printing display snapshots.
- `--backend in-memory`: select the in-memory backend.
- `--interval-ms MS`: set the delay between `watch` refreshes. The value must
  be a positive integer. The default is `1000`.
- `--iterations N`: stop `watch` mode after `N` refreshes. The value must be a
  positive integer. When omitted, `watch` keeps running.
- `--help` or `-h`: print command help.
- `--version` or `-V`: print the package version.
- Any unsupported command, argument, backend, or option value: print an error to
  standard error and exit with status `2`.

## Snapshot Output

The current default snapshot is:

```text
display-ruler
backend: in-memory
outputs: 0
windows: 0
focused: none
top: none
```

The backend label is currently fixed to `in-memory`.

## Backend Selection

Only `in-memory` is supported in the current build. Selecting `x11` or `xorg`
returns a usage error because the Xorg backend is not implemented yet.

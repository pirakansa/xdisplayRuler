# CLI

The binary is named `display-ruler`.

## Arguments

- No arguments: print the current in-memory display snapshot.
- `--help` or `-h`: print command help.
- `--version` or `-V`: print the package version.
- Any other argument: print an error to standard error and exit with status `2`.

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

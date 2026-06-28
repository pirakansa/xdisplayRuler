# Getting Started

`display-ruler` currently starts without connecting to an Xorg server. It prints
the in-memory display snapshot managed by the core state engine.

## Print a Snapshot

```bash
display-ruler
```

The current default snapshot is empty because the Xorg/XRandR event backend is
not implemented yet:

```text
display-ruler
backend: in-memory
outputs: 0
windows: 0
focused: none
top: none
```

The explicit snapshot command is equivalent:

```bash
display-ruler snapshot --backend in-memory
```

## Watch Snapshots

Watch mode repeatedly refreshes the selected backend and prints a snapshot after
each refresh:

```bash
display-ruler watch --iterations 3 --interval-ms 1000
```

Omit `--iterations` to keep watching until the process is stopped.

## Command Help

```bash
display-ruler --help
```

# Getting Started

`display-ruler` currently starts without connecting to an Xorg server. It prints
the in-memory display snapshot managed by the core state engine.

## Build

```bash
vorbere run build
```

## Print a Snapshot

```bash
vorbere run run
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

## Command Help

```bash
cargo run -- --help
```

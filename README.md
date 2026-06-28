# xdisplayRuler

xdisplayRuler is a Rust CLI foundation for tracking Xorg display and window
state on kiosk-style Linux systems.

The current build provides the core state engine, backend event boundary,
monitor flow, CLI snapshot output, and an X11/RandR snapshot backend.

## Quick Start

Print a snapshot from the running Xorg server:

```bash
xdisplay-ruler
```

Watch display and window changes:

```bash
xdisplay-ruler watch
```

The explicit X11 backend form is equivalent:

```bash
xdisplay-ruler snapshot --backend x11
```

Raise a window from the X11 snapshot:

```bash
xdisplay-ruler raise --window 0x800003
```

Place a window fullscreen on an output:

```bash
xdisplay-ruler place --window 0x800003 --output HDMI-2 --fullscreen
```

Move or resize a window:

```bash
xdisplay-ruler configure --window 0x800003 --x 0 --y 0 --width 480 --height 260
```

## Development Setup

- Rust stable
- `vorbere` for task shortcuts

Build the project once after cloning:

```bash
vorbere run build
```

## Development Commands

- Run: `vorbere run run`
- Check: `vorbere run check`
- Test: `vorbere run test`
- Build: `vorbere run build`
- Linux amd64 musl release: `vorbere run release-linux-amd64-musl`
- Linux arm64 musl release: `vorbere run release-linux-arm64-musl`

## Documentation

- [User guides](docs/user-guides/README.md): practical command usage
- [Specification references](docs/specifications/README.md): implemented CLI,
  model, and state behavior

## Project Structure

- `src/cli.rs`: CLI argument handling
- `src/lib.rs`: public module exports
- `src/main.rs`: binary entry point
- `src/models/`: display and window data types
- `src/state.rs`: display state reducer and reporting
- `docs/`: user guides and specification references
- `tests/`: smoke tests for the compiled binary
- `vorbere.yaml`: local development tasks

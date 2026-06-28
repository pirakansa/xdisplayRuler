# displayRuler

displayRuler is a Rust CLI foundation for tracking Xorg display and window
state on kiosk-style Linux systems.

The current build provides the core state engine, backend event boundary,
monitor flow, CLI snapshot output, and an X11/RandR snapshot backend.

## Quick Start

Print the current in-memory snapshot with the released binary:

```bash
display-ruler
```

Run bounded watch mode:

```bash
display-ruler watch --iterations 3 --interval-ms 1000
```

Read a snapshot from the running Xorg server:

```bash
display-ruler snapshot --backend x11
```

Raise a window from the X11 snapshot:

```bash
display-ruler raise --window 0x800003
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

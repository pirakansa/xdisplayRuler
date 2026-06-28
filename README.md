# displayRuler

displayRuler is a Rust CLI foundation for tracking Xorg display and window
state on kiosk-style Linux systems.

The current build provides the core in-memory state engine, backend event
boundary, monitor flow, and CLI snapshot output. The Xorg/XRandR event backend
is not implemented yet.

## Setup

- Rust stable
- `vorbere` for task shortcuts

Build the project once after cloning:

```bash
vorbere run build
```

## Quick Start

Print the current in-memory snapshot:

```bash
vorbere run run
```

## Common Commands

- Run: `vorbere run run`
- Check: `vorbere run check`
- Test: `vorbere run test`
- Build: `vorbere run build`

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

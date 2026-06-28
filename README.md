# displayRuler

displayRuler is a Rust CLI foundation for tracking Xorg display and window
state on kiosk-style Linux systems.

The current build provides the core in-memory state engine for connected
outputs, output geometry, mapped windows, stacking order, and focused windows.
The Xorg/XRandR event backend is not implemented yet.

## Setup

- Rust stable
- `vorbere` for task shortcuts

Build the project once after cloning:

```bash
vorbere run build
```

## Quick Start

Print the current built-in snapshot:

```bash
vorbere run run
```

Expected output for the current in-memory backend:

```text
display-ruler
backend: in-memory
outputs: 0
windows: 0
focused: none
top: none
```

## Common Commands

- Run: `vorbere run run`
- Check: `vorbere run check`
- Test: `vorbere run test`
- Build: `vorbere run build`

## Project Structure

- `src/lib.rs`: display state model and event reducer
- `src/main.rs`: CLI entry point
- `tests/`: smoke tests for the compiled binary
- `vorbere.yaml`: local development tasks

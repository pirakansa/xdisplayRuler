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

List output modes and switch to an existing mode:

```bash
xdisplay-ruler modes --output HDMI-2
xdisplay-ruler mode --output HDMI-2 --width 1280 --height 720 --rate 60
```

The explicit X11 backend form is equivalent:

```bash
xdisplay-ruler snapshot --backend x11
```

Raise a window by `WM_CLASS` class name:

```bash
xdisplay-ruler raise --window-class Gnome-terminal
```

Place a window fullscreen on an output:

```bash
xdisplay-ruler place --window-class Gnome-terminal --output HDMI-2 --fullscreen
```

Preview or run a kiosk layout:

```bash
xdisplay-ruler enforce --layout layout.json --dry-run
xdisplay-ruler enforce --layout layout.json
```

Move or resize a window:

```bash
xdisplay-ruler configure --window-class Gnome-terminal --x 0 --y 0 --width 480 --height 260
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

- `src/cli/mod.rs`: CLI entrypoint and exit mapping
- `src/cli/help.rs`: static help text
- `src/cli/options.rs`: option parsing and value validation
- `src/cli/command.rs`: backend-facing command execution
- `src/cli/report.rs`: user-facing text rendering for CLI reports
- `src/layout/policy.rs`: layout schema, selectors, and validation errors
- `src/layout/planner.rs`: enforce planning and stacking policy decisions
- `src/backend/x11/snapshot.rs`: X11 snapshot collection, window discovery, and event waiting
- `src/backend/x11/control.rs`: RandR mode changes and X11 window control operations
- `src/lib.rs`: public module exports
- `src/main.rs`: binary entry point
- `src/models/`: display and window data types
- `src/state.rs`: display state reducer and reporting
- `docs/user-guides/`: task-oriented usage guides
- `docs/specifications/`: behavior and contract references
- `tests/`: smoke tests for the compiled binary
- `vorbere.yaml`: local development tasks

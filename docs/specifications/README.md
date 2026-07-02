# Specification References

These references describe implemented behavior that should remain stable as the
project evolves. Code and passing tests are the source of truth.

## Implemented Scope

- In-memory display state model
- Backend event source boundary
- Monitor flow that polls a backend and applies events to display state
- X11/RandR snapshot backend for outputs and root-level windows
- X11/RandR and root-window event subscription for watch mode
- X11 fullscreen placement command
- X11 layout enforce command for fitting managed windows to output geometry
- X11 window activation command
- X11 window raise and lower commands
- X11 window move and resize command
- Display output connection, disconnection, geometry, and primary-output state
- Window mapping, unmapping, geometry, stacking order, and focus state
- CLI snapshot, watch, backend selection, help, version, and usage-error handling
- Layout JSON parsing, validation, selector resolution, and dry-run planning

## Out of Scope

- Custom RandR modeline creation
- Arbitrary per-window layout geometry
- Persistent configuration

## References

- [CLI](cli.md): command-line arguments, output, and exit behavior.
- [Output formats](output-formats.md): snapshot and dry-run reports.
- [Layout enforce](layout.md): layout JSON schema and enforce behavior.
- [Backends and monitoring](backends-and-monitoring.md): event source and
  monitor responsibilities.
- [State model](state-model.md): display, window, event, and reducer behavior.

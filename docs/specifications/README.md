# Specification References

These references describe implemented behavior that should remain stable as the
project evolves. Code and passing tests are the source of truth.

## Implemented Scope

- In-memory display state model
- Backend event source boundary
- Monitor flow that polls a backend and applies events to display state
- X11/RandR snapshot backend for outputs and root-level windows
- X11/RandR and root-window event subscription for watch mode
- X11/RandR output mode listing and mode switching
- X11 fullscreen placement command
- X11 window raise and lower commands
- X11 window move and resize command
- Display output connection, disconnection, geometry, and primary-output state
- Window mapping, unmapping, geometry, stacking order, and focus state
- CLI snapshot, watch, backend selection, help, version, and usage-error handling

## Out of Scope

- Custom RandR modeline creation
- Window focus commands
- Persistent configuration

## References

- [CLI](cli.md): command-line arguments, output, and exit behavior.
- [Backends and monitoring](backends-and-monitoring.md): event source and
  monitor responsibilities.
- [State model](state-model.md): display, window, event, and reducer behavior.

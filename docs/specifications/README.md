# Specification References

These references describe implemented behavior that should remain stable as the
project evolves. Code and passing tests are the source of truth.

## Implemented Scope

- In-memory display state model
- Backend event source boundary
- Monitor flow that polls a backend and applies events to display state
- Display output connection, disconnection, geometry, and primary-output state
- Window mapping, unmapping, geometry, stacking order, and focus state
- CLI snapshot, watch, backend selection, help, version, and usage-error handling

## Out of Scope

- Xorg server connection
- XRandR event collection
- X11 window tree inspection
- Display mode switching
- Persistent configuration

## References

- [CLI](cli.md): command-line arguments, output, and exit behavior.
- [Backends and monitoring](backends-and-monitoring.md): event source and
  monitor responsibilities.
- [State model](state-model.md): display, window, event, and reducer behavior.

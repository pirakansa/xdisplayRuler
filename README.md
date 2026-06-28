# codex-sandbox

This repository is a minimal Rust template that prints Hello, world!.
It keeps the project structure intentionally small so it can be used as a clean starting point.

## Setup

- Rust stable
- `vorbere` for task shortcuts

Build the project once after cloning:

```bash
vorbere run build
```

## Common Commands

- Run: `vorbere run run`
- Test: `vorbere run test`
- Format: `vorbere run fmt`
- Lint: `vorbere run clippy`
- CI-equivalent checks: `vorbere run ci`

## Project Structure

- `src/main.rs`: prints Hello, world!
- `tests/`: smoke test for the compiled binary
- `vorbere.yaml`: local development tasks

## Expected Output

```text
Hello, world!
```

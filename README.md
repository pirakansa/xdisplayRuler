# displayRuler

This repository provides a small Rust CLI that prints a terminal display ruler.
It is useful when checking text width, alignment, and wrapping behavior in a terminal.

## Setup

- Rust stable
- `vorbere` for task shortcuts

Build the project once after cloning:

```bash
vorbere run build
```

## Common Commands

- Run: `vorbere run run`
- Check: `vorbere run check`
- Test: `vorbere run test`
- Build: `vorbere run build`

## Project Structure

- `src/lib.rs`: ruler generation logic
- `src/main.rs`: CLI entry point
- `tests/`: smoke test for the compiled binary
- `vorbere.yaml`: local development tasks

## Expected Output

```text
0         1         2         3         4         5         6         7
12345678901234567890123456789012345678901234567890123456789012345678901234567890
```

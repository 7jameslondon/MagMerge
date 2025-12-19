# MagMerge

MagMerge combines multiple `Bead Positions` and `Motor Positions` `.txt` files into two outputs.
Drag a folder onto the GUI or use the CLI for batch runs.

## Quick Start
- GUI: `cargo run --bin MagMerge`
- CLI: `cargo run --bin magmerge_cli -- <folder>`

## Outputs
- `Bead Positions Combined.txt`
- `Motor Positions Combined.txt`

## Build and Test
- Tests: `cargo test`
- Release build: `cargo build --release`

## Distribution
The Windows release binaries are copied to `dist/`:
- `dist/MagMerge.exe`
- `dist/magmerge_cli.exe`

## Samples
Sample input files live in `samples/` (not tracked in git). See `samples/README.md`.

## Tooling
Optional `just` tasks are defined in `justfile`:
- `just test`
- `just build`
- `just dist`

# MagMerge - Agent Notes

## Overview
Rust desktop app that combines multiple `.txt` files into two outputs:
- All "Bead Positions" files -> `Bead Positions Combined.txt`
- All "Motor Positions" files -> `Motor Positions Combined.txt`

Users drag a folder onto the GUI; a CLI is also available for automation/testing.

## Current Behavior
- Scans a single folder (non-recursive) for `.txt` files.
- Groups by filename prefix: `Bead Positions` and `Motor Positions`.
- Sorts filenames lexicographically for deterministic output.
- Uses the first header line (first non-empty line starting with `#`) as the output header.
- Skips empty lines; writes data lines with `\n` line endings.
- If headers differ across files in a group, data is still included and a warning is emitted.
- If no files are found for a type, that output is not created.
- Read failures are reported per file; write failures stop that output.

## GUI
- Built with `eframe`/`egui`.
- Drag-and-drop folder input.
- Immediate status message on drop, plus progress bar while processing.
- Shows counts, output paths, warnings, and errors.
- Windows console hidden in release builds via:
  - `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`
- Window/taskbar icon uses `assets/logo.png` at runtime.

## CLI
- Binary: `magmerge_cli`
- Usage: `magmerge_cli <folder>`
- Prints summary, warnings, and errors.

## Project Structure
- `src/lib.rs`: core discovery + combine logic, warnings/errors, progress reporting.
- `src/cli.rs`: reusable CLI runner (used by binary and tests).
- `src/bin/magmerge_cli.rs`: thin wrapper around `cli::run_cli`.
- `src/bin/magmerge_gui.rs`: GUI app.
- `assets/logo.png`: app logo (runtime + build-time icon).
- `samples/`: sample input files for manual testing.

## Build and Test
- Tests: `cargo test`
- CLI: `cargo run --bin magmerge_cli -- <folder>`
- GUI: `cargo run --bin MagMerge`

## Distribution
- Release build: `cargo build --release`
- Binaries copied to `dist/`:
  - `dist/MagMerge.exe`
  - `dist/magmerge_cli.exe`

## Windows EXE Icon (File Explorer)
- `build.rs` generates a multi-size `.ico` from `assets/logo.png` and embeds it.
- Uses build deps: `image`, `ico`, `embed-resource`.
- The icon is generated into `OUT_DIR` on Windows builds only.

## Notes for Changes
- Keep file grouping based on filename prefix only.
- Progress updates are file-level, not line-level.
- Output files are overwritten each run for the detected types.
- Make frequent commits after any feature is changed or added.

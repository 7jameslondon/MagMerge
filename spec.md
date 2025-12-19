# MagScope File Combiner - Specification

## Overview
Build a Rust desktop GUI app that combines multiple simple `.txt` files into two output files:
- All "Bead Positions" files are concatenated into a single combined bead file.
- All "Motor Positions" files are concatenated into a single combined motor file.

The user experience is a single-window app where the user drags a folder onto the window to start combining.

## Inputs
### Supported files
- Text files with names that start with either:
  - `Bead Positions`
  - `Motor Positions`
- Example filenames in `samples/`:
  - `samples/Bead Positions 2025-06-04 13-15-12.875.txt`
  - `samples/Motor Positions 2025-06-04 13-03-09.934.txt`

### File format (observed)
- Bead positions:
  - Header line starts with `#`, e.g. `# Time(sec) X(nm) Y(nm) Z(nm) Bead-ID`
  - Remaining lines are whitespace-delimited numeric values.
- Motor positions:
  - Header line starts with `#`, e.g. `# Objective(nm), Linear(mm), Rotary (turns) for each: time(sec), position, target, is-moving(bool)`
  - Remaining lines are whitespace-delimited numeric values.

### Folder selection
- User drags a folder onto the app window.
- The app scans only that folder (no recursion).
- Only `.txt` files are considered.

## Output
### Output files
Write two combined files into the same folder:
- `Bead Positions Combined.txt`
- `Motor Positions Combined.txt`

### Combine rules
- Group input files by type (bead vs motor) based on filename prefix.
- For each group:
  - Sort files by filename (lexicographic) for deterministic ordering.
  - Use the first file's header line (first non-empty line that starts with `#`).
  - For every file in the group, append all non-empty data lines, skipping any header line.
- Result files contain:
  - A single header line at the top.
  - All data lines in sorted filename order.

### Header mismatch handling
- If a file in the group has a header line that differs from the first file's header:
  - Still include the data lines.
  - Report a warning in the UI (filename + mismatch notice).

## UI Requirements
### Window
- Single window titled "MagScope File Combiner".
- Drag-and-drop enabled for folders.

### Layout
- Drop area with instructions: "Drop a folder here to combine Bead and Motor files."
- Status area that shows:
  - Selected folder path.
  - Counts of bead and motor files detected.
  - Output file paths.
  - Warnings (e.g., header mismatch, no files found).
  - Errors (e.g., unreadable files, write failures).

### Behavior
- On successful combine, show a success message and counts of combined lines.
- If no matching files are found, show a clear message and do not create outputs.
- If only one type is found, only create that output file and report the missing type.

## Error Handling
- If a file cannot be read, skip it and report the error.
- If output file cannot be created or written, show an error and stop processing that type.
- Avoid overwriting if outputs are open/locked; report the error.
- Do not crash on malformed lines; treat them as data lines and include them as-is.

## Non-goals
- No file content parsing or validation beyond header detection.
- No recursive folder scanning.
- No cloud or network features.
- No advanced command-line UI beyond a simple folder argument.

## Implementation Notes (Rust)
- Use Rust 2021 edition.
- Suggested GUI: `eframe`/`egui` for drag-and-drop support.
- Read files as UTF-8 with fallback to raw bytes if necessary; preserve lines as-is.
- Use buffered I/O for performance on large files.
- Keep behavior deterministic (sorted filenames).

## Acceptance Criteria
- Dragging the `samples/` folder produces:
  - `Bead Positions Combined.txt` with one header and all bead lines.
  - `Motor Positions Combined.txt` with one header and all motor lines.
- The UI reports counts and any header mismatches.
- The app handles missing or unreadable files without crashing.

## Development Tasks (in order)
1. Initialize a git repository and commit frequently while completing each task.
2. Initialize Rust workspace and crate layout, including a shared library for combine logic and one binary target.
3. Implement file discovery:
   - Scan a single folder for `.txt` files.
   - Split into bead vs motor groups by filename prefix.
   - Sort filenames deterministically.
4. Implement core combine logic in the library:
   - Read header and data lines.
   - Skip empty lines.
   - Aggregate per group with a single header and warnings on mismatches.
5. Add unit tests for the library:
   - Fixture-based tests for header selection, data concatenation, and ordering.
   - Tests for header mismatch warnings.
   - Tests for empty or missing groups.
6. Build a simple CLI (early deliverable):
   - Accept a folder path argument.
   - Run combine logic and print a concise summary (counts, outputs, warnings).
7. Harden error handling:
   - Surface read/write errors per file.
   - Ensure partial failures do not crash the app.
8. Integrate output writing:
   - Write combined files to the selected folder.
   - Skip creating outputs when no matching files exist.
9. Add integration tests for CLI behavior (happy path and missing files).
10. Build the GUI last:
   - Drag-and-drop folder input.
   - Status area with counts, outputs, warnings, and errors.
   - Reuse the shared combine logic and output writing.

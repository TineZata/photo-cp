# Photo Organizer & Copy Tool

A Rust-based TUI (Terminal User Interface) application for copying and organising photos from an unorganised source folder to an organised destination folder.

## Features

- **Terminal User Interface**: Clean and interactive TUI built with ratatui
- **Folder Selection**: Easy source and destination folder selection
- **Organization Options**:
  - Flat copy (no hierarchy)
  - Year/Month hierarchy (automatically extracts date from EXIF data or file metadata)
- **Progress Tracking**: Real-time progress display with file count and current file being processed
- **EXIF Support**: Reads photo capture dates from EXIF metadata
- **Multiple Photo Formats**: Supports JPG, PNG, GIF, BMP, TIFF, HEIC, RAW, CR2, NEF, ARW, and more
- **Duplicate Handling**: Automatically renames files if they already exist at the destination
- **Error Reporting**: Displays errors for files that couldn't be copied

## Installation

Make sure you have Rust installed. Then build the project:

```bash
cargo build --release
```

## Usage

Run the application:

```bash
cargo run --release
```

Or run the compiled binary:

```bash
./target/release/photo-cp
```

### Instructions

1. **Enter Source Folder**: Type the path to your unorganized photo folder and press Enter
2. **Enter Destination Folder**: Type the path to your destination folder and press Enter
3. **Toggle Organization**: Press `Ctrl+H` to toggle between flat copy and year/month hierarchy
4. **Start Copying**: Press `s` to start the copy process
5. **View Progress**: Watch the progress bar and current file being processed
6. **Completion**: When done, press `r` to restart or `q` to quit

### Keyboard Shortcuts

- `Enter`: Submit current input
- `Ctrl+H`: Toggle organization mode (flat vs year/month hierarchy)
- `s`: Start copying photos (when both folders are set)
- `r`: Restart (after completion)
- `q` or `ESC`: Quit the application

## Organization Modes

### Flat Copy
Copies all photos to the destination folder without any organization.

### Year/Month Hierarchy
Organizes photos into a hierarchical structure like:
```
destination/
├── 2023/
│   ├── 01-January/
│   │   ├── photo1.jpg
│   │   └── photo2.jpg
│   └── 12-December/
│       └── photo3.jpg
└── 2024/
    └── 06-June/
        └── photo4.jpg
```

Photos without EXIF date information are placed in an "Unknown" folder.

## Dependencies

- `ratatui`: Terminal UI framework
- `crossterm`: Cross-platform terminal manipulation
- `walkdir`: Recursive directory traversal
- `exif`: EXIF metadata extraction
- `chrono`: Date and time handling

## License

MIT

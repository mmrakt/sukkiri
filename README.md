# sukkiri 🧹

A lightweight TUI disk cleanup tool for macOS, written in Rust.
It helps you reclaim disk space by clearing caches, logs, and development-related junk files (Xcode, Docker, node_modules, etc.).

## Installation

You can install `sukkiri` directly from [crates.io](https://crates.io/crates/sukkiri):

```bash
cargo install sukkiri
```

## Features

- **Fast Scanning**: Multi-threaded scanning of system and user caches.
- **Developer Focused**: Targets `node_modules`, Xcode `DerivedData`, Docker images, and more.
- **Safe by Default**: Moves files to the system Trash instead of permanent deletion.
- **Interactive TUI**: Visual dashboard with pie charts and detailed file lists.

## Usage

Simply run:
```bash
sukkiri
```

### Options
- `-h`, `--help`: Show help information
- `-v`, `--version`: Show version information

### Keybindings
- `j` / `Down`: Move down
- `k` / `Up`: Move up
- `Space`: Toggle selection
- `a`: Toggle all
- `Enter`: Proceed to clean selected items
- `q`: Quit

## License
MIT

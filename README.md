# OmaDo

A fast, minimal todo application built specifically for Omarchy. Both GUI and CLI interfaces. Built with Rust and egui, featuring project organization, theme integration, and a clean aesthetic.

![OmaDo Screenshot](screenshot.png) <!-- You can add a screenshot later -->

## Features

- **Dual Interface**: Both GUI and command-line interfaces
- **Project Organization**: Group tasks with `project:` prefixes
- **Theme Integration**: Automatically syncs with Alacritty terminal themes
- **Keyboard-Driven**: Vim-inspired navigation (j/k, dd, etc.)
- **Fast & Lightweight**: Native Rust performance
- **Cross-Platform**: Works on Linux, macOS, and Windows

## Installation

### Option 1: Install from Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/your-username/omado
cd omado

# Install with Cargo
cargo install --path .

# The binary will be installed to ~/.cargo/bin/omado
# Make sure ~/.cargo/bin is in your PATH
```

### Option 2: Build Manually

```bash
# Clone and build
git clone https://github.com/your-username/omado
cd omado
cargo build --release

# The binary will be at target/release/omado
# Copy it to a directory in your PATH, e.g.:
sudo cp target/release/omado /usr/local/bin/
```

### Requirements

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))
- Linux: `libxcb`, `libxrandr`, `libxi` development packages

## Usage

### GUI Mode (Default)

Launch the GUI application:

```bash
omado
```

#### GUI Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `a` | Add new task |
| `Enter` | Edit selected task |
| `x` | Toggle task completion |
| `dd` | Delete task (press twice) |
| `f` | Cycle filter (All → Active → Done) |
| `p` | Cycle project filter |
| `c` | Clear all filters |
| `Shift+S` | Toggle search |
| `Shift+P` | Open project palette |
| `Escape` | Cancel/Clear |
| `g` / `Shift+G` | Go to top/bottom |

### CLI Mode

Add tasks from the command line:

```bash
# Add a simple task
omado add "Buy groceries"

# Add a task with a project
omado add "work: Fix parser bug"
omado add "personal: Call mom"

# Get help
omado help
```

### Project Organization

Tasks can be organized into projects using the `project:` syntax:

- `work: Fix the login bug` - Assigned to "work" project
- `personal: Book dentist appointment` - Assigned to "personal" project
- `Buy milk` - No project assigned

#### Project Features

- **Color-coded**: Each project gets a unique theme-based color
- **Filtering**: Press `p` to cycle through projects or `Shift+P` for project palette
- **Statistics**: View task counts per project in the palette

### File Storage

Tasks are stored in a plain text file at:

- **Linux/macOS**: `~/.local/share/omado/todo.txt`

The format is compatible with standard todo.txt syntax:

```
[ ] Buy groceries
[x] work: Fix parser bug
[ ] personal: Call mom
```

### Theme Integration

OmaDo automatically syncs with your Alacritty terminal theme by reading:

- **Main config**: `~/.config/alacritty/alacritty.toml`
- **Imported themes**: Supports Alacritty's `import` feature

Colors update in real-time when you change your terminal theme.

## Configuration

Currently, OmaDo works out of the box with no configuration required. It automatically:

- Creates the storage directory and file
- Detects your Alacritty theme
- Maintains state between GUI and CLI usage

## Development

### Building from Source

```bash
# Clone the repository
git clone https://github.com/your-username/omado
cd omado

# Run in development mode
cargo run                          # Launch GUI
cargo run -- add "Test task"      # Test CLI

# Build release version
cargo build --release

# Run tests (if any are added)
cargo test
```

### Dependencies

- [egui](https://github.com/emilk/egui) - Immediate mode GUI
- [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) - GUI framework
- [serde](https://serde.rs/) - Serialization
- [toml](https://github.com/toml-rs/toml) - TOML parsing for themes
- [dirs](https://github.com/dirs-dev/dirs-rs) - Cross-platform directories
- [shellexpand](https://github.com/netvl/shellexpand) - Shell path expansion
- [notify](https://github.com/notify-rs/notify) - File watching
- [anyhow](https://github.com/dtolnay/anyhow) - Error handling

## Contributing

Contributions are welcome! Please feel free to submit issues, feature requests, or pull requests.

### Areas for Contribution

- [ ] Add more keyboard shortcuts
- [ ] Implement task due dates
- [ ] Add task priorities
- [ ] Create additional themes
- [ ] Add task export formats
- [ ] Add more Linux desktop environment integrations
- [ ] Add configuration file support
- [ ] Improve performance optimizations


## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Changelog

### v0.1.0 (Initial Release)

- GUI and CLI interfaces
- Project organization with `project:` syntax
- Alacritty theme integration
- Keyboard-driven navigation
- Real-time theme updates

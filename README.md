# F1 - Terminal Text Editor

A terminal text editor without learning curve

## Features

- 📁 Multiple tabs with file management
- 🖱️ Full mouse support (click, drag, scroll)
- ⌨️ VS Code-style keyboard shortcuts
- 🔍 Fuzzy file finder
- 💾 Unsaved changes warnings
- 🎨 Syntax-aware text selection
- 📜 Smooth scrolling

## Installation

### Prerequisites

- Rust 1.70 or higher
- Terminal with mouse support

### Quick Install

**macOS/Linux:**
```bash
curl -sSL https://github.com/rizrmd/f1/releases/latest/download/install.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://github.com/rizrmd/f1/releases/latest/download/install.ps1 | iex
```

**Install from Source (All Platforms):**
```bash
cargo install --git https://github.com/rizrmd/f1
```

### Download Binaries

Pre-built binaries are available for:
- Linux (x86_64, ARM64)
- macOS (Intel, Apple Silicon)  
- Windows (x86_64)

Download from [Releases](https://github.com/rizrmd/f1/releases)

### Build from Source

```bash
git clone https://github.com/rizrmd/f1
cd f1
cargo build --release
sudo mv target/release/f1 /usr/local/bin/
```

## Terminal Setup

### macOS Terminal Configuration

For proper keyboard support (especially Option/Alt keys), configure your terminal:

**Terminal.app:**
1. Terminal → Preferences → Profiles → Keyboard
2. Check "Use Option as Meta key"

**iTerm2:**
1. iTerm2 → Preferences → Profiles → Keys
2. Set "Left Option Key" to "Esc+"

**Alacritty:**
Add to `~/.config/alacritty/alacritty.yml`:
```yaml
window:
  option_as_alt: Both
```

**Kitty:**
Add to `~/.config/kitty/kitty.conf`:
```
macos_option_as_alt yes
```

### Linux Terminal Configuration

Most Linux terminals work out of the box. For Alacritty:
```yaml
key_bindings:
  - { key: Left, mods: Alt, chars: "\x1b[1;3D" }
  - { key: Right, mods: Alt, chars: "\x1b[1;3C" }
```

### Windows Terminal Configuration

Windows Terminal works by default. For WSL2, follow Linux configuration.

## Usage

```bash
# Open empty editor
f1

# Open a file
f1 filename.txt

# Open multiple files
f1 file1.txt file2.rs file3.md
```

## Keyboard Shortcuts

| Action | Shortcut |
|--------|----------|
| **File Operations** |
| New Tab | `Ctrl+N` |
| Close Tab | `Ctrl+W` |
| Save | `Ctrl+S` |
| Open File | `F1` → Open File |
| Quit | `Ctrl+Q` |
| **Navigation** |
| Next Tab | `Ctrl+Tab` |
| Previous Tab | `Ctrl+Shift+Tab` |
| Move by Word | `Ctrl+←/→` or `Alt+←/→` |
| Page Up/Down | `PageUp/PageDown` |
| **Editing** |
| Select All | `Ctrl+A` |
| Copy | `Ctrl+C` |
| Cut | `Ctrl+X` |
| Paste | `Ctrl+V` |
| Delete Word | `Ctrl+Backspace` |
| **Selection** |
| Select with Keyboard | `Shift+Arrows` |
| Select Word | `Ctrl+Shift+←/→` |
| Select with Mouse | Click and drag |
| Select Word with Mouse | Double-click |

## Mouse Controls

- **Click**: Position cursor
- **Drag**: Select text
- **Double-click**: Select word
- **Scroll**: Navigate document
- **Tab click**: Switch tabs or show menu
- **F1 button**: Open menu

## Menu System

Press `F1` or click the `☰ F1` button to open the menu:
- **Current Tab**: Tab-specific operations
- **Open File**: Fuzzy file finder
- **Cancel**: Close menu

## License

MIT License - See LICENSE file for details
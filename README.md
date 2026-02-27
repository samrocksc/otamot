# 🍅 Otamot

A minimalist, cross-platform Pomodoro timer desktop application with Markdown notes support, built with Rust and [eframe/egui](https://github.com/emilk/egui).

> **Note:** "Otamot" is "tomato" spelled backwards - a fitting name for a reverse-engineering of the Pomodoro technique into code!

## What is a Pomodoro Timer?

The [Pomodoro Technique](https://en.wikipedia.org/wiki/Pomodoro_Technique) is a time management method that uses a timer to break work into intervals (traditionally 25 minutes), separated by short breaks (5 minutes). This app implements that technique with note-taking capabilities to capture your thoughts during work sessions.

## Features

### Timer
- ⏱️ **Customizable work/break durations** - Adjust via Settings
- 🔄 **Start/Pause/Reset/Skip controls** - Full timer control
- 📊 **Session tracking** - Counts completed pomodoros
- 🔔 **Auto-switch** - Transitions from work to break automatically

### Notes
- 📝 **Markdown editor** - Write notes during work sessions
- 👁️ **Live preview** - Toggle between Edit and Preview modes (Ctrl+P)
- 💾 **Auto-save** - Notes saved when work session completes
- 📁 **Custom directory** - Configure where notes are stored

### Notes Format
- Files saved as: `MM-DD-YYYY-HH-MM-HH-MM.md` (start time to end time)
- YAML frontmatter included:
  ```yaml
  ---
  title: "Pomodoro Session"
  date: 2026-02-27 11:30:00
  start_time: 2026-02-27 11:05:00
  end_time: 2026-02-27 11:30:00
  duration_minutes: 25
  mode: work
  sessions_completed: 3
  tags:
    - pomodoro
    - work
  ---
  ```

### Technical
- 🌙 **Dark theme** - Easy on the eyes
- 🖥️ **Cross-platform** - macOS, Linux, Windows
- 💾 **Settings persistence** - Configuration saved to `~/.config/otamot/settings.json`

## Installation

### Download

Download the latest release for your platform from the [Releases page](https://github.com/samrocksc/otamot/releases).

### From Source

```bash
# Clone the repository
git clone https://github.com/samrocksc/otamot.git
cd otamot

# Build and run
cargo run

# Build release binary
cargo build --release
```

The binary will be located at `target/release/otamot`.

## Usage

### Controls
- **START/PAUSE** - Begin or pause the timer
- **RESET** - Reset timer to initial work duration
- **SKIP** - Skip to break
- **⚙ Settings** - Adjust durations and notes directory
- **📝 Notes: ON/OFF** - Toggle the notes panel

### Keyboard Shortcuts
- **Ctrl+P** - Toggle between Edit and Preview modes (when notes enabled)

### Configuration
Settings are stored in `~/.config/otamot/settings.json`:
- Work duration (default: 25 minutes)
- Break duration (default: 5 minutes)
- Notes directory (default: `~/.config/otamot/notes`)
- Notes enabled state

## Technical Stack

- **Rust** - Type-safe, high-performance systems programming
- **eframe** - Cross-platform desktop framework
- **egui** - Immediate mode GUI library
- **Glow** - OpenGL-based rendering backend
- **pulldown-cmark** - Markdown parsing
- **chrono** - Date/time handling

## Roadmap

- [ ] System tray support
- [ ] Desktop notifications
- [ ] Sound notifications when timer completes
- [ ] Multiple timer presets
- [ ] Task integration
- [ ] Statistics dashboard

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License

## Acknowledgments

- [eframe/egui](https://github.com/emilk/egui) - The cross-platform GUI framework
- The Pomodoro Technique originally developed by Francesco Cirillo

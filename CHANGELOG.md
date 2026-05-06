# 🍅 Otamot Changelog

All notable changes to the productivity pizza that is **Otamot** will be documented here, bro!

---

## [v0.11.0] - 2026-05-06

### ✨ New Features

- **#hashtags Highlighting** — Hashtags in notes now render with a lighter color (65% brightness) for better visual distinction. Bro, your #project tags pop now!
- **Separate Call Notes Directory** — Call recordings save to a configurable location separate from regular notes. Configurable in Settings.
- **Active Listening Notifications** — New setting to remind you to stay engaged during calls with periodic prompts (every 3 minutes). Perfect for staying present!
- **Call Timer Background Running** — Call timer keeps running when the window loses focus. No more timing drift!

### 🐛 Bug Fixes

- Fixed app title typo in debug builds (was showing "otamot-f78b143e..." in ALSA warnings)
- Call notes now correctly save to `call_notes_directory` config location

---

## [v0.10.0] - 2026-03-16

### ✨ New Features & Improvements

- **Auto-Save Settings** — All settings changes now save automatically. No more "Save" button needed — just change and close!
- **Instant Theme Updates** — Theme changes apply immediately without needing to save.
- **Call Mode** — New timer mode for tracking call durations (counts up instead of down).

### 🐛 Bug Fixes

- **Call Mode Fixes** — Multiple bug fixes for call mode, help hotkey, and sidebar.
- **Settings Repaint** — Fixed immediate visual update after saving settings.
- **Sidebar Divider** — Shortened sidebar divider to match button width.

---

## [v0.9.0] - 2026-03-06

### ✨ New Features

- 🔔 **Tray Icon** — Visible tray icon in system menu bar (Issue #44)
- ⌨️ **Settings Hotkey** — CMD+, opens settings (Issue #47)
- 💾 **Save Notes Hotkey** — CMD+S saves notes (Issue #48)
- 📞 **Call Mode** — Timer counts up for call tracking (Issue #49)
- 🖥️ **Non-Retina Support** — Handle non-Retina screen changes gracefully (Issue #50)
- 🎨 **Sidebar Consistency** — Refactored sidebar buttons for consistency (Issue #51)

---

## [v0.7.1] - 2026-03-06

### ✨ New Features (The Cowabunga Update!)

- 🔔 **Desktop Notifications** — Now you'll get a real desktop heads-up when your work or break time is up. No more missing the transition while you're in the zone! (Powered by `notify-rust`)
- 🏮 **System Tray Integration** — Otamot now hangs out in your system tray like a true ninja. Minimize it, hide it, and control it without cluttering your dock. (Powered by `tray-icon` and `tao`)
- 🎵 **Customizable Bell Tunes** — Choose a chime that fits your vibe. We've added fresh patterns like *La Cukaracha* and *Ice Cream Truck* to celebrate your focus. (Powered by `rodio`)
- ⌨️ **New Pro Hotkeys** — Keep your hands on the keys:
    - `Cmd+,` — Snap straight to Settings.
    - `Ctrl+/` — Open the help menu.
    - `Ctrl+D` — Pop open the Statistics Dashboard.
    - `Ctrl+P` — Still there for that sweet Markdown preview toggle!

### 🏗️ Architectural Overhaul

- **Modular UI Refactor** — We've started breaking down the massive `app.rs` monolith into lean, mean modules in `src/ui/`. It's like cutting a giant pizza into perfect slices (easier to manage, and way better for the code's health!):
    - `src/ui/sidebar.rs` — Handles the navigation and side-kick UI.
    - `src/ui/timer.rs` — Where the focus magic happens.
    - `src/ui/notes.rs` — The new home for all your Markdown notes logic.

### 🐛 Bug Fixes & Chores

- Updated to latest `egui` (v0.29) for smoother visuals.
- Improved auto-save logic for notes when the timer ends.
- Fixed a sneaky layout bug in the settings panel.

---

## [v0.6.0] - Older Versions

*Looking for the legacy stuff? We weren't keeping notes back then, we were too busy eating pizza!*

---

> "One focus session at a time, bro. It's not just a timer, it's a lifestyle." — Michaelangelo 🍕

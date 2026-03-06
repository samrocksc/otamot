# 🏗️ Otamot Architecture: The Productivity Pizza

Listen up, bros! This is how we've built the ultimate productivity machine. It's like a perfectly constructed pizza — every layer matters, and it's all about that fresh delivery! 🍕🛹

---

## 🏗️ The High-Level Slice

1. **The Foundation (`lib.rs`)**: This is the crust. All the heavy lifting, business logic, and testable modules live here. We keep it separate from the GUI so it stays lean and mean.
2. **The Delivery (`main.rs`)**: This is the ninja delivery driver. It starts the engine and hands off the control to the `app.rs`.
3. **The Presentation (`src/ui/`)**: This is where we make it look good! We're moving from a giant monolithic `app.rs` (which was like one massive, hard-to-eat pizza) into perfect, bite-sized modules.

---

## 🍕 Core Modules (The Toppings)

Each module has a specific job, just like every turtle has their own weapon!

- ⏱️ **`timer`**: Manages the work/break cycles. Pure logic, no visuals.
- 📝 **`notes` & `markdown`**: Handles your thoughts, Markdown parsing, and writing to the disk.
- ⚙️ **`config`**: Keeps your settings safe and sound in `~/.config/otamot/`.
- 🔔 **`bell`**: The secret sauce that handles the new custom tunes using `rodio`.

---

## 🎨 UI Architecture: The 0.7.1 Refactor

We're cleaning up the workspace, bros! Instead of `app.rs` doing everything, we're delegating the visual work to these new modules in `src/ui/`:

| Module | What it handles |
|--------|--------------|
| `sidebar.rs` | Navigation, settings toggles, and side-kick UI components. |
| `timer.rs` | The main visual countdown and status displays. |
| `notes.rs` | The Markdown editor and preview rendering. |

---

## 🛠️ Ninja Tools (Dependencies)

We only use the sharpest gear:

- **`eframe/egui`**: Our main GUI engine. Fast, immediate, and as agile as a turtle!
- **`notify-rust`**: For those sweet desktop notifications.
- **`tray-icon` & `tao`**: Powers our new stealth mode in the system tray.
- **`rodio`**: The audio engine for our cowabunga bell tunes.

---

## 🥋 The Brawler's Rule: Business vs. UI

Never mix your business logic with your UI code, bro! It's like putting pineapple on a pizza... some people do it, but it's just not right for the architecture!

- **Logic** goes in `lib/` modules (e.g., `src/timer.rs`)
- **Visuals** go in `src/ui/` (e.g., `src/ui/timer.rs`)
- **`app.rs`** is the master-splinter that coordinates the two.

Stay focused, stay sharp! 🐢🍅

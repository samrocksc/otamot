# Tray Overview Dropdown Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only stats overview (survey score, sessions tracked, top 3 concentration issues) to the top of the system tray menu.

**Architecture:** Add a `TrayInfoItems` struct that holds the five disabled `MenuItem` handles and owns its own `refresh()` method. Store it in `PomodoroApp`. Populate at `setup_tray_icon()` with initial values from `SurveyData`. Call `refresh()` after session completion and survey submission.

**Tech Stack:** Rust, tray-icon 0.19 (`tray_icon::menu::{Menu, MenuItem, MenuItemExt, PredefinedMenuItem}`)

## Global Constraints

- All changes in `src/app.rs` — no new files
- `///` doc comments on all new named functions and structs
- Complete function blocks in every step — no truncated snippets
- `cargo fmt && cargo clippy && cargo test` must pass clean before committing
- Branch: `feature/56-overview-in-drop-down`
- Closes #56

---

### Task 1: Create branch and verify baseline

**Files:**
- No file changes

- [ ] **Step 1: Create branch**

```bash
git checkout -b feature/56-overview-in-drop-down
```

- [ ] **Step 2: Verify baseline**

```bash
cargo check && cargo test
```

Expected: 0 errors, all tests pass (baseline is 72 tests).

---

### Task 2: Add pure formatting helpers and TrayInfoItems struct

**Files:**
- Modify: `src/app.rs`

**Interfaces:**
- Produces: `fn format_tray_score(average_focus: f64, focus_count: u32) -> String`
- Produces: `fn format_tray_sessions(sessions_completed: u32) -> String`
- Produces: `fn format_tray_issue(issue: Option<&str>) -> String`
- Produces: `struct TrayInfoItems` with `fn new(survey: &SurveyData) -> Self` and `fn refresh(&self, survey: &SurveyData)`
- Produces: `tray_info_items: Option<TrayInfoItems>` field on `PomodoroApp`

- [ ] **Step 1: Write failing tests**

Add this module at the bottom of `src/app.rs`, after all existing `#[cfg(test)]` blocks:

```rust
#[cfg(test)]
mod tray_label_tests {
    use super::*;

    #[test]
    fn test_format_tray_score_with_data() {
        assert_eq!(format_tray_score(7.4, 3), "Survey Score: 7.4");
    }

    #[test]
    fn test_format_tray_score_no_data() {
        assert_eq!(format_tray_score(0.0, 0), "Survey Score: —");
    }

    #[test]
    fn test_format_tray_sessions() {
        assert_eq!(format_tray_sessions(12), "Sessions Tracked: 12");
    }

    #[test]
    fn test_format_tray_issue_present() {
        assert_eq!(format_tray_issue(Some("Slack notifications")), "• Slack notifications");
    }

    #[test]
    fn test_format_tray_issue_absent() {
        assert_eq!(format_tray_issue(None), "—");
    }
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test tray_label_tests
```

Expected: FAIL — `format_tray_score`, `format_tray_sessions`, `format_tray_issue` not defined.

- [ ] **Step 3: Add the three pure formatting functions**

Find the line `impl PomodoroApp {` in `src/app.rs` and add these functions **immediately above** it:

```rust
/// Returns the tray label for the survey score.
/// Shows "—" when no survey responses have been recorded yet.
fn format_tray_score(average_focus: f64, focus_count: u32) -> String {
    if focus_count == 0 {
        "Survey Score: —".to_string()
    } else {
        format!("Survey Score: {:.1}", average_focus)
    }
}

/// Returns the tray label for total sessions completed.
fn format_tray_sessions(sessions_completed: u32) -> String {
    format!("Sessions Tracked: {}", sessions_completed)
}

/// Returns the tray label for a single top issue entry.
/// Falls back to "—" when fewer than 3 issues have been recorded.
fn format_tray_issue(issue: Option<&str>) -> String {
    match issue {
        Some(s) => format!("• {}", s),
        None => "—".to_string(),
    }
}
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cargo test tray_label_tests
```

Expected: all 5 tests PASS.

- [ ] **Step 5: Add `TrayInfoItems` struct with `new` and `refresh`**

Add this struct **immediately below** the three formatting functions you just added, still above `impl PomodoroApp`:

```rust
/// Holds the live `MenuItem` handles for the read-only tray overview row.
///
/// Items are disabled (non-clickable) and used only for display.
/// Call `refresh` after any `SurveyData` mutation to keep labels current.
struct TrayInfoItems {
    score: tray_icon::menu::MenuItem,
    sessions: tray_icon::menu::MenuItem,
    issue_0: tray_icon::menu::MenuItem,
    issue_1: tray_icon::menu::MenuItem,
    issue_2: tray_icon::menu::MenuItem,
}

impl TrayInfoItems {
    /// Creates all five disabled menu items populated from `survey`.
    fn new(survey: &otamot::survey::SurveyData) -> Self {
        let top = survey.top_issues(3);
        Self {
            score: tray_icon::menu::MenuItem::new(
                format_tray_score(survey.average_focus, survey.focus_count),
                false,
                None,
            ),
            sessions: tray_icon::menu::MenuItem::new(
                format_tray_sessions(survey.sessions_completed),
                false,
                None,
            ),
            issue_0: tray_icon::menu::MenuItem::new(
                format_tray_issue(top.get(0).map(|s| s.as_str())),
                false,
                None,
            ),
            issue_1: tray_icon::menu::MenuItem::new(
                format_tray_issue(top.get(1).map(|s| s.as_str())),
                false,
                None,
            ),
            issue_2: tray_icon::menu::MenuItem::new(
                format_tray_issue(top.get(2).map(|s| s.as_str())),
                false,
                None,
            ),
        }
    }

    /// Updates all label text from the latest `survey` state.
    ///
    /// Pre-computes all strings before touching menu items to keep
    /// the borrow of `survey` and the borrow of `self` fully separate.
    fn refresh(&self, survey: &otamot::survey::SurveyData) {
        use tray_icon::menu::MenuItemExt;
        let top = survey.top_issues(3);
        let score_text = format_tray_score(survey.average_focus, survey.focus_count);
        let sessions_text = format_tray_sessions(survey.sessions_completed);
        let i0 = format_tray_issue(top.get(0).map(|s| s.as_str()));
        let i1 = format_tray_issue(top.get(1).map(|s| s.as_str()));
        let i2 = format_tray_issue(top.get(2).map(|s| s.as_str()));
        self.score.set_text(score_text);
        self.sessions.set_text(sessions_text);
        self.issue_0.set_text(i0);
        self.issue_1.set_text(i1);
        self.issue_2.set_text(i2);
    }
}
```

- [ ] **Step 6: Add `tray_info_items` field to `PomodoroApp` struct**

Find the `PomodoroApp` struct definition (around line 80). Add this field alongside `tray_icon`:

```rust
tray_info_items: Option<TrayInfoItems>,
```

- [ ] **Step 7: Initialize to `None` in `PomodoroApp::default()` / init block**

In the struct initialization block (around line 158, where `tray_icon: None` is set), add:

```rust
tray_info_items: None,
```

- [ ] **Step 8: Cargo check**

```bash
cargo check
```

Expected: 0 errors.

- [ ] **Step 9: Commit**

```bash
git add src/app.rs
git commit -m "feat: add TrayInfoItems struct and tray label formatters"
```

---

### Task 3: Wire TrayInfoItems into setup_tray_icon()

**Files:**
- Modify: `src/app.rs` — `setup_tray_icon()` function (around line 412)

**Interfaces:**
- Consumes: `TrayInfoItems::new(&SurveyData)` from Task 2
- Produces: `self.tray_info_items = Some(...)` set before function returns

- [ ] **Step 1: Replace the full `setup_tray_icon` function**

Find `fn setup_tray_icon(&mut self)` in `src/app.rs` and replace the entire function with:

```rust
#[cfg(not(target_arch = "wasm32"))]
fn setup_tray_icon(&mut self) {
    use tray_icon::{
        menu::{Menu, MenuItem, PredefinedMenuItem},
        TrayIconBuilder,
    };

    let survey = otamot::survey::SurveyData::load();
    let info = TrayInfoItems::new(&survey);

    let tray_menu = Menu::new();
    let start_pause_item = MenuItem::new("Start/Pause", true, None);
    let reset_item = MenuItem::new("Reset", true, None);
    let quit_item = MenuItem::new("Quit", true, None);

    self.tray_menu_ids
        .insert("start_pause".to_string(), start_pause_item.id().clone());
    self.tray_menu_ids
        .insert("reset".to_string(), reset_item.id().clone());
    self.tray_menu_ids
        .insert("quit".to_string(), quit_item.id().clone());

    let _ = tray_menu.append_items(&[
        &info.score,
        &info.sessions,
        &PredefinedMenuItem::separator(),
        &info.issue_0,
        &info.issue_1,
        &info.issue_2,
        &PredefinedMenuItem::separator(),
        &start_pause_item,
        &reset_item,
        &PredefinedMenuItem::separator(),
        &quit_item,
    ]);

    let icon = (|| {
        let icon_bytes = include_bytes!("../assets/icon.png");
        let img = image::load_from_memory(icon_bytes).ok()?;
        let img = img.resize(22, 22, image::imageops::FilterType::Lanczos3);
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        tray_icon::Icon::from_rgba(rgba.into_raw(), width, height).ok()
    })()
    .unwrap_or_else(|| {
        let size = 22u32;
        let mut pixels = vec![0u8; (size * size * 4) as usize];
        let center = (size / 2) as f32;
        let radius = (size / 2 - 2) as f32;
        for y in 0..size {
            for x in 0..size {
                let dx = x as f32 - center;
                let dy = y as f32 - center;
                let dist = (dx * dx + dy * dy).sqrt();
                let idx = ((y * size + x) * 4) as usize;
                if dist <= radius {
                    pixels[idx] = 220;
                    pixels[idx + 1] = 20;
                    pixels[idx + 2] = 60;
                    pixels[idx + 3] = 255;
                } else if dist <= radius + 1.5 {
                    pixels[idx] = 139;
                    pixels[idx + 1] = 0;
                    pixels[idx + 2] = 0;
                    pixels[idx + 3] = 255;
                }
            }
        }
        tray_icon::Icon::from_rgba(pixels, size, size).unwrap()
    });

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Otamot")
        .with_icon(icon)
        .with_icon_as_template(true)
        .build()
        .unwrap();

    self.tray_info_items = Some(info);
    self.tray_icon = Some(tray_icon);
}
```

- [ ] **Step 2: Cargo check**

```bash
cargo check
```

Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add src/app.rs
git commit -m "feat: populate tray overview items in setup_tray_icon"
```

---

### Task 4: Add update_tray_info() and wire up call sites

**Files:**
- Modify: `src/app.rs`

**Interfaces:**
- Consumes: `TrayInfoItems::refresh(&SurveyData)` from Task 2
- Call sites: after `self.survey_data.save()` at line ~562 (session complete) and line ~385 (survey submit)

- [ ] **Step 1: Add `update_tray_info()` method to `impl PomodoroApp`**

Add this complete method inside `impl PomodoroApp`, alongside `handle_tray_events`:

```rust
/// Refreshes tray overview labels from the current in-memory survey state.
/// No-op when tray is unavailable (e.g. wasm, or tray setup failed).
#[cfg(not(target_arch = "wasm32"))]
fn update_tray_info(&self) {
    if let Some(ref info) = self.tray_info_items {
        info.refresh(&self.survey_data);
    }
}
```

- [ ] **Step 2: Call `update_tray_info()` after session completion**

Find the block around line 560 that reads:

```rust
self.sessions_completed += 1;
self.survey_data.sessions_completed = self.sessions_completed;
let _ = self.survey_data.save();
```

Replace it with:

```rust
self.sessions_completed += 1;
self.survey_data.sessions_completed = self.sessions_completed;
let _ = self.survey_data.save();
#[cfg(not(target_arch = "wasm32"))]
self.update_tray_info();
```

- [ ] **Step 3: Call `update_tray_info()` after survey submission**

Find the block around line 380 that reads:

```rust
self.survey_data.add_response(
    // ...
);
if let Err(e) = self.survey_data.save() {
    eprintln!("Failed to save survey data: {}", e);
}
```

Add one line immediately after the `if let Err` block:

```rust
#[cfg(not(target_arch = "wasm32"))]
self.update_tray_info();
```

- [ ] **Step 4: Full verification**

```bash
cargo fmt && cargo clippy && cargo test
```

Expected: clean format, 0 clippy warnings, all 77 tests pass (72 baseline + 5 new tray label tests).

- [ ] **Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat: add update_tray_info and wire to session/survey events

Closes #56"
```

---

### Task 5: Push and open PR

- [ ] **Step 1: Push branch**

```bash
git push -u origin feature/56-overview-in-drop-down
```

- [ ] **Step 2: Open PR**

```bash
gh pr create \
  --title "Overview in Drop Down" \
  --body "$(cat <<'EOF'
Closes #56

## Changes
- Added `TrayInfoItems` struct with `new()` and `refresh()` encapsulating all tray label logic
- Added `format_tray_score`, `format_tray_sessions`, `format_tray_issue` pure helpers (5 unit tests)
- Tray menu now shows Survey Score, Sessions Tracked, and top 3 concentration issues above Start/Pause
- `update_tray_info()` keeps labels current after session completion and survey submission

## Testing
- [ ] `cargo test` passes (77 tests)
- [ ] `cargo clippy` clean
- [ ] Opened tray — stats visible at top
- [ ] Completed a session — session count incremented in tray
- [ ] Submitted a survey — score updated in tray
EOF
)"
```

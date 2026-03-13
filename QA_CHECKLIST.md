# Otamot QA Checklist - v0.10.0 Release

## Summary of Changes

This release includes **7 commits** addressing bug fixes and enhancements from GitHub issues.

### Commits Included:
1. `3e3ed53` - feat(#48): add CMD+S hotkey for saving notes
2. `f267167` - feat(#49): add Call Mode for call timer functionality
3. `37a2150` - fix(#50): handle non-retina screen changes gracefully
4. `7871e1a` - fix(#44): add visible tray icon
5. `20f5401` - feat(#47): add CMD+, hotkey for settings
6. `54e1069` - feat(#51): refactor sidebar buttons for consistency
7. `8006e38` - fix(ui): remove duplicate kanban frame and implement real tray icon

---

## Manual QA Checklist

### 🔴 Critical Bug Fixes

#### Issue #44: Tray Icon Visibility
- [ ] **Test**: Launch the app and verify the tray icon is visible in the system menu bar
- [ ] **Test**: The icon should appear as a template image (adapts to light/dark mode on macOS)
- [ ] **Test**: Clicking the tray icon should show/hide the app window

#### Issue #50: Non-Retina Screen Segfault
- [ ] **Test**: Connect a non-Retina external monitor
- [ ] **Test**: Move the app window between Retina and non-Retina displays
- [ ] **Test**: Disconnect and reconnect the external monitor - app should not crash
- [ ] **Test**: Verify window position/size persists correctly across monitor changes

---

### 🟢 New Features

#### Issue #48: Save Notes Hotkey (CMD/CTRL+S)
- [ ] **Test**: Open notes editor (Edit tab)
- [ ] **Test**: Type some text in the notes
- [ ] **Test**: Press CMD+S (macOS) or CTRL+S (Windows/Linux)
- [ ] **Test**: Verify notes are saved (check notification or file saved)
- [ ] **Test**: Try in different notes views (Edit, Preview, Project)

#### Issue #47: Settings Hotkey (CMD/CTRL+,)
- [ ] **Test**: Press CMD+, (macOS) or CTRL+, (Windows/Linux)
- [ ] **Test**: Settings dialog should open
- [ ] **Test**: Press ESC to close settings
- [ ] **Test**: Try opening settings from different app states

#### Issue #45: Markdown Link Hotkey (CMD/CTRL+K)
- [ ] **Test**: Open notes editor (Edit tab)
- [ ] **Test**: Position cursor in text area
- [ ] **Test**: Press CMD+K (macOS) or CTRL+K (Windows/Linux)
- [ ] **Test**: Verify `[]()` is inserted and cursor is positioned inside `[]`
- [ ] **Test**: Type link text and URL to complete the markdown link

#### Issue #49: Call Mode
- [ ] **Test**: Open the app and locate the "Start Call" button
- [ ] **Test**: Click "Start Call" - timer should switch to counting UP
- [ ] **Test**: Verify timer format: MM:SS for calls under an hour, HH:MM:SS for longer
- [ ] **Test**: Add some notes during the call
- [ ] **Test**: Click "End Call" - notes should be saved with frontmatter:
  - [ ] Verify `duration_seconds` in frontmatter
  - [ ] Verify `start_time` and `end_time` in frontmatter
  - [ ] Verify `mode: call` in frontmatter
- [ ] **Test**: Check saved file in notes directory
- [ ] **Test**: Verify Start Call button changes to End Call during active call
- [ ] **Test**: Test with both English and German locales

#### Issue #52: Light Theme
- [ ] **Test**: Open Settings (gear icon or CMD+,)
- [ ] **Test**: Navigate to theme selector
- [ ] **Test**: Select "Light" theme
- [ ] **Test**: Verify:
  - [ ] White background
  - [ ] Black/dark text
  - [ ] All UI elements readable
  - [ ] Buttons visible
  - [ ] Timer visible
  - [ ] Notes editor readable

#### Issue #51: Sidebar Buttons Consistency
- [ ] **Test**: Open sidebar
- [ ] **Test**: Verify all buttons have equal size
- [ ] **Test**: Check hover effects on buttons
- [ ] **Test**: Verify buttons are themable (change theme and check)
- [ ] **Test**: Test in both dark and light themes

---

### 🔄 Regression Testing

#### Core Functionality
- [ ] **Test**: Start a Pomodoro timer (25 min work session)
- [ ] **Test**: Timer counts down correctly
- [ ] **Test**: Break timer works after work session
- [ ] **Test**: Bell sound plays at end of timer
- [ ] **Test**: Notes are saved correctly

#### Keyboard Shortcuts Summary
| Shortcut | Action |
|----------|--------|
| CMD+S | Save notes |
| CMD+, | Open settings |
| CMD+K | Insert markdown link |
| CMD+Shift+/ | Toggle help menu |
| Space | Start/Pause timer |
| R | Reset timer |

---

## Build & Test Results

```
✅ cargo check - Success
✅ cargo test - 91 tests passed
✅ cargo fmt - Success
⚠️ cargo clippy - 3 warnings (existing code - too_many_arguments)
```

---

## Pre-Release Checklist

- [ ] All manual QA tests passed
- [ ] No regressions in core functionality
- [ ] All new features working as expected
- [ ] Documentation updated (if needed)
- [ ] CHANGELOG.md updated
- [ ] Version bumped in Cargo.toml
- [ ] Git tags created

---

## Notes for Testing

1. **macOS Focus**: Primary testing on macOS for tray icon and Retina display tests
2. **Cross-platform**: Test keyboard shortcuts on Windows/Linux if available
3. **Data Safety**: Use a test notes directory - don't use production data
4. **Monitor Changes**: Issue #50 fix specifically addresses non-Retina monitor handling

---

## Release Command

After QA approval:

```bash
# Push commits to origin
git push origin main

# Create release tag
git tag -a v0.10.0 -m "Release v0.10.0: Bug fixes and enhancements"
git push origin v0.10.0

# Create GitHub release
gh release create v0.10.0 --title "v0.10.0" --notes-file RELEASE_NOTES.md
```
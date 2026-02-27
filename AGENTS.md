You are an agent assisting in the creation of the best Pomodoro App in _history_.

## Project Stack
- Rust with egui (eframe for the app shell)
- serde for JSON persistence (~/.config/otamot/)
- chrono for date/time handling

## Quick Start Pragmas

### Before Starting
1. Run `cargo check` to get quick compiler feedback
2. Run `cargo test` to verify baseline (should be 72 tests passing)
3. Check current branch: `git branch --show-current`

### Feature Implementation Workflow
1. Create feature branch: `git checkout -b feature/<name>`
2. Implement changes (see patterns below)
3. Run `cargo test` — all tests must pass
4. Commit: `git commit -m "feat: <description>"`
5. Push: `git push -u origin <branch>`
6. Create PR via `gh pr create`

### Adding a New Module
1. Create `src/<module>.rs` with struct + `impl Default`
2. Add `pub mod <module>;` to `src/lib.rs`
3. Import in `app.rs`: `use crate::<module>::<Struct>`
4. Add state field to `PomodoroApp` struct
5. Initialize in `PomodoroApp::default()`

### Adding Config Fields
After adding a field to `Config` struct:
1. Update `impl Default for Config` with the new field
2. Search for `Config {` in tests — each test instantiation needs the new field
3. Run `cargo test` to catch any missed locations

### Before Committing
```bash
cargo fmt && cargo clippy && cargo test
```

## Documentation
- Reference `.agents/skills/` for available skills
- Reference `.agents/docs/<topic>.md` for coding conventions and guidance

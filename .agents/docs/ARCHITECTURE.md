# Architecture

- Binary crate (`main.rs`) includes `app.rs` for GUI; library crate (`lib.rs`) exposes testable modules
- Keep business logic in `lib/` modules (`timer`, `config`, `markdown`, `notes`)
- GUI code in `app.rs` should delegate to library modules, not implement logic

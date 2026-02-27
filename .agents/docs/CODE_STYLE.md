# Code Style

- Use `rustfmt` defaults; run `cargo fmt` before committing
- Prefer `impl Default` over `fn new()` for simple struct initialization
- Use `#[cfg(test)]` modules at the bottom of each file for unit tests

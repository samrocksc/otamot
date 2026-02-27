# Workflow Patterns

## Feature Branch Naming
```
feature/<descriptive-name>    # New features
fix/<issue-description>       # Bug fixes
refactor/<area>               # Code restructuring
```

## Module Creation Pattern

When creating a new module that persists data:

```rust
// src/<module>.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct <Module>Data {
    // fields with serde defaults for backward compatibility
    #[serde(default)]
    pub field: Type,
}

impl <Module>Data {
    fn config_path() -> PathBuf {
        dirs::config_dir()
            .expect("Could not find config directory")
            .join("otamot")
            .join("<module>.json")
    }

    pub fn load() -> Self {
        // Load from disk or return default
    }

    pub fn save(&self) {
        // Persist to disk
    }
}
```

## App Integration Pattern

1. **Add state to PomodoroApp**:
```rust
pub struct PomodoroApp {
    // ... existing fields
    <module>_data: <Module>Data,
    show_<module>_dialog: bool,
}
```

2. **Initialize in Default**:
```rust
impl Default for PomodoroApp {
    fn default() -> Self {
        Self {
            // ... existing fields
            <module>_data: <Module>Data::load(),
            show_<module>_dialog: false,
        }
    }
}
```

3. **Add UI in appropriate location** (settings dialog, main panel, etc.)

## Config Field Updates

When adding a new field to `Config`:

1. Add field with serde default:
```rust
#[serde(default = "default_<field>")]
pub <field>: <type>,

fn default_<field>() -> <type> { default_value }
```

2. Update `impl Default for Config`

3. **Critical**: Update ALL test Config instantiations:
```rust
// Tests will fail with "missing field" errors
let config = Config {
    // existing fields
    <field>: default_value,  // Add this to every test
};
```

4. Run `cargo test` to verify all are updated

## Commit Message Convention

```
feat: add <feature description>
fix: correct <bug description>
refactor: <what was refactored>
docs: update <documentation area>
test: add <test description>
```

## Pull Request Workflow

```bash
# After committing all changes
git push -u origin <branch-name>

# Create PR with title matching commit convention
gh pr create --title "feat: <description>" --body "Closes #<issue>

## Summary
- Brief bullet points of changes

## Testing
- cargo test passes (N tests)
- Manual testing notes"
```

## Testing Best Practices

- Use `tempfile` for file I/O tests to avoid polluting config directory
- Group related tests in `#[cfg(test)] mod tests { ... }` at file bottom
- Test edge cases: empty state, boundary values, error conditions

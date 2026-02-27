# Features

## Implemented Features

### Timer System (`src/timer.rs`)
- Pomodoro timer with work/break sessions
- Configurable work and break durations
- Auto-transition between sessions
- Visual and audio notifications

### Configuration (`src/config.rs`)
- Persistent settings in `~/.config/otamot/config.json`
- Work/break duration settings
- Auto-start options
- Survey enable/disable toggle
- Theme preferences

### Session Notes (`src/notes.rs`)
- Capture notes during work sessions
- Markdown rendering support (`src/markdown.rs`)
- Persistent storage per session

### Post-Session Surveys (`src/survey.rs`)
- Optional surveys after work sessions
- Focus rating (1-10 scale)
- Free-form "what helped" and "what hurt" inputs
- Running and daily averages calculated
- Data stored in `~/.config/otamot/survey.json`
- Reset data option in settings

## Development Guidelines

- All new features require unit tests before merge
- Use conventional commits: `feat:`, `fix:`, `docs:`, `refactor:`
- Keep features modular and testable (GUI code in `app.rs`, logic in lib modules)

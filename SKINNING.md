# Skinning Otamot

Otamot supports full UI skinning via its configuration file. You can create your own look and feel by modifying the `theme` object in your `settings.json`.

## Configuration Path

- **macOS/Linux**: `~/.config/otamot/settings.json`
- **Windows**: `%USERPROFILE%\.config\otamot\settings.json`

## Theme Structure

The `theme` object in `settings.json` controls the color palette for the entire application.

```json
"theme": {
  "name": "Robotic Lime",
  "text": { "r": 0, "g": 255, "b": 0 },
  "text_dim": { "r": 0, "g": 136, "b": 0 },
  "text_highlight": { "r": 0, "g": 51, "b": 0 },
  "work": { "r": 204, "g": 255, "b": 0 },
  "b_break": { "r": 0, "g": 204, "b": 0 },
  "button": { "r": 0, "g": 51, "b": 0 },
  "bg": { "r": 5, "g": 5, "b": 5 },
  "tab_active": { "r": 0, "g": 255, "b": 0 },
  "tab_inactive": { "r": 0, "g": 34, "b": 0 }
}
```

### Key Reference

| Key | Description |
|-----|-------------|
| `name` | The display name of your theme. |
| `text` | Main foreground color (labels, editor text, etc). |
| `text_dim` | Secondary text color used for metadata like session counts. |
| `text_highlight` | Text color when rendered on a bright background (active tabs). |
| `work` | Color of the timer during "Work" mode. |
| `b_break` | Color of the timer during "Break" mode. |
| `button` | Background color for all rounded UI buttons. |
| `bg` | Application window background color. |
| `tab_active` | Background color for the currently selected view tab. |
| `tab_inactive` | Background color for unselected tabs. |

## Creating a Custom Skin

1. Open `settings.json`.
2. Locate the `"theme"` block.
3. Replace the RGB values for any field you wish to change.
4. Restart Otamot to see your changes! Or, use the settings menu in-app to swap between presets.

## Color Tips

- **Contrast**: For dark themes, keep `text` bright and `bg` dark. For light themes, do the opposite.
- **Transparency**: Otamot colors currently use pure RGB (0-255).
- **Robotic Vibe**: Try low `bg` values (5-10) with fully saturated `text` values (255) for an OLED/Terminal look.

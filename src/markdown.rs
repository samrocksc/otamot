//! Markdown processing for the Pomodoro app
//!
//! This module contains markdown formatting logic, extracted for testability.

use chrono::Local;

/// Process inline markdown formatting
/// Returns text with markdown syntax simplified for display
pub fn format_inline_markdown(text: &str) -> String {
    let mut result = text.to_string();

    // Handle inline code `code`
    while let Some(start) = result.find('`') {
        if let Some(end) = result[start + 1..].find('`') {
            let code = &result[start + 1..start + 1 + end];
            result = format!(
                "{}[{}]{}",
                &result[..start],
                code,
                &result[start + 1 + end + 1..]
            );
        } else {
            break;
        }
    }

    // Remove bold markers but keep text
    result = result.replace("**", "");

    result
}

/// Format markdown content for cleaner display
/// This includes normalizing whitespace, fixing list formatting, etc.
pub fn format_markdown(content: &str) -> String {
    let result: Vec<&str> = content.lines().collect();
    let mut formatted_lines = Vec::new();
    let mut prev_was_list = false;

    for line in result {
        let trimmed = line.trim_start();
        let current_indent = line.len() - trimmed.len();

        // Handle list items
        if let Some(list_char) = trimmed.chars().next() {
            if list_char == '-' || list_char == '*' {
                // Bullet list
                if let Some(content) = trimmed
                    .strip_prefix("- ")
                    .or_else(|| trimmed.strip_prefix("* "))
                {
                    // Normalize bullet style to "-"
                    let content = content.trim_start();
                    let normalized_indent = if current_indent == 0 {
                        String::new()
                    } else {
                        // Normalize to 2-space increments
                        let indent_level = (current_indent / 2).max(1);
                        "  ".repeat(indent_level)
                    };
                    formatted_lines.push(format!("{}- {}", normalized_indent, content));
                    prev_was_list = true;
                    continue;
                }
            } else if list_char.is_ascii_digit() {
                // Numbered list
                if let Some(dot_pos) = trimmed.find('.') {
                    if dot_pos > 0 && dot_pos < 4 {
                        let number_part = &trimmed[..dot_pos];
                        if number_part.chars().all(|c| c.is_ascii_digit()) {
                            let content = trimmed[dot_pos + 1..].trim_start();
                            let normalized_indent = if current_indent == 0 {
                                String::new()
                            } else {
                                let indent_level = (current_indent / 2).max(1);
                                "  ".repeat(indent_level)
                            };
                            formatted_lines.push(format!("{}1. {}", normalized_indent, content));
                            prev_was_list = true;
                            continue;
                        }
                    }
                }
            }
        }

        // Add blank line between list and non-list content
        if prev_was_list && !trimmed.is_empty() && !is_list_line(trimmed) {
            formatted_lines.push(String::new());
        }

        // Handle empty lines
        if trimmed.is_empty() {
            // Don't add multiple consecutive empty lines
            if formatted_lines.last().is_none_or(|l| !l.is_empty()) {
                formatted_lines.push(String::new());
            }
            prev_was_list = false;
            continue;
        }

        formatted_lines.push(line.to_string());
        prev_was_list = false;
    }

    // Remove trailing empty lines (keep at most one)
    while formatted_lines.len() > 1 && formatted_lines.last().is_some_and(|l| l.is_empty()) {
        formatted_lines.pop();
    }

    formatted_lines.join("\n")
}

/// Check if a line is a list item
fn is_list_line(trimmed: &str) -> bool {
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        return true;
    }
    // Check for numbered list
    if let Some(dot_pos) = trimmed.find('.') {
        if dot_pos > 0 && dot_pos < 4 {
            let number_part = &trimmed[..dot_pos];
            if number_part.chars().all(|c| c.is_ascii_digit()) {
                return true;
            }
        }
    }
    false
}

/// Insert a date-stamped bullet point at the beginning of content
pub fn insert_date_bullet(content: &str) -> String {
    let date = Local::now().format("%Y-%m-%d %H:%M").to_string();
    if content.is_empty() {
        format!("- {} ", date)
    } else {
        format!("- {} \n{}", date, content)
    }
}

/// Handle smart list continuation
/// If the previous line was a list item, continue the list appropriately
pub fn handle_list_continuation(content: &str, cursor_line: usize) -> Option<(usize, String)> {
    let lines: Vec<&str> = content.lines().collect();
    if cursor_line == 0 || cursor_line > lines.len() {
        return None;
    }

    let prev_line = lines.get(cursor_line - 1)?;
    let trimmed = prev_line.trim_start();
    let indent_len = prev_line.len() - trimmed.len();

    // Empty list item - user pressed enter on empty bullet
    if trimmed == "-" || trimmed == "*" {
        // Remove the empty bullet line
        let mut new_lines = lines.clone();
        new_lines[cursor_line - 1] = "";
        return Some((cursor_line - 1, new_lines.join("\n")));
    }

    // Bullet list continuation
    if let Some(content_after_bullet) = trimmed.strip_prefix("- ") {
        if content_after_bullet.is_empty() {
            return None; // Don't create more bullets on empty
        }
        let indent = " ".repeat(indent_len);
        return Some((cursor_line, format!("{}- ", indent)));
    }

    if let Some(content_after_bullet) = trimmed.strip_prefix("* ") {
        if content_after_bullet.is_empty() {
            return None;
        }
        let indent = " ".repeat(indent_len);
        return Some((cursor_line, format!("{}* ", indent)));
    }

    // Numbered list continuation
    if let Some(dot_pos) = trimmed.find('.') {
        if dot_pos > 0 && dot_pos < 4 {
            let number_part = &trimmed[..dot_pos];
            if number_part.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(num) = number_part.parse::<u32>() {
                    let indent = " ".repeat(indent_len);
                    return Some((cursor_line, format!("{}{}. ", indent, num + 1)));
                }
            }
        }
    }

    None
}

/// Get current date/time formatted for notes
pub fn get_formatted_date() -> String {
    Local::now().format("%Y-%m-%d %H:%M").to_string()
}

/// Represents a parsed markdown line for rendering
#[derive(Debug, Clone, PartialEq)]
pub enum MarkdownLine {
    Heading1(String),
    Heading2(String),
    Heading3(String),
    BulletPoint(String),
    SubBulletPoint(String),
    Bold(String),
    CodeBlockStart(String),
    CodeBlockEnd(String),
    CodeLine(String),
    EmptyLine,
    Paragraph(String),
}

/// Parse a single line of markdown
pub fn parse_markdown_line(line: &str, in_code_block: &mut bool) -> MarkdownLine {
    let trimmed = line.trim();

    // Handle code blocks
    if trimmed.starts_with("```") {
        if *in_code_block {
            *in_code_block = false;
            MarkdownLine::CodeBlockEnd(trimmed.to_string())
        } else {
            *in_code_block = true;
            MarkdownLine::CodeBlockStart(trimmed.to_string())
        }
    } else if *in_code_block {
        MarkdownLine::CodeLine(line.to_string())
    } else if trimmed.starts_with("# ") {
        MarkdownLine::Heading1(trimmed.strip_prefix("# ").unwrap_or(trimmed).to_string())
    } else if trimmed.starts_with("## ") {
        MarkdownLine::Heading2(trimmed.strip_prefix("## ").unwrap_or(trimmed).to_string())
    } else if trimmed.starts_with("### ") {
        MarkdownLine::Heading3(trimmed.strip_prefix("### ").unwrap_or(trimmed).to_string())
    } else if line.starts_with("  - ") || line.starts_with("  * ") {
        // Check original line for indentation (sub-bullets have 2-space indent)
        let content = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .unwrap_or(trimmed);
        MarkdownLine::SubBulletPoint(content.to_string())
    } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        let content = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .unwrap_or(trimmed);
        MarkdownLine::BulletPoint(content.to_string())
    } else if trimmed.starts_with("**") && trimmed.ends_with("**") && trimmed.len() > 4 {
        let bold_text = trimmed[2..trimmed.len() - 2].to_string();
        MarkdownLine::Bold(bold_text)
    } else if trimmed.is_empty() {
        MarkdownLine::EmptyLine
    } else {
        MarkdownLine::Paragraph(line.to_string())
    }
}

/// Parse full markdown content into lines
pub fn parse_markdown(content: &str) -> Vec<MarkdownLine> {
    let mut in_code_block = false;
    content
        .lines()
        .map(|line| parse_markdown_line(line, &mut in_code_block))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for format_inline_markdown

    #[test]
    fn test_format_inline_code() {
        assert_eq!(format_inline_markdown("`hello`"), "[hello]");
        assert_eq!(format_inline_markdown("use `foo` here"), "use [foo] here");
        assert_eq!(
            format_inline_markdown("`code` and `more`"),
            "[code] and [more]"
        );
    }

    #[test]
    fn test_format_inline_bold() {
        assert_eq!(format_inline_markdown("**bold**"), "bold");
        assert_eq!(
            format_inline_markdown("this is **important**"),
            "this is important"
        );
        assert_eq!(format_inline_markdown("**a** and **b**"), "a and b");
    }

    #[test]
    fn test_format_inline_bold_and_code() {
        assert_eq!(
            format_inline_markdown("`code` and **bold**"),
            "[code] and bold"
        );
        assert_eq!(format_inline_markdown("**use `var`**"), "use [var]");
    }

    // Tests for format_markdown

    #[test]
    fn test_format_markdown_normalize_bullets() {
        // Should normalize * to -
        let result = format_markdown("* Item one\n* Item two");
        assert_eq!(result, "- Item one\n- Item two");
    }

    #[test]
    fn test_format_markdown_blank_line_after_list() {
        let result = format_markdown("- Item one\nSome text");
        assert!(result.contains("- Item one\n\nSome text"));
    }

    #[test]
    fn test_format_markdown_remove_extra_blank_lines() {
        let result = format_markdown("Line one\n\n\n\nLine two");
        assert_eq!(result, "Line one\n\nLine two");
    }

    #[test]
    fn test_format_markdown_trailing_blank_lines() {
        let result = format_markdown("Content\n\n\n");
        assert_eq!(result, "Content");
    }

    #[test]
    fn test_format_markdown_numbered_list() {
        let result = format_markdown("1. First\n2. Second");
        assert!(result.contains("1. First"));
        assert!(result.contains("1. Second")); // We normalize to 1.
    }

    // Tests for insert_date_bullet

    #[test]
    fn test_insert_date_bullet_empty() {
        let result = insert_date_bullet("");
        assert!(result.starts_with("- "));
        // Should be format: "- YYYY-MM-DD HH:MM "
        assert!(result.contains(" "));
        // Empty content should not have trailing newline
        assert!(!result.contains('\n'));
    }

    #[test]
    fn test_insert_date_bullet_with_content() {
        let result = insert_date_bullet("Existing content");
        assert!(result.starts_with("- "));
        // Check for the space before newline (format is "- DATE \ncontent")
        assert!(result.contains(" \nExisting content"));
    }

    // Tests for handle_list_continuation

    #[test]
    fn test_list_continuation_bullet() {
        let content = "- First item\n";
        let result = handle_list_continuation(content, 1);
        assert_eq!(result, Some((1, "- ".to_string())));
    }

    #[test]
    fn test_list_continuation_empty_bullet() {
        let content = "- \n";
        let result = handle_list_continuation(content, 1);
        assert_eq!(result, None); // Should not create more bullets on empty
    }

    #[test]
    fn test_list_continuation_numbered() {
        let content = "1. First item\n";
        let result = handle_list_continuation(content, 1);
        assert_eq!(result, Some((1, "2. ".to_string())));
    }

    #[test]
    fn test_list_continuation_not_list() {
        let content = "Just text\n";
        let result = handle_list_continuation(content, 1);
        assert_eq!(result, None);
    }

    // Tests for get_formatted_date

    #[test]
    fn test_get_formatted_date_format() {
        let date = get_formatted_date();
        // Format should be YYYY-MM-DD HH:MM
        assert!(date.len() == 16); // "2026-03-02 09:20" = 16 chars
        assert!(date.contains('-'));
        assert!(date.contains(':'));
        assert!(date.contains(' '));
    }
}

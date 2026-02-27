//! Markdown processing for the Pomodoro app
//! 
//! This module contains markdown formatting logic, extracted for testability.

/// Process inline markdown formatting
/// Returns text with markdown syntax simplified for display
pub fn format_inline_markdown(text: &str) -> String {
    let mut result = text.to_string();
    
    // Handle inline code `code`
    while let Some(start) = result.find('`') {
        if let Some(end) = result[start+1..].find('`') {
            let code = &result[start+1..start+1+end];
            result = format!("{}[{}]{}", &result[..start], code, &result[start+1+end+1..]);
        } else {
            break;
        }
    }
    
    // Remove bold markers but keep text
    result = result.replace("**", "");
    
    result
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
        let content = trimmed.strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .unwrap_or(trimmed);
        MarkdownLine::SubBulletPoint(content.to_string())
    } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        let content = trimmed.strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .unwrap_or(trimmed);
        MarkdownLine::BulletPoint(content.to_string())
    } else if trimmed.starts_with("**") && trimmed.ends_with("**") && trimmed.len() > 4 {
        let bold_text = trimmed[2..trimmed.len()-2].to_string();
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
    content.lines()
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
        assert_eq!(format_inline_markdown("`code` and `more`"), "[code] and [more]");
    }

    #[test]
    fn test_format_inline_bold() {
        assert_eq!(format_inline_markdown("**bold**"), "bold");
        assert_eq!(format_inline_markdown("this is **important**"), "this is important");
        assert_eq!(format_inline_markdown("**a** and **b**"), "a and b");
    }

    #[test]
    fn test_format_inline_bold_and_code() {
        assert_eq!(format_inline_markdown("`code` and **bold**"), "[code] and bold");
        assert_eq!(format_inline_markdown("**use `var`**"), "use [var]");
    }

    #[test]
    fn test_format_inline_no_markers() {
        assert_eq!(format_inline_markdown("plain text"), "plain text");
        assert_eq!(format_inline_markdown(""), "");
    }

    #[test]
    fn test_format_inline_unmatched_markers() {
        assert_eq!(format_inline_markdown("`unclosed"), "`unclosed");
        // ** gets replaced with empty string, so **unclosed becomes unclosed
        assert_eq!(format_inline_markdown("**unclosed"), "unclosed");
    }

    // Tests for parse_markdown_line

    #[test]
    fn test_parse_heading1() {
        let mut in_code = false;
        let result = parse_markdown_line("# Hello World", &mut in_code);
        assert_eq!(result, MarkdownLine::Heading1("Hello World".to_string()));
        assert!(!in_code);
    }

    #[test]
    fn test_parse_heading2() {
        let mut in_code = false;
        let result = parse_markdown_line("## Section", &mut in_code);
        assert_eq!(result, MarkdownLine::Heading2("Section".to_string()));
    }

    #[test]
    fn test_parse_heading3() {
        let mut in_code = false;
        let result = parse_markdown_line("### Subsection", &mut in_code);
        assert_eq!(result, MarkdownLine::Heading3("Subsection".to_string()));
    }

    #[test]
    fn test_parse_bullet_dash() {
        let mut in_code = false;
        let result = parse_markdown_line("- Item one", &mut in_code);
        assert_eq!(result, MarkdownLine::BulletPoint("Item one".to_string()));
    }

    #[test]
    fn test_parse_bullet_asterisk() {
        let mut in_code = false;
        let result = parse_markdown_line("* Item two", &mut in_code);
        assert_eq!(result, MarkdownLine::BulletPoint("Item two".to_string()));
    }

    #[test]
    fn test_parse_sub_bullet() {
        let mut in_code = false;
        let result = parse_markdown_line("  - Sub item", &mut in_code);
        assert_eq!(result, MarkdownLine::SubBulletPoint("Sub item".to_string()));
    }

    #[test]
    fn test_parse_bold_line() {
        let mut in_code = false;
        let result = parse_markdown_line("**Important Note**", &mut in_code);
        assert_eq!(result, MarkdownLine::Bold("Important Note".to_string()));
    }

    #[test]
    fn test_parse_bold_too_short() {
        let mut in_code = false;
        // "**a**" has length 5, which is not > 4, so it's treated as paragraph
        let result = parse_markdown_line("**a**", &mut in_code);
        // Actually 5 > 4, so should be bold
        assert_eq!(result, MarkdownLine::Bold("a".to_string()));
    }

    #[test]
    fn test_parse_empty_line() {
        let mut in_code = false;
        let result = parse_markdown_line("", &mut in_code);
        assert_eq!(result, MarkdownLine::EmptyLine);
        
        let result = parse_markdown_line("   ", &mut in_code);
        assert_eq!(result, MarkdownLine::EmptyLine);
    }

    #[test]
    fn test_parse_paragraph() {
        let mut in_code = false;
        let result = parse_markdown_line("Just some text", &mut in_code);
        assert_eq!(result, MarkdownLine::Paragraph("Just some text".to_string()));
    }

    #[test]
    fn test_parse_code_block_start() {
        let mut in_code = false;
        let result = parse_markdown_line("```rust", &mut in_code);
        assert_eq!(result, MarkdownLine::CodeBlockStart("```rust".to_string()));
        assert!(in_code);
    }

    #[test]
    fn test_parse_code_block_content() {
        let mut in_code = true;
        let result = parse_markdown_line("let x = 5;", &mut in_code);
        assert_eq!(result, MarkdownLine::CodeLine("let x = 5;".to_string()));
        assert!(in_code);
    }

    #[test]
    fn test_parse_code_block_end() {
        let mut in_code = true;
        let result = parse_markdown_line("```", &mut in_code);
        assert_eq!(result, MarkdownLine::CodeBlockEnd("```".to_string()));
        assert!(!in_code);
    }

    #[test]
    fn test_parse_full_markdown() {
        let content = r#"# Title

Some paragraph text.

- Item 1
- Item 2

## Section

More text."#;
        
        let lines = parse_markdown(content);
        // Lines: Title, empty, paragraph, empty, item1, item2, empty, section, empty, text = 10 lines
        assert_eq!(lines.len(), 10);
        assert_eq!(lines[0], MarkdownLine::Heading1("Title".to_string()));
        assert_eq!(lines[1], MarkdownLine::EmptyLine);
        assert_eq!(lines[2], MarkdownLine::Paragraph("Some paragraph text.".to_string()));
        assert_eq!(lines[3], MarkdownLine::EmptyLine);
        assert_eq!(lines[4], MarkdownLine::BulletPoint("Item 1".to_string()));
        assert_eq!(lines[5], MarkdownLine::BulletPoint("Item 2".to_string()));
        assert_eq!(lines[6], MarkdownLine::EmptyLine);
        assert_eq!(lines[7], MarkdownLine::Heading2("Section".to_string()));
    }

    #[test]
    fn test_parse_code_block_multiline() {
        let content = r#"```rust
fn main() {
    println!("hello");
}
```"#;
        
        let lines = parse_markdown(content);
        assert_eq!(lines.len(), 5);
        assert!(matches!(lines[0], MarkdownLine::CodeBlockStart(_)));
        assert!(matches!(lines[1], MarkdownLine::CodeLine(_)));
        assert!(matches!(lines[2], MarkdownLine::CodeLine(_)));
        assert!(matches!(lines[3], MarkdownLine::CodeLine(_)));
        assert!(matches!(lines[4], MarkdownLine::CodeBlockEnd(_)));
    }
}

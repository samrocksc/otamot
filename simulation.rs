fn test(notes_content: &mut String, notes_cursor_pos: usize) {
    let byte_pos = notes_content
        .char_indices()
        .nth(notes_cursor_pos)
        .map(|(i, _)| i)
        .unwrap_or(notes_content.len());

    let line_start = notes_content[..byte_pos]
        .rfind('\n')
        .map(|i| i + 1)
        .unwrap_or(0);

    // This is where line_content used to be assigned in update
    // But let's look at Shift+Tab block
    let line_content = &notes_content[line_start..];
    if line_content.starts_with("  ") {
        *notes_content = format!(
            "{}{}",
            &notes_content[..line_start],
            &notes_content[line_start + 2..]
        );
    }
}

fn main() {
    let mut s = "a🦀b".to_string();
    // char indices are 0, 1, 5
    // nth(1) gives index 1
    // nth(2) gives index 5
    test(&mut s, 1);
    test(&mut s, 2);
    test(&mut s, 0);
    println!("OK: {}", s);
}

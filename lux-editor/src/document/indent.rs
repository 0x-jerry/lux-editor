//! Enter-key indentation rules: the text Enter inserts at a caret, growing
//! inside `{` blocks and dedenting after `}`.

use super::buffer::DocumentBuffer;

const INDENT: &str = "    ";

/// Whitespace at the start of the line containing `caret_char`.
fn leading_indent(buffer: &DocumentBuffer, caret_char: usize) -> String {
    let total_chars = buffer.text().len_chars();
    if total_chars == 0 {
        return String::new();
    }
    let line_probe = if caret_char == 0 {
        0
    } else {
        caret_char
            .saturating_sub(1)
            .min(total_chars.saturating_sub(1))
    };
    let line_idx = buffer.text().char_to_line(line_probe);
    let line = buffer.text().line(line_idx).to_string();
    line.trim_end_matches(['\n', '\r'])
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect::<String>()
}

/// Text for an Enter press at `caret_char`: newline plus the next line's
/// indentation, growing inside `{` blocks and dedenting after `}`.
pub fn indentation_for_newline(buffer: &DocumentBuffer, caret_char: usize) -> String {
    let total_chars = buffer.text().len_chars();
    if total_chars == 0 {
        return "\n".to_string();
    }

    let line_probe = if caret_char == 0 {
        0
    } else {
        caret_char
            .saturating_sub(1)
            .min(total_chars.saturating_sub(1))
    };
    let line_idx = buffer.text().char_to_line(line_probe);
    let line = buffer.text().line(line_idx).to_string();
    let content = line.trim_end_matches(['\n', '\r']);
    let leading = leading_indent(buffer, caret_char);
    let trimmed = content.trim_end();

    if trimmed.ends_with('{') {
        return format!("\n{}{}", leading, INDENT);
    }

    if trimmed.starts_with('}') {
        let dedented = if leading.ends_with('\t') {
            leading.trim_end_matches('\t').to_string()
        } else if leading.ends_with(INDENT) {
            leading.trim_end_matches(INDENT).to_string()
        } else {
            String::new()
        };
        return format!("\n{}", dedented);
    }

    format!("\n{}", leading)
}

#[cfg(test)]
mod tests {
    use super::super::buffer::DocumentBuffer;
    use super::indentation_for_newline;

    fn insert(buffer: &mut DocumentBuffer, text: &str) {
        let caret = buffer.text().len_chars();
        buffer.insert(caret, text);
    }

    #[test]
    fn indentation_for_newline_grows_after_open_brace() {
        let mut buffer = DocumentBuffer::new();
        insert(&mut buffer, "fn main() {");
        let text = indentation_for_newline(&buffer, 11);
        assert_eq!(text, "\n    ");
    }

    #[test]
    fn indentation_for_newline_dedents_after_close_brace() {
        let mut buffer = DocumentBuffer::new();
        insert(&mut buffer, "}");
        let text = indentation_for_newline(&buffer, 1);
        assert_eq!(text, "\n");
    }

    #[test]
    fn indentation_for_newline_empty_buffer_is_bare_newline() {
        let buffer = DocumentBuffer::new();
        assert_eq!(indentation_for_newline(&buffer, 0), "\n");
    }

    #[test]
    fn leading_indent_collects_line_whitespace() {
        let mut buffer = DocumentBuffer::new();
        insert(&mut buffer, "  \tfoo");
        assert_eq!(super::leading_indent(&buffer, 4), "  \t");
    }
}

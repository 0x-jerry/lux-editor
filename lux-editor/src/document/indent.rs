//! Enter-key indentation rules: the text Enter inserts at a caret, growing
//! inside `{` blocks and dedenting after `}`.

use super::buffer::DocumentBuffer;

/// One indent level, for Enter and for the Tab key.
pub(crate) const INDENT: &str = "    ";

/// The line a caret splits. `None` only for an empty buffer, which has no
/// line to take an indent or a line ending from.
fn caret_line(buffer: &DocumentBuffer, caret_char: usize) -> Option<usize> {
    let total_chars = buffer.text().len_chars();
    if total_chars == 0 {
        return None;
    }
    let line_probe = if caret_char == 0 {
        0
    } else {
        caret_char
            .saturating_sub(1)
            .min(total_chars.saturating_sub(1))
    };
    Some(buffer.text().char_to_line(line_probe))
}

/// Whitespace at the start of the line containing `caret_char`.
fn leading_indent(buffer: &DocumentBuffer, caret_char: usize) -> String {
    let Some(line_idx) = caret_line(buffer, caret_char) else {
        return String::new();
    };
    buffer
        .text()
        .line(line_idx)
        .to_string()
        .trim_end_matches(['\n', '\r'])
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect::<String>()
}

/// The line ending the split line uses, so editing a CRLF file never mixes
/// line endings.
fn line_ending(buffer: &DocumentBuffer, caret_char: usize) -> &'static str {
    let Some(line_idx) = caret_line(buffer, caret_char) else {
        return "\n";
    };
    if buffer.text().line(line_idx).to_string().ends_with("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// Text for an Enter press at `caret_char`: newline plus the next line's
/// indentation, growing inside `{` blocks and dedenting after `}`.
pub fn indentation_for_newline(buffer: &DocumentBuffer, caret_char: usize) -> String {
    let Some(line_idx) = caret_line(buffer, caret_char) else {
        return "\n".to_string();
    };
    let eol = line_ending(buffer, caret_char);
    let line = buffer.text().line(line_idx).to_string();
    let content = line.trim_end_matches(['\n', '\r']);
    let leading = leading_indent(buffer, caret_char);
    let trimmed = content.trim_end();

    if trimmed.ends_with('{') {
        return format!("{eol}{}{}", leading, INDENT);
    }

    if trimmed.starts_with('}') {
        let dedented = if leading.ends_with('\t') {
            leading.trim_end_matches('\t').to_string()
        } else if leading.ends_with(INDENT) {
            leading.trim_end_matches(INDENT).to_string()
        } else {
            String::new()
        };
        return format!("{eol}{dedented}");
    }

    format!("{eol}{leading}")
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

    #[test]
    fn enter_in_a_crlf_file_keeps_crlf() {
        let mut buffer = DocumentBuffer::new();
        insert(&mut buffer, "one\r\ntwo\r\nthree");
        // Caret at the end of "two" (8 chars in): the new line must be CRLF.
        assert_eq!(indentation_for_newline(&buffer, 8), "\r\n");
        // ... and pressing Enter at the end of the file adds a CRLF too.
        assert_eq!(indentation_for_newline(&buffer, 15), "\n");

        let mut lf = DocumentBuffer::new();
        insert(&mut lf, "one\ntwo");
        assert_eq!(indentation_for_newline(&lf, lf.text().len_chars()), "\n");
    }
}

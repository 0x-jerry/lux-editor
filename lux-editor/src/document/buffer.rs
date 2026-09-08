//! The text buffer behind one document: a rope plus the file path it
//! belongs to. File loading/saving is handled by the documents pipeline
//! (`crate::documents::state`), so the buffer itself stays synchronous.

use ropey::Rope;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct DocumentBuffer {
    rope: Rope,
    path: Option<PathBuf>,
}

impl DocumentBuffer {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            path: None,
        }
    }

    pub fn text(&self) -> &Rope {
        &self.rope
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    pub fn set_path<P: AsRef<Path>>(&mut self, path: P) {
        self.path = Some(path.as_ref().to_path_buf());
    }

    pub fn line(&self, line_idx: usize) -> Option<ropey::iter::Lines<'_>> {
        if line_idx < self.rope.len_lines() {
            Some(self.rope.lines_at(line_idx))
        } else {
            None
        }
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn insert(&mut self, char_idx: usize, text: &str) {
        self.rope.insert(char_idx, text);
    }

    pub fn remove(&mut self, range: std::ops::Range<usize>) {
        self.rope.remove(range);
    }
}

impl Default for DocumentBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_remove_and_text_roundtrip() {
        let mut buffer = DocumentBuffer::new();
        assert_eq!(buffer.text().to_string(), "");
        buffer.insert(0, "hello");
        buffer.insert(5, " world");
        assert_eq!(buffer.text().to_string(), "hello world");
        buffer.remove(5..6);
        assert_eq!(buffer.text().to_string(), "helloworld");
    }

    #[test]
    fn len_lines_counts_newlines() {
        let mut buffer = DocumentBuffer::new();
        assert_eq!(buffer.len_lines(), 1);
        buffer.insert(0, "a\nb\nc");
        assert_eq!(buffer.len_lines(), 3);
    }

    #[test]
    fn line_iterates_valid_lines_and_returns_none_for_invalid() {
        let mut buffer = DocumentBuffer::new();
        buffer.insert(0, "a\nb");
        assert!(buffer.line(0).is_some());
        assert!(buffer.line(1).is_some());
        assert!(buffer.line(2).is_none());
        let mut first = buffer.line(0).unwrap();
        assert_eq!(first.next().unwrap(), "a\n");
    }

    #[test]
    fn path_is_none_for_new_buffer() {
        let buffer = DocumentBuffer::new();
        assert!(buffer.path().is_none());
    }
}

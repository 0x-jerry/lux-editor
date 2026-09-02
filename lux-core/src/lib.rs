use anyhow::Result;
use ropey::Rope;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};

#[derive(Debug)]
pub struct Buffer {
    rope: Rope,
    path: Option<PathBuf>,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            path: None,
        }
    }

    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut file = File::open(&path).await?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).await?;

        Ok(Self {
            rope: Rope::from_str(&contents),
            path: Some(path),
        })
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

    pub async fn save(&self) -> Result<()> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("buffer has no file path"))?;
        self.save_to_path(path).await
    }

    pub async fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let file = File::create(path).await?;
        let mut writer = BufWriter::new(file);
        for chunk in self.rope.chunks() {
            writer.write_all(chunk.as_bytes()).await?;
        }
        writer.flush().await?;
        Ok(())
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_remove_and_text_roundtrip() {
        let mut buffer = Buffer::new();
        assert_eq!(buffer.text().to_string(), "");
        buffer.insert(0, "hello");
        buffer.insert(5, " world");
        assert_eq!(buffer.text().to_string(), "hello world");
        buffer.remove(5..6);
        assert_eq!(buffer.text().to_string(), "helloworld");
    }

    #[test]
    fn len_lines_counts_newlines() {
        let mut buffer = Buffer::new();
        assert_eq!(buffer.len_lines(), 1);
        buffer.insert(0, "a\nb\nc");
        assert_eq!(buffer.len_lines(), 3);
    }

    #[test]
    fn line_iterates_valid_lines_and_returns_none_for_invalid() {
        let mut buffer = Buffer::new();
        buffer.insert(0, "a\nb");
        assert!(buffer.line(0).is_some());
        assert!(buffer.line(1).is_some());
        assert!(buffer.line(2).is_none());
        let mut first = buffer.line(0).unwrap();
        assert_eq!(first.next().unwrap(), "a\n");
    }

    #[test]
    fn path_is_none_for_new_buffer() {
        let buffer = Buffer::new();
        assert!(buffer.path().is_none());
    }

    #[tokio::test]
    async fn save_and_from_file_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.txt");
        let mut buffer = Buffer::new();
        buffer.insert(0, "line one\nline two");
        buffer.set_path(&path);
        buffer.save().await.unwrap();

        let loaded = Buffer::from_file(&path).await.unwrap();
        assert_eq!(loaded.text().to_string(), "line one\nline two");
        assert_eq!(loaded.path().unwrap(), &path);
    }
}

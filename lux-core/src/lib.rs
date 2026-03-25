use ropey::Rope;
use std::path::{Path, PathBuf};
use anyhow::Result;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

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

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

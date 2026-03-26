use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LanguageKind {
    PlainText,
    Extension(String),
}

impl LanguageKind {
    pub fn from_path(path: Option<&std::path::Path>) -> Self {
        let Some(path) = path else {
            return Self::PlainText;
        };
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| Self::Extension(ext.to_ascii_lowercase()))
            .unwrap_or(Self::PlainText)
    }
}

pub fn available_syntax_theme_names() -> &'static [String] {
    static THEME_NAMES: OnceLock<Vec<String>> = OnceLock::new();
    THEME_NAMES
        .get_or_init(|| {
            let mut names = ThemeSet::load_defaults()
                .themes
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            names.sort();
            names
        })
        .as_slice()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HighlightSpan {
    pub start_col: usize,
    pub end_col: usize,
    pub color: [u8; 4],
}

#[derive(Clone, Debug, Default)]
pub struct HighlightSnapshot {
    pub version: u64,
    pub line_tokens: Vec<Vec<HighlightSpan>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HighlightThemeConfig {
    pub theme_name: String,
    pub theme_path: Option<PathBuf>,
}

impl Default for HighlightThemeConfig {
    fn default() -> Self {
        Self {
            theme_name: "base16-ocean.dark".to_string(),
            theme_path: None,
        }
    }
}

enum WorkerRequest {
    Parse {
        version: u64,
        text: String,
        language: LanguageKind,
        theme: HighlightThemeConfig,
    },
    Shutdown,
}

struct WorkerResponse {
    version: u64,
    snapshot: HighlightSnapshot,
}

pub struct HighlightingService {
    request_tx: Sender<WorkerRequest>,
    response_rx: Receiver<WorkerResponse>,
    worker: Option<JoinHandle<()>>,
    latest: HighlightSnapshot,
    next_version: u64,
    theme: HighlightThemeConfig,
}

impl HighlightingService {
    pub fn new() -> Self {
        let (request_tx, request_rx) = mpsc::channel();
        let (response_tx, response_rx) = mpsc::channel();
        let worker = thread::spawn(move || worker_loop(request_rx, response_tx));
        Self {
            request_tx,
            response_rx,
            worker: Some(worker),
            latest: HighlightSnapshot::default(),
            next_version: 0,
            theme: HighlightThemeConfig::default(),
        }
    }

    pub fn set_theme(&mut self, theme: HighlightThemeConfig) {
        self.theme = theme;
    }

    pub fn request_parse(&mut self, text: String, language: LanguageKind) {
        self.next_version += 1;
        self.request_tx
            .send(WorkerRequest::Parse {
                version: self.next_version,
                text,
                language,
                theme: self.theme.clone(),
            })
            .ok();
    }

    pub fn update(&mut self) {
        while let Ok(response) = self.response_rx.try_recv() {
            if response.version >= self.latest.version {
                self.latest = response.snapshot;
            }
        }
    }

    pub fn snapshot(&self) -> &HighlightSnapshot {
        &self.latest
    }
}

impl Drop for HighlightingService {
    fn drop(&mut self) {
        self.request_tx.send(WorkerRequest::Shutdown).ok();
        if let Some(worker) = self.worker.take() {
            worker.join().ok();
        }
    }
}

fn worker_loop(request_rx: Receiver<WorkerRequest>, response_tx: Sender<WorkerResponse>) {
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();
    let mut active_theme_config = HighlightThemeConfig::default();
    let mut active_theme = resolve_theme(&theme_set, &active_theme_config);

    while let Ok(request) = request_rx.recv() {
        match request {
            WorkerRequest::Shutdown => break,
            WorkerRequest::Parse {
                mut version,
                mut text,
                mut language,
                mut theme,
            } => {
                while let Ok(next_request) = request_rx.try_recv() {
                    match next_request {
                        WorkerRequest::Shutdown => return,
                        WorkerRequest::Parse {
                            version: next_version,
                            text: next_text,
                            language: next_language,
                            theme: next_theme,
                        } => {
                            version = next_version;
                            text = next_text;
                            language = next_language;
                            theme = next_theme;
                        }
                    }
                }

                if theme != active_theme_config {
                    active_theme = resolve_theme(&theme_set, &theme);
                    active_theme_config = theme;
                }

                let snapshot = parse_snapshot(&syntax_set, &active_theme, &text, language, version);
                response_tx.send(WorkerResponse { version, snapshot }).ok();
            }
        }
    }
}

fn resolve_theme(theme_set: &ThemeSet, theme_config: &HighlightThemeConfig) -> Theme {
    if let Some(theme_path) = &theme_config.theme_path
        && let Ok(theme) = ThemeSet::get_theme(theme_path)
    {
        return theme;
    }

    theme_set
        .themes
        .get(&theme_config.theme_name)
        .cloned()
        .or_else(|| theme_set.themes.values().next().cloned())
        .unwrap_or_default()
}

fn parse_snapshot(
    syntax_set: &SyntaxSet,
    theme: &Theme,
    text: &str,
    language: LanguageKind,
    version: u64,
) -> HighlightSnapshot {
    let line_count = text.lines().count().max(1);
    let mut snapshot = HighlightSnapshot {
        version,
        line_tokens: vec![Vec::new(); line_count],
    };

    let LanguageKind::Extension(extension) = language else {
        return snapshot;
    };
    let Some(syntax) = syntax_set.find_syntax_by_extension(&extension) else {
        fill_black_fallback(&mut snapshot, text);
        return snapshot;
    };
    let mut highlighter = HighlightLines::new(syntax, theme);

    for (line_idx, line) in LinesWithEndings::from(text).enumerate() {
        if line_idx >= snapshot.line_tokens.len() {
            break;
        }
        let ranges = highlighter
            .highlight_line(line, syntax_set)
            .unwrap_or_default();
        append_ranges(&mut snapshot.line_tokens[line_idx], ranges);
    }

    snapshot
}

fn fill_black_fallback(snapshot: &mut HighlightSnapshot, text: &str) {
    for (line_idx, line) in LinesWithEndings::from(text).enumerate() {
        if line_idx >= snapshot.line_tokens.len() {
            break;
        }
        let line_len = line.trim_end_matches(['\r', '\n']).len();
        if line_len == 0 {
            continue;
        }
        snapshot.line_tokens[line_idx].push(HighlightSpan {
            start_col: 0,
            end_col: line_len,
            color: [0, 0, 0, 255],
        });
    }
}

fn append_ranges(line_tokens: &mut Vec<HighlightSpan>, ranges: Vec<(Style, &str)>) {
    let mut cursor = 0usize;
    for (style, segment) in ranges {
        let length = segment.len();
        if length == 0 {
            continue;
        }
        line_tokens.push(HighlightSpan {
            start_col: cursor,
            end_col: cursor + length,
            color: [
                style.foreground.r,
                style.foreground.g,
                style.foreground.b,
                style.foreground.a,
            ],
        });
        cursor += length;
    }
}

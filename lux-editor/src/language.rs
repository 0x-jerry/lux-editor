use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use tree_sitter::{Node, Parser, Point, Tree};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguageKind {
    PlainText,
    Rust,
}

impl LanguageKind {
    pub fn from_path(path: Option<&std::path::Path>) -> Self {
        let Some(path) = path else {
            return Self::PlainText;
        };
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("rs") => Self::Rust,
            _ => Self::PlainText,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HighlightScope {
    Keyword,
    String,
    Comment,
    Type,
    Function,
    Number,
    Constant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HighlightSpan {
    pub start_col: usize,
    pub end_col: usize,
    pub scope: HighlightScope,
}

#[derive(Clone, Debug, Default)]
pub struct HighlightSnapshot {
    pub version: u64,
    pub line_tokens: Vec<Vec<HighlightSpan>>,
}

enum WorkerRequest {
    Parse {
        version: u64,
        text: String,
        language: LanguageKind,
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
        }
    }

    pub fn request_parse(&mut self, text: String, language: LanguageKind) {
        self.next_version += 1;
        self.request_tx
            .send(WorkerRequest::Parse {
                version: self.next_version,
                text,
                language,
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
    let mut parser = Parser::new();
    let mut previous_tree: Option<Tree> = None;

    while let Ok(request) = request_rx.recv() {
        match request {
            WorkerRequest::Shutdown => break,
            WorkerRequest::Parse {
                mut version,
                mut text,
                mut language,
            } => {
                while let Ok(next_request) = request_rx.try_recv() {
                    match next_request {
                        WorkerRequest::Shutdown => return,
                        WorkerRequest::Parse {
                            version: next_version,
                            text: next_text,
                            language: next_language,
                        } => {
                            version = next_version;
                            text = next_text;
                            language = next_language;
                        }
                    }
                }

                let snapshot =
                    parse_snapshot(&mut parser, &mut previous_tree, &text, language, version);
                response_tx.send(WorkerResponse { version, snapshot }).ok();
            }
        }
    }
}

fn parse_snapshot(
    parser: &mut Parser,
    previous_tree: &mut Option<Tree>,
    text: &str,
    language: LanguageKind,
    version: u64,
) -> HighlightSnapshot {
    let mut snapshot = HighlightSnapshot {
        version,
        line_tokens: vec![Vec::new(); text.lines().count().max(1)],
    };

    if language != LanguageKind::Rust {
        *previous_tree = None;
        return snapshot;
    }

    let rust_language = tree_sitter::Language::from(tree_sitter_rust::LANGUAGE);
    if parser.set_language(&rust_language).is_err() {
        *previous_tree = None;
        return snapshot;
    }

    let tree = parser.parse(text, previous_tree.as_ref());
    let Some(tree) = tree else {
        *previous_tree = None;
        return snapshot;
    };

    let root = tree.root_node();
    collect_tokens(root, &mut snapshot.line_tokens);
    for line in &mut snapshot.line_tokens {
        line.sort_by_key(|span| span.start_col);
    }
    *previous_tree = Some(tree);
    snapshot
}

fn collect_tokens(node: Node<'_>, line_tokens: &mut [Vec<HighlightSpan>]) {
    if node.child_count() > 0 {
        let kind = node.kind();
        if is_full_span_kind(kind) {
            if let Some(scope) = scope_for_kind(kind) {
                push_span(
                    node.start_position(),
                    node.end_position(),
                    scope,
                    line_tokens,
                );
            }
            return;
        }

        for idx in 0..(node.child_count() as u32) {
            if let Some(child) = node.child(idx) {
                collect_tokens(child, line_tokens);
            }
        }
        return;
    }

    if let Some(scope) = scope_for_node(node) {
        push_span(
            node.start_position(),
            node.end_position(),
            scope,
            line_tokens,
        );
    }
}

fn is_full_span_kind(kind: &str) -> bool {
    matches!(
        kind,
        "line_comment" | "block_comment" | "string_literal" | "raw_string_literal" | "char_literal"
    )
}

fn scope_for_node(node: Node<'_>) -> Option<HighlightScope> {
    let kind = node.kind();
    if let Some(scope) = scope_for_kind(kind) {
        if scope == HighlightScope::Function && !is_function_name(node) {
            return Some(HighlightScope::Constant);
        }
        return Some(scope);
    }
    None
}

fn scope_for_kind(kind: &str) -> Option<HighlightScope> {
    if is_keyword(kind) {
        return Some(HighlightScope::Keyword);
    }
    match kind {
        "line_comment" | "block_comment" => Some(HighlightScope::Comment),
        "string_literal" | "raw_string_literal" | "char_literal" => Some(HighlightScope::String),
        "type_identifier" | "primitive_type" => Some(HighlightScope::Type),
        "integer_literal" | "float_literal" => Some(HighlightScope::Number),
        "true" | "false" => Some(HighlightScope::Constant),
        "identifier" => Some(HighlightScope::Function),
        _ => None,
    }
}

fn is_function_name(node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    if !matches!(
        parent.kind(),
        "function_item" | "function_signature_item" | "call_expression" | "generic_function"
    ) {
        return false;
    }
    let Some(name_node) = parent.child_by_field_name("name") else {
        return false;
    };
    node.start_byte() == name_node.start_byte() && node.end_byte() == name_node.end_byte()
}

fn is_keyword(kind: &str) -> bool {
    matches!(
        kind,
        "as" | "async"
            | "await"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
    )
}

fn push_span(
    start: Point,
    end: Point,
    scope: HighlightScope,
    line_tokens: &mut [Vec<HighlightSpan>],
) {
    if start.row == end.row {
        if let Some(line) = line_tokens.get_mut(start.row)
            && start.column < end.column
        {
            line.push(HighlightSpan {
                start_col: start.column,
                end_col: end.column,
                scope,
            });
        }
        return;
    }

    let max_row = line_tokens.len().saturating_sub(1);
    let end_row = end.row.min(max_row);
    for row in start.row..=end_row {
        if let Some(line) = line_tokens.get_mut(row) {
            let start_col = if row == start.row { start.column } else { 0 };
            let end_col = if row == end.row {
                end.column
            } else {
                usize::MAX
            };
            if start_col < end_col {
                line.push(HighlightSpan {
                    start_col,
                    end_col,
                    scope,
                });
            }
        }
    }
}

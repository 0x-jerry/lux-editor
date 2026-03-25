# Lux Editor

Fast Rust editor focused on large-file performance and low-latency editing.

## Current Stack
- UI/runtime: `eframe` + `egui`
- Buffer: `ropey` via `lux-core`
- Highlighting: `tree-sitter` + `syntect`
- Async/runtime services: `tokio`
- Config/theme/fonts: `config-rs`, `notify`, `font-kit`

## Run
```bash
cargo run -p lux-editor
```

## CLI
- `lux-editor <file>` opens a file
- `lux-editor <folder>` opens a workspace

# Lux Editor

Fast Rust editor focused on large-file performance and low-latency editing.

## Run
```bash
cargo run
```

## CLI
- `lux-editor <file>` opens a file
- `lux-editor <folder>` opens a workspace

## Workspace state
Reopening a workspace restores its open tabs (in order, focused tab included)
and which file-tree folders were expanded; a workspace seen for the first time
starts with its top level expanded and the editor area pointing at the tree.
State is keyed by canonical path in `recent.json`, beside `config.json` in the
settings directory.

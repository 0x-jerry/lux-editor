# Lux Editor

Fast Rust editor focused on large-file performance and low-latency editing.

## Run
```bash
cargo run
```

## CLI
- `lux-editor <file>` opens a file
- `lux-editor <folder>` opens a workspace

## Logs
Logs go to stderr and `~/.lux-editor/lux.log`; the file rotates to
`lux.log.1` once it passes 5 MiB. `RUST_LOG` overrides the configured level.

## Workspace state
Reopening a workspace restores its open tabs (in order, focused tab included)
and which file-tree folders were expanded; a workspace seen for the first time
starts with its top level expanded and the editor area pointing at the tree.
State is keyed by canonical path in `~/.lux-editor/recent.json`, beside
`config.json` in the same directory.

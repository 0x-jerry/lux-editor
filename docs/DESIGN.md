# Lux Editor Design

## Objectives
- Keep typing latency low.
- Stay responsive on large files.
- Keep architecture modular and maintainable.

## Runtime and Stack
- UI/runtime: `eframe` + `egui`
- Buffer engine: `lux-core` (`ropey`)
- Highlighting: `tree-sitter` + `syntect`
- Async and background tasks: `tokio`
- Workspace/config/watchers: `notify`, `config-rs`
- Fonts: `font-kit`

## Architecture
- **App Runtime**: `src/app/` modules own lifecycle, events, input, settings, and watcher wiring.
- **UI Layer**: `src/ui.rs` renders welcome screen, file tree panel, and virtualized editor view.
- **Workspace Layer**: `src/file_tree.rs` and `src/file_watcher.rs` handle explorer and fs changes.
- **Language Layer**: `src/language.rs` updates parse/highlight snapshots in background.
- **Config Layer**: `src/config.rs` loads settings, recents, and hot-reload paths.

## Implemented Capabilities
- Open file/folder from CLI and welcome page.
- File tree with rename/delete/new file/new folder.
- Recent item tracking.
- Virtualized line rendering.
- Syntax highlighting snapshots.
- Theme/font hot reload.
- Basic typing, backspace, tab, and newline indentation.

## Next Technical Focus
- Command palette.
- Formatter integration.
- Selection/caret model.
- Copy, cut, paste, select-all pipeline.
- Undo/redo and multi-cursor.

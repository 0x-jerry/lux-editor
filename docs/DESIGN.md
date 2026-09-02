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

## Planned UI Shell Extensions

### Configuration Page
- Add an in-app configuration page as a first-class screen in the app shell.
- Keep persisted settings in `src/config.rs` and use a staged draft state in UI.
- Apply settings through an explicit save action and support revert/discard flow.
- Organize settings by sections: appearance, editor behavior, workspace, and keybindings.
- Keep settings updates non-blocking and immediately reflected where safe.

### Custom Title Bar and Menu
- Replace native window title bar with an app-rendered title bar in `eframe`.
- Include window actions: minimize, maximize/restore, and close.
- Add a top-level menu in the title bar with entries for File, Edit, View, and Help.
- Route menu actions through the same command pipeline as shortcuts and palette.
- Expose active workspace/file context in the title bar without blocking editor input.

### UI State and Command Flow
- Introduce a shell-level view state to switch between editor and configuration page.
- Keep title bar and menu stateless where possible; dispatch actions to `App`.
- Ensure platform-specific window behavior is isolated behind a small adapter.
- Keep rendering lightweight to preserve sub-millisecond typing responsiveness.

## Implemented Capabilities
- Open file/folder from CLI and welcome page.
- File tree with rename/delete/new file/new folder.
- Recent item tracking.
- Virtualized line rendering.
- Syntax highlighting snapshots.
- Theme/font hot reload.
- Basic typing, backspace, tab, and newline indentation.
- Glyph-accurate caret, painted selection, double-click word select, caret reveal (`text_editor` view).

## Next Technical Focus
- Command palette.
- Formatter integration.
- Selection/caret model.
- Copy, cut, paste, select-all pipeline.
- Undo/redo and multi-cursor.
- Configuration page architecture and settings flow.
- Custom title bar with integrated menu system.

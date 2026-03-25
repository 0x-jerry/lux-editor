# Lux Editor Plan

## Current State
- Runtime/UI migrated to `eframe` + `egui`.
- Workspace explorer, recent items, and filesystem watchers are in place.
- Syntax highlighting, theming, and font loading are in place.
- `App` runtime has been split into `src/app/` modules.

## Active Roadmap

### Phase A: Editing Core
- Build selection and caret state model.
- Add robust range-aware insertion/deletion flow.
- Add undo/redo transaction boundaries.

### Phase B: Clipboard and Commands
- Implement copy, cut, paste, select-all command pipeline.
- Add platform clipboard adapter and failure-safe behavior.
- Connect standard keybindings through command routing.

### Phase C: Productivity Features
- Add command palette with fuzzy search.
- Integrate formatter workflow (`oxfmt`) with trigger options.
- Improve smart pairing and indentation rules.

### Phase D: Advanced Editing
- Add multi-cursor editing.
- Add regression coverage for large-file editing scenarios.
- Optimize editing operations to preserve low-latency typing.

### Phase E: App Shell and Settings UX
- Add shell view routing to support editor and configuration page screens.
- Build configuration page sections for appearance, editor behavior, workspace, and keybindings.
- Add draft-save-revert settings flow backed by persisted config.
- Replace native title bar with a custom app title bar and window controls.
- Add title bar menu entries (File/Edit/View/Help) wired to command routing.

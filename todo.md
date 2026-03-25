# Lux Editor Project Tracker

This file tracks the progress of the **Lux** editor project.

## **Project Status: Planning**

---

### **Phase 1: Core Infrastructure & Project Setup**
- [x] Initialize Rust project and dependencies `[Completed]`
- [x] Setup `eframe` runtime and native window integration `[Completed]`
- [x] Integrate `egui` for the basic UI layer `[Completed]`

### **Phase 2: Text Engine & Viewport**
- [x] Implement `ropey` buffer management `[Completed]`
- [x] Create virtualized text viewport `[Completed]`
- [x] Add async file I/O with `tokio` `[Completed]`

### **Phase 3: Workspace Management**
- [x] Implement collapsible file tree view `[Completed]`
- [x] Add file system watcher with `notify-rs` `[Completed]`
- [x] Implement context menu for file operations `[Completed]`
- [x] Only show workspace explorer when a folder is opened `[Completed]`
- [x] Implement CLI usage: `lux file.md` or `lux somefolder` `[Completed]`
- [x] Support opening files by clicking child items in the file tree `[Completed]`
- [x] Implement backend logic for `New File`, `New Folder`, and `Delete` `[Completed]`
- [x] Add a welcome page for `lux` without parameters (open file/folder, recent list) `[Completed]`

### **Phase 4: Language Intelligence**
- [x] Integrate `tree-sitter` for incremental parsing `[Completed]`
- [x] Implement background syntax highlighting service `[Completed]`
- [x] Add context-aware auto-indentation logic `[Completed]`

### **Phase 5: Configuration, Theming & Fonts**
- [x] Build hierarchical configuration system with `config-rs` `[Completed]`
- [x] Implement hot-reloadable theme engine `[Completed]`
- [x] Support custom font loading via `font-kit` `[Completed]`

### **Phase 6: Advanced Features & Polish**
- [ ] Implement fuzzy-search command palette `[Pending]`
- [ ] Integrate `oxfmt` with configurable triggers `[Pending]`
- [ ] Refine smart bracket pairing and selection wrapping `[Pending]`
- [ ] Add multi-cursor support and undo/redo history `[Pending]`

### **Phase 7: Editing Core & Clipboard UX**
- [ ] Build selection and caret engine for robust edit operations `[Pending]`
- [ ] Implement copy, cut, paste, and select-all command pipeline `[Pending]`
- [ ] Integrate platform clipboard adapter and error-safe paste behavior `[Pending]`
- [ ] Validate multi-line edit transactions and synchronization with viewport/highlighting `[Pending]`

---

## **Recent Updates**
- [2026-03-25] Refactored `lux-editor/src/app.rs` into focused `src/app/` modules for lifecycle, events, input, settings, and watchers.
- [2026-03-25] Migrated editor runtime from custom `winit`/`wgpu` loop to `eframe`, and updated architecture docs.
- [2026-03-25] Phase 5 completed: added hierarchical settings with `config-rs`, hot-reloadable theme updates, and custom font loading via `font-kit`.
- [2026-03-25] Added next-phase editing architecture and moved edit feature work into new Phase 7 planning.
- [2026-03-25] Planned basic editing feature implementation (copy/cut/paste/select all) across `docs/PLAN.md`, `docs/DESIGN.md`, and `todo.md`.
- [2026-03-25] Switched editor syntax highlighting engine to `syntect`.
- [2026-03-25] Phase 4 completed: integrated Tree-sitter incremental parsing, background syntax highlighting, and context-aware auto-indentation on Enter.
- [2026-03-25] Refactored `lux-editor/src/main.rs` by splitting application, state, UI, config, and events logic into focused modules.
- [2026-03-25] Phase 3 completed: Workspace management, Welcome Page, and Recent Items tracking.
- [2026-03-25] Integrated `rfd` for native file dialogs.
- [2026-03-25] Implemented `Config` system for recent items.
- [2026-03-25] Fixed runtime error: provided Tokio runtime for async FS operations.
- [2026-03-25] Phase 1 completed: Core infrastructure, windowing, and UI integration.
- [2026-03-25] Project named **Lux**.
- [2026-03-25] Architecture and Tech Stack designed ([DESIGN.md](docs/DESIGN.md)).
- [2026-03-25] Phased implementation plan created ([PLAN.md](docs/PLAN.md)).
- [2026-03-25] Documentation moved to `docs/` folder.

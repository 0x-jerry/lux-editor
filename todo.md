# Lux Editor Project Tracker

This file tracks the progress of the **Lux** editor project.

## **Project Status: Planning**

---

### **Phase 1: Core Infrastructure & Project Setup**
- [x] Initialize Rust project and dependencies `[Completed]`
- [x] Setup `winit` window and `wgpu` context `[Completed]`
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
- [ ] Integrate `tree-sitter` for incremental parsing `[Pending]`
- [ ] Implement background syntax highlighting service `[Pending]`
- [ ] Add context-aware auto-indentation logic `[Pending]`

### **Phase 5: Configuration, Theming & Fonts**
- [ ] Build hierarchical configuration system with `config-rs` `[Pending]`
- [ ] Implement hot-reloadable theme engine `[Pending]`
- [ ] Support custom font loading via `font-kit` `[Pending]`

### **Phase 6: Advanced Features & Polish**
- [ ] Implement fuzzy-search command palette `[Pending]`
- [ ] Integrate `oxfmt` with configurable triggers `[Pending]`
- [ ] Refine smart bracket pairing and selection wrapping `[Pending]`
- [ ] Add multi-cursor support and undo/redo history `[Pending]`

---

## **Recent Updates**
- [2026-03-25] Phase 3 completed: Workspace management, Welcome Page, and Recent Items tracking.
- [2026-03-25] Integrated `rfd` for native file dialogs.
- [2026-03-25] Implemented `Config` system for recent items.
- [2026-03-25] Fixed runtime error: provided Tokio runtime for async FS operations.
- [2026-03-25] Phase 1 completed: Core infrastructure, windowing, and UI integration.
- [2026-03-25] Project named **Lux**.
- [2026-03-25] Architecture and Tech Stack designed ([DESIGN.md](docs/DESIGN.md)).
- [2026-03-25] Phased implementation plan created ([PLAN.md](docs/PLAN.md)).
- [2026-03-25] Documentation moved to `docs/` folder.

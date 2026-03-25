# Lux Editor Todo

## Current Focus
- [x] Build selection and caret state engine
- [x] Implement copy, cut, paste, and select-all command pipeline
- [x] Add undo/redo transaction model
- [x] Add blinking caret rendering in editor view
- [x] Add bottom status bar and move caret position info
- [x] Optimize workspace watcher and apply .gitignore filtering

## Next Up
- [ ] Add command palette with fuzzy search
- [ ] Integrate formatter workflow with configurable triggers
- [ ] Refine smart bracket pairing behavior
- [ ] Add multi-cursor editing support
- [x] Add shell view routing for editor and configuration page
- [ ] Build configuration page sections and draft/save/revert settings flow
- [ ] Replace native title bar with custom app title bar and window controls
- [ ] Add title bar menu and route actions through command pipeline

## Notes
- Runtime/UI use `eframe` + `egui`
- App lifecycle logic is split under `lux-editor/src/app/`

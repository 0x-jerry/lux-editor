# Lux Editor Todo

## Current Focus
- [ ] Build selection and caret state engine
- [ ] Implement copy, cut, paste, and select-all command pipeline
- [ ] Add undo/redo transaction model

## Next Up
- [ ] Add command palette with fuzzy search
- [ ] Integrate formatter workflow with configurable triggers
- [ ] Refine smart bracket pairing behavior
- [ ] Add multi-cursor editing support
- [ ] Add shell view routing for editor and configuration page
- [ ] Build configuration page sections and draft/save/revert settings flow
- [ ] Replace native title bar with custom app title bar and window controls
- [ ] Add title bar menu and route actions through command pipeline

## Notes
- Runtime/UI use `eframe` + `egui`
- App lifecycle logic is split under `lux-editor/src/app/`

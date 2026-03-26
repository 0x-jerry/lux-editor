# Lux Editor Todo

## Current Focus
- [x] Add command palette with fuzzy search
- [ ] Integrate formatter workflow with configurable triggers
- [ ] Refine smart bracket pairing behavior
- [ ] Add multi-cursor editing support
- [ ] Build configuration page sections and draft/save/revert settings flow
- [ ] Replace native title bar with custom app title bar and window controls
- [ ] Add title bar menu and route actions through command pipeline

## Notes
- Runtime/UI use `eframe` + `egui`
- App lifecycle logic is split under `lux-editor/src/app/`
- [x] Expanded syntax highlighting to use extension-based syntect lookup and default unsupported file text to black
- [x] Added VSCode-style `Open Recently` command flow in command palette
- [x] Added `Recent Used` command section in command palette (up to 5 commands)
- [x] Added file save workflow with `Cmd/Ctrl+S`, command palette support, and dirty-state status feedback

# Lux Editor Todo

## Completed
- [x] Replace `text_area` view with `text_editor`: galley-based rendering, glyph-accurate caret, painted selections, double-click word select, vertical caret reveal.
- [x] Remove direct `egui` dependency — route all `egui::` usage through the `eframe::egui` re-export (single source of truth for the egui version).

## Current Focus
- [ ] Integrate formatter workflow with configurable triggers
- [ ] Refine smart bracket pairing behavior
- [ ] Add multi-cursor editing support
- [ ] Replace native title bar with custom app title bar and window controls

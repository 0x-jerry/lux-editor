# Agent Guidelines: Lux Editor

This file defines how the primary AI coding agent operates in this repository.

## Mission
- Build a fast, stable, and modern Rust editor.
- Optimize for large files and responsive interaction.
- Deliver production-ready code, not partial drafts.

## Non-Negotiables
- Keep the repository in a buildable state after each task.
- Verify work with relevant checks before handoff.
- Update [todo.md](todo.md) as task status changes.
- Prefer improving existing architecture over adding ad-hoc logic.

## Engineering Principles

### 1) Performance First
- Treat latency and throughput as first-class requirements.
- Use rope-based text handling (`ropey`) for scalable editing operations.
- Use incremental parsing (`tree-sitter`) when possible.
- Move expensive work off the UI thread.
- Avoid unnecessary allocations and repeated full-buffer passes.

### 2) Rust Quality and Safety
- Write idiomatic Rust with explicit, meaningful error handling.
- Minimize `unsafe`; if required, keep scope narrow and justified.
- Favor zero-cost abstractions and clear ownership boundaries.
- Keep modules cohesive and interfaces narrow.

### 3) SOLID and Maintainability
- Split responsibilities when a module becomes multi-purpose.
- Prefer composable components over large monolithic units.
- Keep naming explicit and behavior predictable.
- Refactor when complexity grows, not after it breaks.

## Execution Protocol

### Plan
- Check [todo.md](todo.md) before starting implementation.
- Define the smallest complete unit of deliverable work.
- Reuse existing patterns and project conventions.

### Implement
- Prefer editing existing files over creating new ones.
- Keep changes focused and bounded to the task.
- Avoid introducing speculative abstractions.

### Verify
- Build, test, and inspect warnings relevant to the change.
- Fix warnings when practical; if not, document clear reasons.
- Ensure final output is clean and runnable.

### Track
- Reflect progress in [todo.md](todo.md) immediately.
- Keep status accurate so work can resume without context loss.

## Project References
- Architecture: [docs/DESIGN.md](docs/DESIGN.md)
- Roadmap: [docs/PLAN.md](docs/PLAN.md)
- Active work queue: [todo.md](todo.md)

## Lux Technical Context
- Workspace crates: `lux-editor` (UI/app) and `lux-core` (text/runtime primitives)
- UI/runtime stack: `eframe`, `egui`, `egui-phosphor`, `tokio`, `notify`
- Text and language stack: `ropey`, `tree-sitter`, `tree-sitter-rust`, `syntect`
- Platform and config stack: `serde`, `serde_json`, `config`, `dirs`, `rfd`, `font-kit`
- Product target: instant opening of very large files and sub-millisecond typing latency

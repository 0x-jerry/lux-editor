# Lux Editor → gpui-kit migration plan

Status: in progress — M0–M3 done, M4/M5 nearly done, M6 not started (see §11)
Target crate: `gpui-kit` 0.7.3 (pins GPUI; bundles `gpui-base` + styled `gpui-component` + Lucide assets)

Decisions locked in:
1. **Full in-place rewrite**, committed directly (no feature branch).
2. **Adopt gpui-kit's Code Editor** (tree-sitter + ropey).
3. **Editor-authoritative** buffer model (gpui-kit's `EditorState` owns text/highlighting).
4. **Lucide SVGs** replace the `egui-phosphor` + `devicons` Nerd Font icons.
5. **Tray icon is not a hard requirement** (keep it only if cheap; no cross-platform tray mandate).

---

## 1. Goal

Replace `egui`/`eframe` (and `egui-phosphor`, `egui_extras`, `egui_commonmark_backend`) with
`gpui-kit`, keeping Lux Editor's behaviour: large-file editing, workspace/tree, tabs,
command palette, settings UI, markdown preview, image/binary viewers, themes, and native
menubar.

## 2. Why this is a rewrite, not a swap

egui is **immediate mode** (rebuild the whole UI every frame from `&mut Ui`); GPUI is
**retained mode** (persistent `Entity` state, a `Render` trait that returns an element tree,
`Context<Self>` + `cx.notify()` for invalidation, actions/keybindings for input, an async
executor for tasks). There is no compatibility layer. Every render path changes shape.

Scale: 118 Rust files, ~16.7k lines. 60 files touch `egui`/`eframe` (720 references),
218 test markers, many of which spin up a headless `egui::Context`.

## 3. What survives vs. what is rewritten

**Survives largely intact (domain / pure logic):**
- `document/{buffer, document, edit_history, caret_state, formatter, indent, input/*}` — rope model, edit history, command mapping
- `settings/{types, store, schema, mod}`
- `workspace/{tree, watcher}`
- `theme/{color, choice, file}` (data); `assets/themes/*.json` (converted, see §7)
- `events.rs`, `paths.rs`, `logging.rs`, `atomic.rs`, `build.rs`

**Rewritten (presentation + plumbing):**
- `main.rs` — `eframe::run_native` → `gpui_kit::application().with_assets(...).run()` + `open_window(WindowOptions)`
- `app/{update, state, context, runtime}.rs` — the eframe `App` pass and the `egui::Context` that currently doubles as the wake/repaint handle *everywhere*
- `app/actions/{chrome, editing, settings, documents, workspace, highlighting}.rs` — input, shortcuts, clipboard, viewport commands, native-menu plumbing
- `component.rs` — the `Component::render(&mut self, ui, input) -> Vec<Message>` trait
- `theme/{apply, visuals}.rs`, `theme/{mod, builtin}.rs` wiring
- `highlighting/{state, paint}.rs` + `highlighting/service/*` wake plumbing (see §7 — likely deleted)
- `chrome/ui/*` (shell, app_view, title_bar, status_bar, welcome, about, close_prompt, command_panel/*, widgets/*, file_image, file_binary, file_missing, frame_input, workspace_start)
- `tabs/{strip.rs, ui/editor.rs}`
- `document/ui/*` (text_editor/{row,gutter,metrics,mod}, markdown/*, scroll_sync)
- `workspace/file_tree_panel.rs`, `settings/configuration_view.rs`
- `native/{mod, menubar, tray, icon}.rs`

## 4. Dependency changes

Remove: `eframe`, `egui-phosphor`, `egui_extras`, `egui_commonmark_backend`, `font-kit`,
`pulldown-cmark`, `image`, and (probably) `tree-sitter*`, `tree-sitter-highlight` once the
editor stack owns highlighting.

Add:
```toml
gpui-kit = { version = "0.7.3", features = [
    "tree-sitter-rust", "tree-sitter-javascript", "tree-sitter-typescript",
    "tree-sitter-markdown", "tree-sitter-json", "tree-sitter-yaml",
    "tree-sitter-bash", "tree-sitter-html", "tree-sitter-python",
    # toml/xml: verify feature availability; else register a custom LanguageConfig
] }
```
Keep: `tokio`, `ropey` (editor also uses it), `serde`, `serde_json`, `dirs`, `notify`, `rfd`,
`ignore`, `arboard` (or use gpui's clipboard), `muda`, `rolling-file`, `log`, `env_logger`.
Remove `devicons` (replaced by Lucide SVGs from gpui-kit's `assets` feature).
`tray-icon`: optional — drop unless it stays cheap (tray is not a requirement).
Re-validate: workspace `panic = "abort"` + LTO profile vs. GPUI; tree-sitter version
alignment (GPUI kit pins `tree-sitter ^0.26.13`).

## 5. egui → gpui-kit mapping

| egui / eframe today | gpui-kit replacement |
| --- | --- |
| `eframe::run_native`, `NativeOptions`, `ViewportBuilder` | `application().with_assets(assets::Assets).run(...)`, `init(cx)`, `open_window(WindowOptions, cx, ...)` |
| `egui::Context` (wake + input + viewport + clipboard) | `Entity` + `cx.notify()`; `Window` for viewport; `App` clipboard; `Action`/`KeyBinding` for input |
| `Component::render(&mut Ui, input) -> Vec<Msg>` | `Render`/`RenderOnce`; `cx.listener(...)`; `EventEmitter`/subscriptions |
| `egui::Panel::left/right`, `Frame::side_top_panel` | `Dock` (serializable splits) or `flex` + `resizable` |
| `egui::ScrollArea` | `overflow_scroll` + `ScrollHandle`; `VirtualList` for the tree |
| `egui::TextEdit::singleline` | `Input` + `InputState` |
| `egui::ComboBox` | `Select` / `Combobox` |
| `egui::DragValue` | `NumberInput` / `Slider` |
| `egui::Modal`, `Window` overlays | `window.open_dialog` / `Sheet` / `Popover` / `Root` layers |
| `egui::text::LayoutJob` + `TextFormat` | editor `HighlightTheme`/`SyntaxColors` + highlighter; or styled `TextRun` |
| `egui_phosphor` + `devicons` Nerd Font | Lucide `IconName` SVGs (`assets` feature) |
| `egui_commonmark_backend` + `pulldown-cmark` | gpui-kit native Markdown element |
| `egui_extras` image loader + `image` | gpui `img()`/`ImageSource` + kit assets |
| `KeyboardShortcut` + `input_mut(consume_shortcut)` | `Action` + `KeyBinding` |
| `ctx.send_viewport_cmd(Title/Visible/Focus/Close)` | `Window` methods (`set_window_title`, etc.) |
| `font-kit` custom family loading | gpui TextSystem font registration (kit *Fonts* doc) |
| `muda`/`objc2` hooked into egui loop | GPUI menus + `native-extension` doc (tray optional) |

## 6. Staged milestones

**M0 — Spike (gate)**
Work directly on the current branch (no feature branch). In a scratch binary (or on `main.rs`)
open a gpui-kit window and prove: custom font loading, an image tab, markdown rendering,
menubar integration, and loading a **large real file** into `EditorState`
(`code_editor(lang).line_number(true)`). Go/no-go on the editor before touching the app.

**M1 — Skeleton & shell**
`main.rs` (window options: undecorated / macOS `fullsize_content_view`), `app/state.rs`
(`App` as an `Entity`), replace the eframe pass in `app/update.rs` with `impl Render`, wire
theme + fonts from `assets/themes/*.json`, and rebuild the frame: title bar, status bar,
sidebar/content split.

**M2 — Workspace**
`workspace/file_tree_panel.rs` → tree (`VirtualList`/`Tree`), `chrome/ui/welcome.rs` +
`workspace_start.rs`, open/close folder/workspace actions, native menus.

**M3 — Editor (largest)**
Adopt gpui-kit's editor as the authoritative buffer (see §7.1). Port gutter/line numbers/
whitespace/folding from `settings` config; drop the custom `text_editor/*` and `scroll_sync`;
make Lux's `DocumentBuffer`/`CaretState`/`EditHistory` adapters over `EditorState`; re-point
highlighting to the editor's highlighter + converted `SyntaxColors`.

**M4 — Remaining UI**
Tabs (`tabs/strip.rs`), command palette (`chrome/ui/command_panel/*`), settings UI
(`settings/configuration_view.rs`), overlays (about, close prompt), markdown preview,
image/binary/missing-file views.

**M5 — Native & input**
Menubar, clipboard, keybindings/actions, macOS `objc2` activation, close-request/save gate.
Tray only if cheap (not a requirement).

**M6 — Tests, cleanup, perf**
Port the 218 tests to the gpui-kit test harness; delete dead deps/modules; validate large-file
performance and packaging (`panic="abort"`, LTO, stripped release).

## 7. Hard problems & mitigation

1. **Editor ownership model — DECIDED: editor-authoritative.** gpui-kit's `EditorState` owns
   the text (ropey) and highlighting. Lux's `DocumentBuffer`/`CaretState`/`EditHistory` become
   adapters over it: mirror the editor value for disk save/dirty/reload and status, and drive
   file loads via `set_value`/`apply_edits`. Rewrite `document/input/*` as `Action`s the editor
   consumes. Validate on a real 100k+ line file in M0 before committing the app-wide rewrite.
2. **Highlighting stack.** Adopting the editor implies using gpui-kit's tree-sitter
   highlighter, so `highlighting/{service, state, paint}` are likely deleted, and custom assets
   (`assets/highlights/typescript.scm`, `typescript-tsx.scm`) are replaced by the built-in
   registry. **Verify XML support** (not in the advertised feature list) — else register a
   custom `LanguageConfig` with `tree-sitter-xml`.
3. **Wake handle everywhere.** `egui::Context` is cloned into the highlight worker, settings
   watcher, workspace watcher, and `Runtime`. Replace each with an `Entity`/`AsyncApp` notify
   (`cx.notify()`), or a small channel drained in an entity's update loop.
4. **Native menubar.** `muda`/`tray-icon`/`objc2` currently hook the egui loop with a
   repaint callback. GPUI exposes menus and a `native-extension` mechanism. Tray is **not a
   requirement** — port the menubar; keep `tray-icon` only if it drops in cheaply, otherwise
   drop it.
5. **Theming.** Convert `assets/themes/{dark,light}.json` (`colors` + `syntax.foreground/
   background/tokens`) into gpui-kit's Zed-style `HighlightTheme` (`editor.*` keys, flattened
   status colors, `syntax.*`). Write a converter + a round-trip test so custom themes keep
   working.
6. **Icons.** Replace `egui-phosphor` + the `devicons` Nerd Font family with Lucide SVG
   `IconName`s (file-type icons come from the icon set / svg assets rather than a glyph font).
7. **Input/IME/shortcuts.** `consume_shortcut`, `egui_wants_keyboard_input`,
   `text_edit_focused`, and the "re-inject event into a focused TextEdit" hack in
   `actions/chrome.rs` all become actions/keybindings + `Window::focus`/focus handles.
8. **Toolchain & packaging.** gpui-kit pins a GPUI snapshot; platform build deps (Wayland/X11,
   MSVC) and the `panic="abort"`/LTO profile need re-validation. macOS requires the `font-kit`
   feature on `gpui-pre-platform` (gpui-kit enables it).
9. **Tests.** Many tests assert on egui shapes/fonts (e.g. `fonts().families()`); port to
   `#[gpui_kit::test]` windows asserting state/focus/layout instead.

## 8. Testing strategy

- Keep domain unit tests (buffer, edit history, indent, settings schema, tree, paths).
- Replace UI tests with gpui-kit's headless test windows (`#[gpui_kit::test]`): mount a view,
  drive pointer/keyboard, assert state/focus/accessibility.
- Add a theme-conversion round-trip test and a large-file smoke test.

## 9. Risks

- **Effort:** multi-week rewrite; the editor + input layer dominate.
- **Behaviour drift:** large-file performance and IME/input edge cases are the highest risk.
- **Feature gaps:** XML grammar, custom highlight queries, exact font-family selection. (Tray
  and Nerd-Font icons are out of scope by decision.)
- **Dependency churn:** GPUI snapshot pins, tree-sitter major versions, `panic="abort"`.

## 10. Resolved decisions

1. Editor ownership: **editor-authoritative** (gpui-kit `EditorState` owns the buffer).
2. Icons: **Lucide SVGs** (drop `egui-phosphor` + `devicons`).
3. Tray icon: **not required**; port the menubar only.
4. **No feature branch** — commit the rewrite directly on the current branch.
5. Doc path: **`docs/gpui-kit-migration.md`**.

## 11. Progress

The rewrite is committed directly on `main`, one milestone per commit:

| Commit | Milestone |
| --- | --- |
| `c70d3ee` | this plan |
| `2ced621` | M0/M1 — gpui-kit skeleton, window, theme bridge |
| `1547bcf` | M1 — `Wake` handle replaces `egui::Context` as the repaint handle (see §7.3) |
| `ccc7d3d` | M1 — the app logic pass runs from the gpui root view |
| `34ae5fd` | M2/M3 — document model, file tree, editor shell on `EditorState` |
| `b8ff053` | M1/M5 — title bar, app menu bar, status bar, save/close/dirty |
| `1c76864` | M4/M5 — command palette, markdown preview, image/notice tabs, settings tab, close gate |
| `ff0f761` | M6 — the legacy egui tree and its dependencies are deleted |
| `705c8e9` | M4 — the real settings page |
| `46adf44` | M2/M4 — workspace session restore, settings-file watcher |
| `7ad82d7` | M4 — exclusion changes apply to the open tree |
| `6e54a40` | M4 — recent items in the command palette |
| `81e2d82` | M4 — the external formatter, on save and on demand |

**Landed.** `lux-editor/src/gpui_ui/` is the only presentation layer: `root.rs` (shell, actions,
save gate, background pumping), `tabs.rs`/`document.rs` (tab model over the kit's
`EditorState`), `views.rs` (file classification, image and notice bodies), `markdown.rs`
(preview), `palette.rs` (command palette with the recent items), `settings_view.rs` (settings
pages), `theme.rs` (chrome palette), `actions.rs` (actions, key bindings, menus),
`runtime.rs`/`wake.rs` (watcher channel). The egui tree, `Cargo.toml`'s eframe stack and the
tokio runtime handle are gone; `cargo check --workspace --all-targets` is warning-free and
`cargo test -p lux-editor --bins` runs 30 tests.

**Still to do.**
1. **Tests (§8).** 30 tests remain, covering what is left of the domain plus the new
   classification tests: `gpui_ui::views::classify` (text/binary/image/missing), `theme`
   parsing and choice resolution, `settings` types/store, `workspace` tree/watcher, `paths`,
   `atomic`, `formatter` (its Unix-only cases gated). The 218 egui-era tests and the harness
   they needed are gone with the code they asserted; headless `#[gpui_kit::test]` coverage for
   the views, the palette and the save gate is **not written** (it needs gpui-kit's
   `test-support` feature, which is not enabled).
2. **Large-file performance (§9).** Files are read, classified and loaded on the UI thread;
   nothing has been measured on a 100k+ line file since the rewrite, and packaging
   (`panic="abort"`, LTO, stripped release) is unvalidated.
3. **Theme mapping (§7.5).** Chrome colours come from `assets/themes/*.json`; editor syntax
   colours and the kit's own widget surfaces come from gpui-component's built-in `Theme`
   (left at its defaults). Mapping Lux's theme file onto that `Theme` — and converting it into
   a `HighlightTheme` for custom syntax colours, with a round-trip test — is still open, which
   is also why five palette tokens are currently held but unread.
4. **Formatting is synchronous.** The formatter runs on the save path, so the UI waits for the
   external process; moving it to a background task needs a completion event and a staleness
   check against the buffer text.
5. **A macOS-native menubar (§7.4)** is not implemented: the app menu is the in-window
   `AppMenuBar`.

**Deviations found while implementing.**
1. `gpui_component` items are **not** re-exported from `gpui_kit`'s root; they must be imported
   as `gpui_kit::component::…` (only `gpui-base` + `gpui` are globbed).
2. There is no reusable host command palette: Lux owns its `CommandState` and renders the
   palette **inline** as an absolutely positioned overlay, so a chosen command bubbles back
   through `Root`'s action handlers. Recent-item rows carry no action, so confirming one
   resolves the index path back to a path, and the root opens it on the next render (the
   palette's event handler has no window).
3. `TextView::markdown` renders GFM and math but does not forward `image_source` or
   `code_block_highlighter`, so markdown images and custom code-block themes are unthemed.
4. The save gate needs three registrations, not one: `window.on_window_should_close` covers the
   OS close (and `Quit` must gate itself, because `App::quit` never consults that callback),
   while the title bar's own close button calls `window.remove_window()` and is gated by
   `TitleBar::on_close_window`.
5. `EditorState::set_value` clears the undo history and emits no change event, so the
   formatter path restores the dirty flag by hand.
6. The theme JSON's `syntax` section is no longer parsed; the chrome tokens are, and the
   built-in `Theme` covers code colours until §7.5 above is done.

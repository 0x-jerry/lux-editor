# Lux Editor - Implementation Plan

This document outlines the phased implementation plan for **Lux**, the high-performance Rust editor, based on the architecture defined in `DESIGN.md`.

---

## **Phase 1: Core Infrastructure & Project Setup**

*Goal: Establish a basic window that can render a UI and handle user input.*

1.  **Initialize Project**:
    -   Run `cargo new editor` to create a new Rust project.
    -   Add initial dependencies to `Cargo.toml`: `winit`, `wgpu`, `egui`, `egui-wgpu`, `egui-winit`, `tokio`.

2.  **Setup Windowing & Rendering Loop**:
    -   Create a `main.rs` that initializes a `winit` window.
    -   Set up a `wgpu` graphics context to render to the window.
    -   Implement a basic event loop to handle window resizing, closing, and keyboard/mouse input.

3.  **Integrate `egui`**:
    -   Integrate `egui-wgpu` and `egui-winit` to create a basic UI layer.
    -   Render a simple "Hello, World" `egui` panel to confirm the integration is working.

---

## **Phase 2: Text Engine & Viewport**

*Goal: Display and edit text from a file using a rope data structure.*

1.  **Integrate `ropey`**:
    -   Create a `Buffer` struct that wraps a `ropey::Rope`.
    -   Implement methods to load a file into the `Buffer` and to retrieve its contents.

2.  **Implement a Basic Text Viewport**:
    -   Create an `egui` widget that displays the text from the `Buffer`.
    -   Implement virtual scrolling: only fetch and render the lines currently visible in the viewport.
    -   Handle basic keyboard input to insert and delete characters in the `ropey` buffer.

3.  **File I/O**:
    -   Implement `File -> Open` and `File -> Save` functionality using `tokio` for asynchronous file operations.

---

## **Phase 3: Workspace Management**

*Goal: Implement a file explorer and workspace-level functionality.*

1.  **File Tree View**:
    -   Create a new `egui` panel for the workspace explorer.
    -   Implement logic to recursively scan a directory and display its contents in a collapsible tree structure.
    -   **Workspace Visibility**: Only show the file tree when a folder is opened.
    -   **File Opening**: Implement the click event on file tree items to open the selected file in the main editor view.

2.  **File System Watching**:
    -   Integrate `notify-rs` to watch the workspace directory for changes.
    -   Implement handlers to automatically refresh the file tree when files or folders are created, deleted, or renamed.

3.  **CLI Usage**:
    -   Update the entry point to parse command-line arguments.
    -   Support `lux <file>` to open a specific file.
    -   Support `lux <folder>` to open a folder as a workspace and show the explorer.

4.  **Context Menu Operations**:
    -   Add a right-click context menu to the file tree items.
    -   Implement the backend logic for `Rename`, `Delete`, `New File`, and `New Folder` operations.

5.  **Welcome Page (No Parameter Mode)**:
    -   Implement a welcome view when `lux` is started without a file or folder path.
    -   Provide large, accessible buttons to "Open File" and "Open Folder".
    -   Maintain a "Recent Files/Folders" list in the editor's configuration.
    -   Display the recent list on the welcome page, allowing one-click access.

---

## **Phase 4: Language Intelligence**

*Goal: Add syntax highlighting and smart indentation.*

1.  **Integrate `tree-sitter`**:
    -   Add `tree-sitter` and a language grammar (e.g., `tree-sitter-rust`) to the project.
    -   Create a `HighlightingService` that runs in a background thread.
    -   This service will parse the buffer content with `tree-sitter` and generate a set of highlight tokens (e.g., `(line, col, length, scope)`).

2.  **Apply Syntax Highlighting**:
    -   The rendering layer will use the tokens from the `HighlightingService` to color the text.
    -   Initially, map scopes to hardcoded colors.

3.  **Implement Context-Aware Indentation**:
    -   When the `Enter` key is pressed, query the `tree-sitter` syntax tree at the cursor's position.
    -   Implement the logic described in `DESIGN.md` to automatically indent or create new lines based on the syntactic context.

---

## **Phase 5: Configuration, Theming & Fonts**

*Goal: Make the editor customizable.*

1.  **Configuration System**:
    -   Integrate `config-rs` and `serde`.
    -   Define a `Settings` struct that can be deserialized from a `config.toml` file.
    -   Load settings on startup and watch for changes using `notify-rs` for hot-reloading.

2.  **Theme Engine**:
    -   Create a `Theme` struct that maps syntax scopes (e.g., `"keyword"`, `"string"`) to colors.
    -   Load a theme file (JSON or TOML) based on the `theme` setting in `config.toml`.
    -   The rendering layer will now use the active theme's colors instead of hardcoded ones.

3.  **Custom Font Rendering**:
    -   Integrate `font-kit`.
    -   Load the font specified in the `font_family` configuration setting.
    -   Pass the loaded font to `cosmic-text` (or the `egui` equivalent) to be used for rendering all text.

---

## **Phase 6: Advanced Features & Polish**

*Goal: Implement the command palette, formatting, and other smart features.*

1.  **Command Palette**:
    -   Create a `CommandRegistry` to register actions the user can perform.
    -   Build the command palette UI as a modal overlay.
    -   Use `fuzzy-matcher` to filter commands as the user types.

2.  **`oxfmt` Integration**:
    -   Integrate the `oxc_format` library.
    -   Create a `FormattingService` that can format a buffer's content.
    -   Implement the `format_on_save` and `format_on_type` triggers based on the user's configuration.

3.  **Refine Smart Editing**:
    -   Implement the full bracket and quote pairing logic as described in `DESIGN.md`.
    -   Add multi-cursor support.
    -   Implement undo/redo functionality for the text buffer.

# Lux Editor - High-Performance Design

This document outlines the architecture, tech stack, and key libraries for **Lux**, a high-performance code editor built in Rust.

## **Core Objectives**
- **Speed**: Instant file loading and zero-latency typing.
- **Scalability**: Handle files gigabytes in size without memory exhaustion.
- **Modernity**: Built-in syntax highlighting, formatting, and smart editing features.

---

## **Tech Stack & Libraries**

### **1. Core Text Engine (Large File Support)**
To achieve "instant" loading for large files, we use a **Rope** data structure. Unlike a standard string, a rope splits text into smaller nodes, allowing O(log N) insertions and deletions.

- **[ropey](https://github.com/cessen/ropey)**: The industry standard for Rust text editors. It provides an efficient, thread-safe rope implementation that can handle files larger than memory.
- **Memory Mapping**: For extremely large files, we can use `memmap2` to map the file into memory and load it into the rope lazily or in chunks.

### **2. Syntax Highlighting**
- **[Tree-sitter](https://tree-sitter.github.io/tree-sitter/)**: A modern, incremental parsing library.
    - **Incremental**: Only re-parses the parts of the file that changed, making it perfect for real-time highlighting in large files.
    - **C-Bindings**: Fast and provides high-quality syntax trees for almost all languages.
- **[tree-sitter-highlight](https://github.com/tree-sitter/tree-sitter/tree/master/highlight)**: A crate for performing syntax highlighting using Tree-sitter queries.

### **3. Formatting (oxfmt)**
- **[Oxc (The Oxidation Compiler)](https://github.com/oxc-project/oxc)**: A collection of high-performance tools for JavaScript and TypeScript.
- **oxfmt**: The formatter part of Oxc, designed to be 10-100x faster than Prettier. We will integrate this as the primary formatter for supported languages.

### **4. UI & Rendering Layer**
- **[egui](https://github.com/emilk/egui)** or **[Iced](https://github.com/iced-rs/iced)**:
    - **egui**: An immediate-mode GUI that is extremely fast and easy to integrate with `wgpu` for hardware-accelerated rendering.
    - **cosmic-text**: For high-performance text shaping and layout, ensuring smooth rendering across all platforms.
    - **[font-kit](https://github.com/servo/font-kit)**: A cross-platform library for loading fonts from the system or files, used to enable custom font rendering.
- **[wgpu](https://github.com/gfx-rs/wgpu)**: The underlying graphics API for cross-platform GPU acceleration.

### **5. Configuration & Theme System**
- **[Serde](https://serde.rs/)**: For efficient serialization and deserialization of configuration files (TOML/JSON).
- **[config-rs](https://github.com/mehcode/config-rs)**: For managing hierarchical configurations (default, user-level, project-level).
- **[Syntect](https://github.com/trishume/syntect)** (Optional integration): While Tree-sitter is used for parsing, `syntect`'s theme formats (.tmTheme) can be mapped to Tree-sitter scopes for broad theme compatibility.

### **6. Command Palette**
- **[Fuzzy-matcher](https://github.com/lotabout/fuzzy-matcher)**: For lightning-fast filtering of commands in the palette.
- **egui/Iced Modal**: A centered modal overlay for the palette UI.

### **7. Concurrency & Runtime**
- **[Tokio](https://tokio.rs/)**: For asynchronous I/O and managing background tasks like formatting and file saving.
- **[Rayon](https://github.com/rayon-rs/rayon)**: For parallelizing heavy computations like global search or initial file indexing.

---

## **Architectural Design**

The editor follows a **Modular Layered Architecture**:

### **A. Buffer Management Layer**
Manages the `Ropey` instances. Each open file is a "buffer".
- Tracks cursor positions (multi-cursor support).
- Manages the undo/redo history (using a transaction-based approach).
- Handles basic editing logic (e.g., bracket autocomplete).

### **B. Language Layer**
Provides intelligence to the editor.
- **Highlighting Service**: Runs Tree-sitter in the background to provide style tokens for the current viewport.
- **Formatting Service**: Invokes `oxfmt` (or other formatters) on save or command.
- **Autocomplete Service**: Implements basic logic for bracket pairs (e.g., typing `(` inserts `)` automatically).

### **C. Workspace Management Layer**
- Manages the collection of files and folders in the current project.
- **File Tree**: Maintains a real-time, hierarchical view of the workspace directory.
- **File Watcher**: Uses `notify-rs` to watch for filesystem changes (create, delete, rename) and updates the file tree automatically.
- **State**: Tracks the currently open files and the active editor view.

### **D. Global State & Config Layer**
- **Config Management**: Centralized store for user preferences.
    - **Recent Items**: Stores an array of paths to recently opened files and folders.
- **Theme Engine**: Maps style tokens (from Tree-sitter) to RGBA values based on the active theme.
- **Command Registry**: A central place to register and execute commands (Open, Save, Format, Theme Switch).

### **E. Rendering Layer**
- **Viewport Management**: Only renders the lines currently visible on the screen (virtual scrolling).
- **Shaping**: Uses `cosmic-text` to convert text into glyphs.
- **Rasterization**: Uses the GPU to draw glyphs and UI elements.

---

## **Feature Implementation Details**

### **1. Instant Large File Loading**
- Use `ropey` to load the file.
- The UI layer only requests the lines needed for the current viewport (e.g., lines 100-150).
- Tree-sitter performs a "quick parse" of the visible range first, then finishes the rest of the file in a background thread.

### 2. Welcome Page (No Workspace Mode)
- Displayed when the editor is opened without a file or folder path.
- **Actions**:
    - **Open File**: Triggers a native file selection dialog.
    - **Open Folder**: Triggers a native folder selection dialog.
- **Recent Items List**:
    - Displays a clickable list of the most recently accessed files and folders.
    - Paths are stored in the user's global configuration.
    - Items are sorted by access time (most recent first).

### 3. Smart Auto-Completion

#### A. Bracket & Quote Pairing
- Implemented as a "middleware" in the **Buffer Management Layer**.
- When an opening delimiter (`(`, `[`, `{`, `"`, `'`, `` ` ``) is typed, the corresponding closing delimiter is automatically inserted.
- If text is selected, typing an opening delimiter wraps the selection with the pair.
- Typing a closing delimiter when the cursor is immediately before it will "overwrite" it by simply moving the cursor forward.
- Pressing backspace on an empty pair (e.g., `()|`) deletes both delimiters.

#### B. Context-Aware Auto-Indentation
- This feature is powered by **Tree-sitter**'s syntax tree for semantic understanding.
- **On `Enter` Key Press**:
    - The editor queries the syntax node at the cursor's position.
    - If the cursor is inside a block-level element (e.g., `{...}`), the new line's indentation is increased by one level.
    - If the cursor is between a pair like `{}`, it creates a new indented line and moves the closing `}` to its own correctly indented line.
- **On `Tab` / `Shift+Tab`**:
    - Indents or de-indents the current line or selected lines by one level.

### 4. oxfmt & Formatting Triggers
- **Trigger Options**:
    - **Manual**: Triggered via Command Palette or keybinding.
    - **On-Save**: Automatically formats the file before writing to disk.
    - **On-Type**: Formats the current line or block after a configurable delay.
- **Implementation**: The buffer content is sent to `oxc_format`, and the diff is applied back to the `ropey` instance as a single transaction (to preserve undo history).

### 5. Configuration & Theme Support
- **Theme Support**: The editor uses a JSON/TOML file to define colors for Tree-sitter scopes (e.g., `keyword`, `function`, `string`). Themes can be hot-swapped via the Command Palette.
- **Configuration**:
    - `format_on_save`: boolean
    - `format_on_type`: boolean
    - `theme`: string (name of the active theme)
    - `font_family`: string (e.g., "Fira Code", "JetBrains Mono")
    - `font_size`, `line_height`, etc.
- **Hot-reloading**: The editor watches for changes in the configuration file using `notify-rs` and applies updates instantly.

### 6. Command Palette
- **Invocation**: Triggered via `Cmd+Shift+P` (macOS) or `Ctrl+Shift+P`.
- **Fuzzy Search**: Uses `fuzzy-matcher` to provide real-time filtering as the user types.
- **Action Execution**: When an item is selected, the corresponding command is looked up in the **Command Registry** and executed.

### 7. Workspace Explorer
- **File Tree View**: A collapsible tree view is displayed in a side panel, showing the directory structure of the opened workspace.
    - **Visibility**: The explorer is only visible when a folder is explicitly opened as a workspace.
    - **Interactions**: Clicking a file in the tree opens it in the active editor buffer.
- **File Operations**: Right-clicking on files/folders provides a context menu with options like:
    - `Open`
    - `Rename`
    - `Delete`
    - `New File`
    - `New Folder`
- **File System Watching**: The explorer automatically updates when files are changed on disk, using `notify-rs` to listen for filesystem events.

### 8. Command Line Interface (CLI)
- **Usage**:
    - `lux <file_path>`: Opens the specified file in the editor.
    - `lux <directory_path>`: Opens the specified directory in the workspace explorer.
- **Implementation**: Uses `std::env::args` to parse command-line arguments and initialize the editor state with the specified file or workspace.

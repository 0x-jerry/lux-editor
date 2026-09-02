#[derive(Debug)]
pub enum CustomEvent {
    FileChange,
    ConfigChange,
    FileLoaded {
        path: std::path::PathBuf,
        buffer: Result<lux_core::Buffer, String>,
    },
    FileSaved {
        path: std::path::PathBuf,
        generation: u64,
        ok: bool,
    },
    OpenFile(std::path::PathBuf),
    OpenFolder(std::path::PathBuf),
    Delete(std::path::PathBuf),
    Rename(std::path::PathBuf, std::path::PathBuf),
    NewFile(std::path::PathBuf),
    NewFolder(std::path::PathBuf),
    ClearRecentItems,
    SwitchDocument(usize),
    CloseDocument(usize),
    SwitchToEditor,
    SwitchToConfiguration,
    ConfigurationDraftChanged,
    SetCaretFromPointer {
        line_index: usize,
        column: usize,
        selecting: bool,
    },
    SelectWordFromPointer {
        line_index: usize,
        column: usize,
    },
}

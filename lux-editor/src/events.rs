#[derive(Debug)]
pub enum CustomEvent {
    FileChange,
    ConfigChange,
    OpenFile(std::path::PathBuf),
    Delete(std::path::PathBuf),
    Rename(std::path::PathBuf, std::path::PathBuf),
    NewFile(std::path::PathBuf),
    NewFolder(std::path::PathBuf),
}

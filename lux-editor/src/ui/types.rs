//! View contracts shared across components: tab metadata for the editor tab
//! strip. The shell's frame snapshot (`ShellInput`) lives with the `Shell`
//! component in `ui/components/shell.rs`.

pub struct DocumentTab {
    pub title: String,
    pub dirty: bool,
}

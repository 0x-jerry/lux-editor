//! Declarative schema of the configuration view, modeled on JSON Schema: a
//! [`SettingSchema`] contains [`SectionSchema`]s, each section contains
//! [`RowSchema`]s, and each row's `type` (its [`RowType`]) generates the UI —
//! string → text edit, string+enum → combo, number → drag, boolean → checkbox.
//!
//! The view renders generically from [`BUILTIN`]; nothing here knows about the
//! typed `EditorSettings` except the guard test `tests::rows_resolve_against_the_settings`,
//! which walks every row and asserts its config path and default equal the
//! serde-serialized settings. Renaming/retyping a field in `types.rs` or a
//! typo'd path/default here fails the test instead of silently.
//!
//! A row's `path` is the dot-separated key into config.json (e.g. `font.size`),
//! read and written by walking the object tree. Sections are a pure UI
//! grouping — Appearance spans the `theme` and `font` groups — so a section
//! carries only presentation metadata (`title`, `description`).
//!
//! Adding a setting: a `RowSchema` in the right section (its `RowType` picks
//! the widget). Adding a section: a `SectionSchema` entry — no other code
//! changes.

/// The `type` of a setting; drives which widget renders it. Variants carry the
/// per-row default so the whole schema stays const-friendly.
#[derive(Clone, Copy)]
pub(crate) enum RowType {
    /// string → single-line text edit.
    Text {
        /// Builtin value; only the guard test reads it.
        #[cfg_attr(not(test), allow(dead_code))]
        default: &'static str,
    },
    /// string constrained to a fixed set → combo box. `(label, value)` pairs.
    TextChoice {
        options: &'static [(&'static str, &'static str)],
        /// Builtin value; only the guard test reads it.
        #[cfg_attr(not(test), allow(dead_code))]
        default: &'static str,
    },
    /// number → drag value, clamped to `min..=max`.
    Number {
        min: f64,
        max: f64,
        /// Builtin value; only the guard test reads it.
        #[cfg_attr(not(test), allow(dead_code))]
        default: f64,
    },
    /// boolean → checkbox with `label` as its text.
    Bool {
        label: &'static str,
        /// Builtin value; only the guard test reads it.
        #[cfg_attr(not(test), allow(dead_code))]
        default: bool,
    },
}

impl RowType {
    /// JSON value of this row's builtin default.
    #[cfg(test)]
    fn default_json(&self) -> serde_json::Value {
        match self {
            RowType::Text { default } => serde_json::json!(default),
            RowType::TextChoice { default, .. } => serde_json::json!(default),
            RowType::Number { default, .. } => serde_json::json!(default),
            RowType::Bool { default, .. } => serde_json::json!(default),
        }
    }

    /// Text the control itself shows (currently only the checkbox label); also
    /// searchable, since it is visible in the row.
    fn control_label(&self) -> Option<&'static str> {
        match self {
            RowType::Bool { label, .. } => Some(label),
            _ => None,
        }
    }
}

pub(crate) struct RowSchema {
    /// Dot-separated key into config.json, e.g. "font.size".
    pub(crate) path: &'static str,
    pub(crate) title: &'static str,
    /// Weak helper text under the title; empty renders nothing.
    pub(crate) description: &'static str,
    pub(crate) kind: RowType,
}

impl RowSchema {
    fn matches(&self, query: &str, section: &SectionSchema) -> bool {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return true;
        }
        [
            section.title,
            section.description,
            self.title,
            self.description,
        ]
        .into_iter()
        .chain(self.kind.control_label())
        .any(|haystack| haystack.to_lowercase().contains(&query))
    }
}

pub(crate) struct SectionSchema {
    pub(crate) title: &'static str,
    /// Weak helper text under the section title; empty renders nothing.
    pub(crate) description: &'static str,
    pub(crate) rows: &'static [RowSchema],
}

impl SectionSchema {
    /// Rows matching `query` against the section title/description plus each
    /// row's labels; an empty query returns every row.
    pub(crate) fn visible_rows(&self, query: &str) -> Vec<&'static RowSchema> {
        self.rows
            .iter()
            .filter(|row| row.matches(query, self))
            .collect()
    }
}

pub(crate) struct SettingSchema {
    pub(crate) sections: &'static [SectionSchema],
}

/// The built-in configuration: the schema every config view renders from.
pub(crate) const BUILTIN: SettingSchema = SettingSchema {
    sections: &[
        SectionSchema {
            title: "Appearance",
            description: "Colors, theme and the code font.",
            rows: &[
                RowSchema {
                    path: "theme.choice",
                    title: "Theme",
                    description: "App theme; Auto follows the operating system.",
                    kind: RowType::TextChoice {
                        options: &[("Auto", "auto"), ("Dark", "dark"), ("Light", "light")],
                        default: "auto",
                    },
                },
                RowSchema {
                    path: "font.family",
                    title: "Font family",
                    description: "Monospace font family used for code.",
                    kind: RowType::Text {
                        default: "JetBrains Mono",
                    },
                },
                RowSchema {
                    path: "font.size",
                    title: "Font size",
                    description: "Base font size in points (8–64).",
                    kind: RowType::Number {
                        min: 8.0,
                        max: 64.0,
                        default: 14.0,
                    },
                },
            ],
        },
        SectionSchema {
            title: "Formatting",
            description: "How documents are piped to an external formatter.",
            rows: &[
                RowSchema {
                    path: "formatter.command",
                    title: "Formatter command",
                    description: "The document is piped to the command on stdin and \
                                  replaced with its stdout. An empty command disables \
                                  formatting.",
                    kind: RowType::Text { default: "" },
                },
                RowSchema {
                    path: "formatter.args",
                    title: "Arguments",
                    description: "Arguments are split on whitespace.",
                    kind: RowType::Text { default: "--stdin" },
                },
                RowSchema {
                    path: "formatter.format_on_save",
                    title: "Format on save",
                    description: "",
                    kind: RowType::Bool {
                        label: "Run the formatter before writing the file.",
                        default: true,
                    },
                },
            ],
        },
    ],
};

#[cfg(test)]
mod tests {
    use super::{BUILTIN, RowSchema};
    use crate::settings::types::EditorSettings;

    fn all_rows() -> impl Iterator<Item = &'static RowSchema> {
        BUILTIN
            .sections
            .iter()
            .flat_map(|section| section.rows.iter())
    }

    #[test]
    fn section_search_filters_its_rows() {
        let [appearance, formatting] = BUILTIN.sections else {
            panic!("expected the two declared sections");
        };
        assert_eq!(appearance.rows.len(), 3);
        assert_eq!(formatting.rows.len(), 3);

        assert_eq!(appearance.visible_rows("").len(), 3);
        let family: Vec<&str> = appearance
            .visible_rows("family")
            .iter()
            .map(|row| row.path)
            .collect();
        assert_eq!(family, vec!["font.family"]);
        let size: Vec<&str> = appearance
            .visible_rows("size")
            .iter()
            .map(|row| row.path)
            .collect();
        assert_eq!(size, vec!["font.size"]);
        assert!(appearance.visible_rows("xyz").is_empty());

        // Section titles, section descriptions and control labels are searchable too.
        assert_eq!(appearance.visible_rows("APPEARANCE").len(), 3);
        assert_eq!(appearance.visible_rows("code font").len(), 3); // Appearance description
        let rows = formatting.visible_rows("writing");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].path, "formatter.format_on_save");
    }

    #[test]
    fn rows_are_unique_and_non_empty() {
        let rows: Vec<&RowSchema> = all_rows().collect();
        assert_eq!(rows.len(), 6);
        for row in &rows {
            assert_eq!(
                rows.iter().filter(|other| other.path == row.path).count(),
                1,
                "duplicated path {:?}",
                row.path
            );
            assert!(!row.title.is_empty());
        }
    }

    /// Every row's path must resolve on the serialized settings defaults and its
    /// default value must match, so a field rename/retype in `types.rs` or a
    /// typo'd path/default here fails loudly.
    #[test]
    fn rows_resolve_against_the_settings() {
        let defaults = serde_json::to_value(EditorSettings::default()).unwrap();
        for row in all_rows() {
            let mut node = &defaults;
            for segment in row.path.split('.') {
                node = node
                    .get(segment)
                    .unwrap_or_else(|| panic!("{:?} has no config key {:?}", row.title, row.path));
            }
            assert_eq!(
                node,
                &row.kind.default_json(),
                "default mismatch at {}",
                row.path
            );
        }
    }
}

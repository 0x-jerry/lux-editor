//! Language registry. `define_languages!` derives the `LanguageKind` identity
//! enum and the `LANGUAGES` table from one arm list, so the enum and the
//! registry can never drift. Adding a language touches four places:
//! Cargo.toml (the tree-sitter crate), a `mod <name>;` line, the module file
//! with its `DEF` (copy `rust.rs`), and one arm in `define_languages!`.

mod def;
mod html;
mod javascript;
mod json;
mod markdown;
mod markdown_inline;
mod python;
mod rust;
mod shell;
mod toml;
mod tsx;
mod typescript;
mod xml;
mod yaml;

pub(crate) use def::{ConfigInput, LanguageDef};

macro_rules! define_languages {
    ( $( $variant:ident => $def:path ),* $(,)? ) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum LanguageKind {
            PlainText,
            $( $variant, )*
        }

        /// Visible defs in enum order (variant N maps to index N-1).
        pub(crate) static LANGUAGES: &[&LanguageDef] = &[
            $( &$def, )*
        ];

        /// Same order as `LANGUAGES`.
        pub(crate) static KINDS: &'static [LanguageKind] = &[
            $( LanguageKind::$variant, )*
        ];

        impl LanguageKind {
            /// Variant 0 is `PlainText`; every following variant maps to its
            /// def at `self as usize - 1`, in `KINDS`/`LANGUAGES` order — keep
            /// the macro arms and variants in lockstep. `registry_aligns_with_enum`
            /// tests this invariant.
            pub(crate) const fn def_index(self) -> usize {
                match self {
                    Self::PlainText => usize::MAX,
                    _ => self as usize - 1,
                }
            }
        }
    };
}

define_languages! {
    Rust => rust::DEF,
    JavaScript => javascript::DEF,
    TypeScript => typescript::DEF,
    Tsx => tsx::DEF,
    Markdown => markdown::DEF,
    Json => json::DEF,
    Yaml => yaml::DEF,
    Toml => toml::DEF,
    Python => python::DEF,
    Shell => shell::DEF,
    Xml => xml::DEF,
    Html => html::DEF,
}

/// Injection-only grammars appended after the visible defs: no file extension
/// selects them and they have no `LanguageKind` variant.
pub(crate) static INTERNAL_LANGUAGES: &[&LanguageDef] = &[
    &markdown_inline::DEF,
];

/// Canonical names and aliases → index into `LANGUAGES` then
/// `INTERNAL_LANGUAGES`, used by injection resolution and `requires` recursion.
pub(crate) static NAME_INDEX: std::sync::LazyLock<
    std::collections::HashMap<&'static str, usize>,
> = std::sync::LazyLock::new(|| {
    let mut map = std::collections::HashMap::new();
    for (index, def) in LANGUAGES.iter().chain(INTERNAL_LANGUAGES).enumerate() {
        map.insert(def.name, index);
        for alias in def.aliases {
            map.insert(*alias, index);
        }
    }
    map
});

static EXTENSIONS: std::sync::LazyLock<
    std::collections::HashMap<&'static str, LanguageKind>,
> = std::sync::LazyLock::new(|| {
    let mut map = std::collections::HashMap::new();
    for (index, def) in LANGUAGES.iter().enumerate() {
        for ext in def.extensions {
            map.insert(*ext, KINDS[index]);
        }
    }
    map
});

impl LanguageKind {
    fn from_extension(extension: &str) -> Self {
        EXTENSIONS
            .get(extension)
            .copied()
            .unwrap_or(Self::PlainText)
    }

    pub fn from_path(path: Option<&std::path::Path>) -> Self {
        let Some(path) = path else {
            return Self::PlainText;
        };
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| Self::from_extension(&ext.to_ascii_lowercase()))
            .unwrap_or(Self::PlainText)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_aligns_with_enum() {
        assert_eq!(KINDS.len(), LANGUAGES.len());
        for (index, kind) in KINDS.iter().enumerate() {
            assert_eq!(kind.def_index(), index);
            assert!(!LANGUAGES[index].extensions.is_empty());
        }
    }

    #[test]
    fn injection_names_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for def in LANGUAGES.iter().chain(INTERNAL_LANGUAGES) {
            for name in std::iter::once(def.name).chain(def.aliases.iter().copied()) {
                assert!(
                    seen.insert(name),
                    "name `{name}` registered more than once"
                );
            }
        }
    }

    #[test]
    fn required_languages_are_registered() {
        for def in LANGUAGES.iter().chain(INTERNAL_LANGUAGES) {
            for name in def.requires {
                assert!(
                    NAME_INDEX.contains_key(name),
                    "unknown requires `{name}`"
                );
            }
        }
    }
}

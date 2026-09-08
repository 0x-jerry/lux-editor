use tree_sitter_highlight::{
    HighlightConfiguration, HighlightEvent, Highlighter,
};

use super::languages::{ConfigInput, INTERNAL_LANGUAGES, LANGUAGES, LanguageDef, NAME_INDEX};
use super::style::{RECOGNIZED_NAMES, ThemeColors};
use super::LanguageKind;

#[derive(Clone, Copy)]
pub(super) struct RawSpan {
    pub start: usize,
    pub end: usize,
    pub color: [u8; 4],
}

#[derive(Default)]
struct EngineState {
    config: Option<HighlightConfiguration>,
    /// A `configure` failure must not make every subsequent parse retry the
    /// compile (and re-log the error).
    tried: bool,
}

/// One `HighlightConfiguration` per registered language, compiled only on the
/// first request that needs it (or the first injection that names it).
pub(super) struct Engines {
    highlighter: Highlighter,
    /// Parallel to `LANGUAGES` then `INTERNAL_LANGUAGES`; visible kinds index
    /// `kind.def_index()`.
    states: Vec<EngineState>,
}

fn configure(def: &LanguageDef, input: ConfigInput) -> Option<HighlightConfiguration> {
    let mut config = HighlightConfiguration::new(
        input.language,
        def.name,
        &input.highlights_query,
        &input.injection_query,
        &input.locals_query,
    )
    .inspect_err(|error| log::error!("highlight config for {} failed: {error}", def.name))
    .ok()?;
    config.configure(RECOGNIZED_NAMES);
    Some(config)
}

/// Def at `index` in the concatenation of `LANGUAGES` + `INTERNAL_LANGUAGES`.
fn def_for(index: usize) -> &'static LanguageDef {
    LANGUAGES.get(index).copied().unwrap_or_else(|| {
        INTERNAL_LANGUAGES[index - LANGUAGES.len()]
    })
}

impl Engines {
    pub(super) fn new() -> Self {
        Self {
            highlighter: Highlighter::default(),
            states: (0..LANGUAGES.len() + INTERNAL_LANGUAGES.len())
                .map(|_| EngineState::default())
                .collect(),
        }
    }

    pub(super) fn ensure_index(&mut self, index: usize) {
        let state = &mut self.states[index];
        if state.tried {
            return;
        }
        state.tried = true;
        let def = def_for(index);
        let input = (def.config)();
        state.config = configure(def, input);
        if state.config.is_none() {
            return;
        }
        // `requires` grammars are compiled eagerly so the common case needs
        // no demand-compile passes in `spans`.
        for name in def.requires {
            if let Some(&required) = NAME_INDEX.get(name) {
                self.ensure_index(required);
            }
        }
    }

    pub(super) fn spans(
        &mut self,
        language: LanguageKind,
        document: &str,
        colors: &ThemeColors,
    ) -> Option<Vec<RawSpan>> {
        if language == LanguageKind::PlainText {
            return Some(Vec::new());
        }
        let index = language.def_index();
        self.ensure_index(index);
        // An injection may name a language that is not compiled yet; keep
        // re-highlighting until every mentioned name either compiled or
        // failed. Each pass covers the whole document, so later passes
        // supersede the earlier ones.
        let mut mentioned: Vec<usize> = Vec::new();
        loop {
            let spans = self.highlight_pass(index, document, colors, &mut mentioned);
            if mentioned.is_empty() {
                return spans;
            }
            for i in mentioned.drain(..) {
                self.ensure_index(i);
            }
        }
    }

    fn highlight_pass(
        &mut self,
        index: usize,
        document: &str,
        colors: &ThemeColors,
        mentioned: &mut Vec<usize>,
    ) -> Option<Vec<RawSpan>> {
        let Self { highlighter, states } = self;
        let config = states[index].config.as_ref()?;
        let events = highlighter
            .highlight(config, document.as_bytes(), None, |name| {
                match NAME_INDEX.get(name).copied() {
                    Some(candidate) if states[candidate].config.is_some() => {
                        states[candidate].config.as_ref()
                    }
                    Some(candidate) => {
                        if !states[candidate].tried && !mentioned.contains(&candidate) {
                            mentioned.push(candidate);
                        }
                        None
                    }
                    None => None,
                }
            })
            .ok()?;

        let mut spans: Vec<RawSpan> = Vec::new();
        let mut stack: Vec<usize> = Vec::new();
        for event in events {
            match event {
                Ok(HighlightEvent::HighlightStart(index)) => stack.push(index.0),
                Ok(HighlightEvent::HighlightEnd) => {
                    stack.pop();
                }
                Ok(HighlightEvent::Source { start, end }) => {
                    if let Some(index) = stack.last().copied() {
                        let color = colors.color(index);
                        if color != colors.foreground && end > start {
                            // Events split at every capture boundary; fuse same-color runs.
                            if let Some(last) = spans.last_mut()
                                && last.color == color
                                && last.end == start
                            {
                                last.end = end;
                                continue;
                            }
                            spans.push(RawSpan { start, end, color });
                        }
                    }
                }
                Err(error) => {
                    log::debug!("highlighting stopped: {error}");
                    break;
                }
            }
        }
        Some(spans)
    }
}

#[cfg(test)]
mod tests {
    use super::super::languages::KINDS;
    use super::*;

    #[test]
    fn every_engine_compiles() {
        let mut engines = Engines::new();
        for kind in KINDS {
            engines.ensure_index(kind.def_index());
        }
        for (index, state) in engines.states.iter().enumerate() {
            assert!(
                state.config.is_some(),
                "engine for {} (index {index}) failed to compile",
                def_for(index).name
            );
        }
    }

    #[test]
    fn markdown_inline_injection_is_configured() {
        let mut engines = Engines::new();
        engines.ensure_index(LanguageKind::Markdown.def_index());
        let inline_index = NAME_INDEX["markdown_inline"];
        assert!(
            engines.states[inline_index]
                .config
                .as_ref()
                .unwrap()
                .query
                .capture_names()
                .contains(&"injection.content"),
            "inline config must carry the inline injection patterns"
        );
    }
}

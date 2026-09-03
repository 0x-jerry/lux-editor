//! Smart bracket pairing: auto-close, skip-over, pair-aware backspace, and
//! empty-pair newline. Pure helpers over the characters immediately before
//! (`prev`) and after (`next`) a caret position.

/// What should happen when the user types `ch` at a caret.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PairingAction {
    /// Insert the opener plus its closing pair, caret between them.
    AutoClose,
    /// The character right after the caret already closes this pair: move past
    /// it instead of inserting a duplicate.
    Skip,
    /// Plain single-character insert.
    Plain,
}

/// Closing pair for an opening character, if it is one.
pub fn closing_pair(opener: char) -> Option<char> {
    match opener {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '"' | '\'' | '`' => Some(opener),
        _ => None,
    }
}

/// Opening pair for a closing character, if it is one.
pub fn opening_pair(closer: char) -> Option<char> {
    match closer {
        ')' => Some('('),
        ']' => Some('['),
        '}' => Some('{'),
        '"' | '\'' | '`' => Some(closer),
        _ => None,
    }
}

/// Decide the pairing behavior for typing `ch` between `prev` and `next`.
pub fn action_for(ch: char, prev: Option<char>, next: Option<char>) -> PairingAction {
    if closing_pair(ch).is_some() {
        // Opening character.
        let is_quote = closing_pair(ch) == Some(ch);
        if is_quote {
            if next == Some(ch) && prev.is_some() {
                // Closing an existing pair (auto-inserted or not): step over it.
                return PairingAction::Skip;
            }
            if prev.is_some_and(is_word_char) {
                // Don't open a quote right after a word character (e.g. `it'`).
                return PairingAction::Plain;
            }
        }
        return PairingAction::AutoClose;
    }

    if let Some(opener) = opening_pair(ch) {
        // Closing character.
        let next_matches = next == Some(ch);
        if opener == ch {
            // Quote-style pair: typing the same char before its copy closes it.
            if next_matches {
                return PairingAction::Skip;
            }
        } else if next_matches && prev == Some(opener) {
            // Completing an auto-closed bracket pair.
            return PairingAction::Skip;
        }
    }
    PairingAction::Plain
}

/// True when the caret sits between a matched opener/closer with nothing in
/// between (e.g. `|` in `(|)`) — used by backspace and Enter.
pub fn matched_pair_around(prev: Option<char>, next: Option<char>) -> bool {
    match (prev, next) {
        (Some(prev), Some(next)) => closing_pair(prev) == Some(next),
        _ => false,
    }
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod tests {
    use super::{PairingAction, action_for, matched_pair_around};

    #[test]
    fn brackets_auto_close() {
        for opener in ['(', '[', '{'] {
            assert_eq!(action_for(opener, None, None), PairingAction::AutoClose);
            assert_eq!(action_for(opener, Some('a'), None), PairingAction::AutoClose);
        }
    }

    #[test]
    fn quotes_auto_close_only_outside_words() {
        assert_eq!(action_for('"', None, None), PairingAction::AutoClose);
        assert_eq!(action_for('\'', Some(' '), None), PairingAction::AutoClose);
        assert_eq!(action_for('"', Some('t'), None), PairingAction::Plain);
        // Closing an auto-inserted quote skips over it.
        assert_eq!(action_for('"', Some('t'), Some('"')), PairingAction::Skip);
        assert_eq!(action_for('"', Some('"'), Some('"')), PairingAction::Skip);
    }

    #[test]
    fn closer_completes_auto_closed_pair() {
        assert_eq!(action_for(')', Some('('), Some(')')), PairingAction::Skip);
        assert_eq!(action_for(']', Some('['), Some(']')), PairingAction::Skip);
        assert_eq!(action_for('}', Some('{'), Some('}')), PairingAction::Skip);
        // Without the matching opener, the closer inserts normally.
        assert_eq!(action_for(')', Some('a'), Some(')')), PairingAction::Plain);
        assert_eq!(action_for(')', None, Some(')')), PairingAction::Plain);
    }

    #[test]
    fn ordinary_characters_are_plain() {
        assert_eq!(action_for('a', None, None), PairingAction::Plain);
        assert_eq!(action_for('+', Some(' '), None), PairingAction::Plain);
    }

    #[test]
    fn matched_pair_detection() {
        assert!(matched_pair_around(Some('('), Some(')')));
        assert!(matched_pair_around(Some('"'), Some('"')));
        assert!(!matched_pair_around(Some('('), Some(']')));
        assert!(!matched_pair_around(None, Some(')')));
        assert!(!matched_pair_around(Some('a'), Some('b')));
    }
}
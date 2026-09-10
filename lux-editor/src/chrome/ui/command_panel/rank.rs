//! Fuzzy ranking for the palette: score a query against command titles and
//! keywords, sort, and derive the emphasis ranges for matched characters.

use super::commands::PaletteItem;

type MatchSpans = Option<Vec<(usize, usize)>>;

#[derive(Clone)]
pub(super) struct RankedCommand {
    pub(super) command: PaletteItem,
    pub(super) score: i32,
    /// Char-index ranges within `title` that matched the query (emphasis).
    pub(super) match_spans: MatchSpans,
}

/// A run of commands rendered under an optional group header.
pub(super) struct Group {
    pub(super) title: Option<&'static str>,
    pub(super) commands: Vec<RankedCommand>,
}

pub(super) fn rank_commands(query: &str, commands: Vec<PaletteItem>) -> Vec<RankedCommand> {
    let mut ranked = commands
        .into_iter()
        .filter_map(|command| {
            score_command(query, &command).map(|(score, match_spans)| RankedCommand {
                command,
                score,
                match_spans,
            })
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.command.title.cmp(&right.command.title))
    });
    ranked
}

fn score_command(query: &str, command: &PaletteItem) -> Option<(i32, MatchSpans)> {
    let normalized = query.trim();
    if normalized.is_empty() {
        return Some((0, None));
    }

    let title_match = fuzzy_match(normalized, &command.title);
    let mut best = title_match.as_ref().map(|matched| matched.score);
    for keyword in &command.keywords {
        if let Some(matched) = fuzzy_match(normalized, keyword)
            && best.is_none_or(|score| matched.score > score)
        {
            best = Some(matched.score);
        }
    }
    let score = best?;

    // Emphasize only when the query is a subsequence of the title itself and
    // lowercasing did not change its length (so char indices still align).
    let match_spans = if let Some(matched) = title_match {
        if command.title.to_lowercase().chars().count() == command.title.chars().count() {
            Some(positions_to_ranges(&matched.positions))
        } else {
            None
        }
    } else {
        None
    };

    Some((score, match_spans))
}

/// A fuzzy (subsequence) match with its score and matched char positions.
struct FuzzyMatch {
    score: i32,
    positions: Vec<usize>,
}

fn fuzzy_match(query: &str, candidate: &str) -> Option<FuzzyMatch> {
    let query = query.to_lowercase();
    let candidate = candidate.to_lowercase();
    let candidate_chars = candidate.chars().collect::<Vec<_>>();

    let mut positions = Vec::with_capacity(query.len());
    let mut search_index = 0usize;
    for q in query.chars() {
        let mut found = None;
        for (idx, c) in candidate_chars.iter().enumerate().skip(search_index) {
            if *c == q {
                found = Some(idx);
                search_index = idx + 1;
                break;
            }
        }
        positions.push(found?);
    }

    let first = *positions.first()? as i32;
    let last = *positions.last()? as i32;
    let span = last - first + 1;
    let compactness_bonus = (query.chars().count() as i32) * 16 - span * 4;
    let prefix_bonus = if first == 0 { 24 } else { 0 };
    let length_penalty = candidate_chars.len() as i32;

    Some(FuzzyMatch {
        score: compactness_bonus + prefix_bonus - first - length_penalty,
        positions,
    })
}

/// Score-only wrapper kept for the tests and simpler call sites.
#[cfg(test)]
fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    fuzzy_match(query, candidate).map(|matched| matched.score)
}

/// Collapse consecutive matched positions into inclusive char ranges.
fn positions_to_ranges(positions: &[usize]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut iter = positions.iter().copied();
    if let Some(first) = iter.next() {
        let mut start = first;
        let mut end = first + 1;
        for position in iter {
            if position == end {
                end = position + 1;
            } else {
                ranges.push((start, end));
                start = position;
                end = position + 1;
            }
        }
        ranges.push((start, end));
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::{fuzzy_match, fuzzy_score, positions_to_ranges, rank_commands, score_command};
    use crate::chrome::ui::command_panel::commands::{CommandIcon, PaletteItem, PaletteTarget};
    use std::path::PathBuf;

    fn item(title: &str, keywords: &[&str]) -> PaletteItem {
        PaletteItem {
            title: title.to_string(),
            subtitle: None,
            keywords: keywords.iter().map(|k| k.to_string()).collect(),
            icon: CommandIcon::Phosphor("x"),
            target: PaletteTarget::OpenRecent {
                path: PathBuf::new(),
                is_dir: false,
            },
        }
    }

    #[test]
    fn fuzzy_score_prefers_compact_matches() {
        let compact = fuzzy_score("opf", "Open File").unwrap();
        let sparse = fuzzy_score("opf", "Open Folder").unwrap();
        assert!(compact > sparse);
    }

    #[test]
    fn fuzzy_score_requires_subsequence() {
        assert!(fuzzy_score("xyz", "Open File").is_none());
    }

    #[test]
    fn rank_commands_can_match_recent_list_entry() {
        let ranked = rank_commands(
            "rustfmt",
            vec![PaletteItem {
                title: "rustfmt.toml".to_string(),
                subtitle: Some("/tmp/rustfmt.toml".to_string()),
                keywords: vec!["recent".to_string(), "rustfmt.toml".to_string()],
                icon: CommandIcon::File(PathBuf::from("/tmp/rustfmt.toml")),
                target: PaletteTarget::OpenRecent {
                    path: PathBuf::from("/tmp/rustfmt.toml"),
                    is_dir: false,
                },
            }],
        );
        assert_eq!(ranked[0].command.title, "rustfmt.toml");
    }

    #[test]
    fn score_command_produces_title_match_spans() {
        let command = item("Save File", &[]);
        // "sf" matches "Save File" at chars 0 and 5 -> two single-char ranges.
        let (score, spans) = score_command("sf", &command).unwrap();
        assert!(score > 0);
        assert_eq!(spans.unwrap(), vec![(0, 1), (5, 6)]);
    }

    #[test]
    fn score_command_skips_emphasis_for_keyword_only_match() {
        let command = item("Save File", &["persist"]);
        // "persist" matches the keyword but not the title -> no emphasis.
        let (_score, spans) = score_command("persist", &command).unwrap();
        assert!(spans.is_none());
    }

    #[test]
    fn empty_query_has_no_emphasis() {
        let command = item("Save File", &[]);
        let (_score, spans) = score_command("", &command).unwrap();
        assert!(spans.is_none());
    }

    #[test]
    fn positions_to_ranges_merges_runs() {
        assert_eq!(positions_to_ranges(&[0, 1, 2, 5]), vec![(0, 3), (5, 6)]);
        assert_eq!(positions_to_ranges(&[3]), vec![(3, 4)]);
        assert_eq!(positions_to_ranges(&[]), vec![]);
    }

    #[test]
    fn fuzzy_match_returns_positions() {
        // "opf" matches "open file" at chars 0, 1 and 5.
        let matched = fuzzy_match("opf", "Open File").unwrap();
        assert_eq!(matched.positions, vec![0, 1, 5]);
    }
}

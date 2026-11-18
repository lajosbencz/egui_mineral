use std::cell::RefCell;

use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{AtomKind, CaseMatching, Normalization, Pattern},
};

/// Fuzzy match: score and matched character indices.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Match {
    /// nucleo score; higher is better.
    pub score: u32,
    /// Char indices of matched characters in the haystack, ascending.
    pub indices: Vec<usize>,
}

// nucleo's Matcher carries ~135 KB of scratch space; reused with the haystack and index buffers.
struct Scratch {
    matcher: Matcher,
    chars: Vec<char>,
    indices: Vec<u32>,
}

thread_local! {
    static SCRATCH: RefCell<Scratch> = RefCell::new(Scratch {
        matcher: Matcher::new(Config::DEFAULT),
        chars: Vec::new(),
        indices: Vec::new(),
    });
}

impl Scratch {
    fn run(&mut self, p: &Pattern, candidate: &str) -> Option<Match> {
        let haystack = if candidate.is_ascii() {
            Utf32Str::Ascii(candidate.as_bytes())
        } else {
            self.chars.clear();
            self.chars.extend(candidate.chars());
            Utf32Str::Unicode(&self.chars)
        };
        self.indices.clear();
        let score = p.indices(haystack, &mut self.matcher, &mut self.indices)?;
        self.indices.sort_unstable();
        self.indices.dedup();
        Some(Match {
            score,
            indices: self.indices.iter().map(|&i| i as usize).collect(),
        })
    }
}

/// Parsed fuzzy query (nucleo), reusable across candidates.
#[derive(Clone, Debug)]
pub struct Query(Pattern);

impl Query {
    /// Smart-case, diacritic-folding fuzzy query.
    pub fn new(text: &str) -> Self {
        Self(Pattern::new(
            text,
            CaseMatching::Smart,
            Normalization::Smart,
            AtomKind::Fuzzy,
        ))
    }

    /// Match of the query against `candidate`; `None` on no match.
    pub fn matches(&self, candidate: &str) -> Option<Match> {
        SCRATCH.with_borrow_mut(|s| s.run(&self.0, candidate))
    }

    /// Matching items with their [`Match`], best score first.
    pub fn rank<'a, T>(&self, items: impl IntoIterator<Item = (T, &'a str)>) -> Vec<(T, Match)> {
        let mut out: Vec<_> = SCRATCH.with_borrow_mut(|scratch| {
            items
                .into_iter()
                .filter_map(|(t, s)| scratch.run(&self.0, s).map(|r| (t, r)))
                .collect()
        });
        out.sort_by_key(|(_, m)| std::cmp::Reverse(m.score));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_prefers_boundaries() {
        let m = Query::new("gs").matches("Git Status").unwrap();
        assert_eq!(m.indices, [0, 4]);
        assert!(
            Query::new("resume").matches("R\u{e9}sum\u{e9}").is_some(),
            "diacritics fold"
        );
        assert!(Query::new("xyz").matches("Git Status").is_none());
        let ranked = Query::new("op").rank([(0, "Stop"), (1, "Open Project")]);
        assert_eq!(ranked[0].0, 1);
    }

    #[test]
    fn indices_count_chars_not_bytes_or_graphemes() {
        // 'A' + combining mark form one grapheme; every grapheme starts with an ASCII char.
        let label = "A\u{11ca9}'";
        let m = Query::new("'").matches(label).unwrap();
        assert_eq!(m.indices, [2]);
        assert_eq!(label.chars().nth(2), Some('\''));
    }
}

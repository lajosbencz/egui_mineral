/// Result of [`split`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Split {
    /// Trimmed tokens before the last delimiter or newline; empty ones dropped.
    pub tags: Vec<String>,
    /// Text after the last delimiter, still being typed.
    pub rest: String,
}

/// Splits typed `input` into completed tags and the text still being typed.
pub fn split(input: &str, delimiters: &[char]) -> Split {
    let is_delim = |c: char| c == '\n' || c == '\r' || delimiters.contains(&c);
    let Some(last) = input.rfind(is_delim) else {
        return Split {
            tags: Vec::new(),
            rest: input.to_owned(),
        };
    };
    let (done, rest) = input.split_at(last);
    let mut chars = rest.chars();
    chars.next();
    let rest = chars.as_str();
    let tags = done
        .split(is_delim)
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect();
    Split {
        tags,
        rest: rest.to_owned(),
    }
}

/// Appends new tags case-insensitively unique up to `max`; returns how many were added.
pub fn add(tags: &mut Vec<String>, new: impl IntoIterator<Item = String>, max: Option<usize>) -> usize {
    let mut added = 0;
    for t in new {
        if max.is_some_and(|m| tags.len() >= m) {
            break;
        }
        let lower = t.to_lowercase();
        if !t.is_empty() && !tags.iter().any(|x| x.to_lowercase() == lower) {
            tags.push(t);
            added += 1;
        }
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_and_add() {
        assert_eq!(
            split("a, b ,,c", &[',']),
            Split {
                tags: vec!["a".to_owned(), "b".to_owned()],
                rest: "c".to_owned()
            }
        );
        assert_eq!(split(",", &[',']), Split::default());
        let mut tags = vec!["A".to_owned()];
        assert_eq!(add(&mut tags, ["a".into(), "b".into(), "c".into()], Some(2)), 1);
        assert_eq!(tags, ["A", "b"]);
    }
}

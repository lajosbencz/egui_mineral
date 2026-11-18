#![warn(clippy::arithmetic_side_effects)]

/// Characters accepted by a [`CellMask`] cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Charset {
    /// ASCII digits only.
    #[default]
    Digits,
    /// Unicode letters and digits.
    Alphanumeric,
    /// Any visible char; no controls or whitespace.
    Any,
}

impl Charset {
    /// `true` when a cell may hold `c`.
    pub fn accepts(self, c: char) -> bool {
        match self {
            Self::Digits => c.is_ascii_digit(),
            Self::Alphanumeric => c.is_alphanumeric(),
            Self::Any => !c.is_control() && !c.is_whitespace(),
        }
    }
}

/// Fixed-length code of single-char cells (OTP, PIN).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellMask {
    /// Chars a cell accepts.
    pub charset: Charset,
    /// Cell count.
    pub len: usize,
}

impl CellMask {
    /// Accepted chars of `value`, truncated to `len`.
    pub fn sanitize(&self, value: &str) -> String {
        value
            .chars()
            .filter(|c| self.charset.accepts(*c))
            .take(self.len)
            .collect()
    }

    /// Writes the accepted chars of `text` over a gap-free `value` from cell `at` (a full-length
    /// paste replaces everything). Returns the cell to focus next.
    pub fn fill(&self, value: &mut String, at: usize, text: &str) -> usize {
        let len = self.len;
        let incoming: Vec<char> = text.chars().filter(|c| self.charset.accepts(*c)).collect();
        let mut cells: Vec<char> = value.chars().take(len).collect();
        let mut at = at.min(cells.len());
        if incoming.len() >= len {
            cells.clear();
            at = 0;
        }
        let written = len.saturating_sub(at).min(incoming.len());
        for (i, c) in (at..len).zip(incoming) {
            if i < cells.len() {
                cells[i] = c;
            } else {
                cells.push(c);
            }
        }
        *value = cells.into_iter().collect();
        at.saturating_add(written).min(len.saturating_sub(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_fill_and_paste() {
        let m = CellMask {
            charset: Charset::Digits,
            len: 4,
        };
        let mut v = String::from("12");
        assert_eq!(m.fill(&mut v, 2, "3"), 3);
        assert_eq!(v, "123");
        assert_eq!(m.fill(&mut v, 0, "x9"), 1);
        assert_eq!(v, "923");
        assert_eq!(m.fill(&mut v, 2, "4567"), 3, "full paste restarts");
        assert_eq!(v, "4567");
        let a = CellMask {
            charset: Charset::Alphanumeric,
            len: 3,
        };
        assert_eq!(a.sanitize("a1b2"), "a1b");
    }
}

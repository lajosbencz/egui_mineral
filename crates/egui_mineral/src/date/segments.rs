#![warn(clippy::arithmetic_side_effects)]

use jiff::civil::Date;

use crate::i18n::DatePart;

/// Segmented date entry: locale-ordered year, month and day fields with typed digits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateSegments {
    order: [DatePart; 3],
    values: [Option<u16>; 3],
    typing: Option<(usize, u8)>,
}

#[expect(
    clippy::indexing_slicing,
    reason = "`seg` and `position` yield 0..=2 into the `[_; 3]` arrays"
)]
impl DateSegments {
    /// Empty segments in display `order` (from [`LocaleFormat::date_order`](crate::i18n::LocaleFormat::date_order)).
    pub fn new(order: [DatePart; 3]) -> Self {
        Self {
            order,
            values: [None; 3],
            typing: None,
        }
    }

    /// Display order of the three segments.
    pub fn order(&self) -> [DatePart; 3] {
        self.order
    }

    /// Segments filled from `date`, empty for `None`; a year before 1 shows as year 1.
    pub fn from_date(order: [DatePart; 3], date: Option<Date>) -> Self {
        let mut s = Self::new(order);
        if let Some(d) = date {
            let year = if d.year() >= 1 {
                d.year().unsigned_abs()
            } else {
                crate::kit::sanitize::report(format_args!("DateSegments year {} is before 1", d.year()));
                1
            };
            for (value, f) in s.values.iter_mut().zip(order) {
                *value = Some(match f {
                    DatePart::Year => year,
                    DatePart::Month => u16::from(d.month().unsigned_abs()),
                    DatePart::Day => u16::from(d.day().unsigned_abs()),
                });
            }
        }
        s
    }

    fn seg(seg: usize) -> usize {
        crate::kit::sanitize::index(seg, 3, "DateSegments segment").unwrap_or_default()
    }

    /// Digits a segment takes before auto-advancing: 4 for years, 2 otherwise.
    pub fn max_digits(field: DatePart) -> u8 {
        if field == DatePart::Year { 4 } else { 2 }
    }

    fn bounds(&self, field: DatePart) -> (u16, u16) {
        match field {
            DatePart::Year => (1, 9999),
            DatePart::Month => (1, 12),
            DatePart::Day => {
                let y = self
                    .get(DatePart::Year)
                    .and_then(|y| i16::try_from(y).ok())
                    .filter(|y| (1..=9999).contains(y));
                let days_in_month = self
                    .get(DatePart::Month)
                    .and_then(|m| i8::try_from(m).ok())
                    .and_then(|m| Date::new(y.unwrap_or(2000), m, 1).ok())
                    .map(|d| u16::from(d.days_in_month().unsigned_abs()));
                (1, days_in_month.unwrap_or(31))
            }
        }
    }

    /// Value of the segment holding `field`; `None` while empty.
    pub fn get(&self, field: DatePart) -> Option<u16> {
        self.order.iter().position(|f| *f == field).and_then(|i| self.values[i])
    }

    /// Value by display index 0..3; `None` while empty or out of range.
    pub fn value(&self, seg: usize) -> Option<u16> {
        self.values.get(seg).copied().flatten()
    }

    /// Ends digit entry, so the next digit restarts the segment.
    pub fn commit(&mut self) {
        self.typing = None;
    }

    /// Types digit `d` into `seg`; returns the segment that gets focus next. `d` above 9 counts as 9
    /// and `seg` above 2 as 2.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "a segment holds < max_digits typed digits (<= 999) before the next one, so v <= 9999"
    )]
    pub fn digit(&mut self, seg: usize, d: u8) -> usize {
        let d = crate::kit::sanitize::at_most(d, ..=9, "DateSegments digit");
        let seg = Self::seg(seg);
        let field = self.order[seg];
        let typed = match self.typing {
            Some((s, n)) if s == seg => n,
            _ => 0,
        };
        let v = if typed == 0 {
            u16::from(d)
        } else {
            self.values[seg].unwrap_or(0) * 10 + u16::from(d)
        };
        self.values[seg] = Some(v);
        let typed = typed + 1;
        let (lo, hi) = self.bounds(field);
        if typed >= Self::max_digits(field) || (field != DatePart::Year && v * 10 > hi) {
            self.values[seg] = Some(v.clamp(lo, hi));
            self.typing = None;
            (seg + 1).min(2)
        } else {
            self.typing = Some((seg, typed));
            seg
        }
    }

    /// Backspace: clears `seg`, or when it is already empty reports the previous segment; `seg` above 2 counts as 2.
    pub fn clear(&mut self, seg: usize) -> usize {
        let seg = Self::seg(seg);
        self.typing = None;
        if self.values[seg].take().is_some() {
            seg
        } else {
            seg.saturating_sub(1)
        }
    }

    /// Up/Down: steps `seg` by `delta`, wrapping month and day; an empty segment starts at `today`.
    /// `seg` above 2 counts as 2.
    pub fn bump(&mut self, seg: usize, delta: i32, today: Date) {
        let seg = Self::seg(seg);
        self.typing = None;
        let field = self.order[seg];
        let (lo, hi) = self.bounds(field);
        let (lo32, hi32) = (i32::from(lo), i32::from(hi));
        let v = match (self.values[seg], field) {
            (Some(v), DatePart::Year) => i32::from(v).saturating_add(delta),
            (Some(v), _) => {
                let span = hi32.saturating_sub(lo32).saturating_add(1).max(1);
                i32::from(v)
                    .saturating_sub(lo32)
                    .saturating_add(delta)
                    .rem_euclid(span)
                    .saturating_add(lo32)
            }
            (None, DatePart::Year) => i32::from(today.year()),
            (None, DatePart::Month) => i32::from(today.month()),
            (None, DatePart::Day) => i32::from(today.day()),
        };
        self.values[seg] = Some(u16::try_from(v.clamp(lo32, hi32)).unwrap_or(hi));
    }

    /// `Some` when all three segments form a valid date.
    pub fn date(&self) -> Option<Date> {
        let y = self.get(DatePart::Year)?;
        let m = self.get(DatePart::Month)?;
        let d = self.get(DatePart::Day)?;
        (y >= 1).then_some(())?;
        Date::new(i16::try_from(y).ok()?, i8::try_from(m).ok()?, i8::try_from(d).ok()?).ok()
    }

    /// All segments empty.
    pub fn is_empty(&self) -> bool {
        self.values.iter().all(Option::is_none)
    }

    /// Zero-padded ASCII text of `seg` (years pad to 4 only once complete); `None` while empty or out of range.
    pub fn text(&self, seg: usize) -> Option<String> {
        let v = self.value(seg)?;
        let field = *self.order.get(seg)?;
        let width = if field == DatePart::Year && self.typing.is_some_and(|(s, _)| s == seg) {
            0
        } else {
            usize::from(Self::max_digits(field))
        };
        Some(format!("{v:0width$}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_segments() {
        let order = [DatePart::Month, DatePart::Day, DatePart::Year];
        let mut s = DateSegments::new(order);
        assert_eq!(s.digit(0, 4), 1, "month 4 cannot take a second digit");
        assert_eq!(s.digit(1, 3), 1);
        assert_eq!(s.digit(1, 1), 2);
        for d in [2, 0, 2, 4] {
            s.digit(2, d);
        }
        assert_eq!(s.date(), Date::new(2024, 4, 30).ok(), "day clamped to April");
        s.bump(1, 1, Date::constant(2000, 1, 1));
        assert_eq!(s.value(1), Some(1), "day wraps");
        assert_eq!(s.clear(0), 0);
        assert_eq!(s.clear(0), 0);
        assert_eq!(s.date(), None);
    }

    #[test]
    fn dates_before_year_one_show_as_year_one() {
        let order = [DatePart::Year, DatePart::Month, DatePart::Day];
        for year in [0, -44] {
            let s = DateSegments::from_date(order, Date::new(year, 3, 9).ok());
            assert_eq!((s.value(0), s.value(1), s.value(2)), (Some(1), Some(3), Some(9)));
        }
        let s = DateSegments::from_date(order, Date::new(2024, 12, 31).ok());
        assert_eq!(s.date(), Date::new(2024, 12, 31).ok());
    }

    #[test]
    fn out_of_range_arguments_are_clamped() {
        let order = [DatePart::Month, DatePart::Day, DatePart::Year];
        let mut s = DateSegments::new(order);
        assert_eq!(s.digit(7, 12), 2, "segment 7 is the last one, digit 12 is 9");
        assert_eq!(s.value(2), Some(9));
        assert_eq!(s.value(7), None);
        assert_eq!(s.text(7), None);
        assert_eq!(s.clear(7), 2);
        s.bump(7, 1, Date::constant(2000, 1, 1));
        assert_eq!(s.value(2), Some(2000));
    }

    #[test]
    fn typing_restarts_in_another_segment() {
        let order = [DatePart::Year, DatePart::Month, DatePart::Day];
        let mut s = DateSegments::from_date(order, Date::new(9999, 1, 1).ok());
        s.digit(1, 1);
        s.digit(0, 9);
        assert_eq!(s.value(0), Some(9));
    }
}

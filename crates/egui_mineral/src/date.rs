// docs/rules.md rule 10: indices here outlive the frame.
#![deny(clippy::indexing_slicing)]
#![warn(clippy::arithmetic_side_effects)]
use egui::{Key, Modifiers};
use jiff::ToSpan;
pub use jiff::civil::{Date, Time, Weekday};

pub use self::segments::DateSegments;
use crate::i18n::{DayPeriod, Direction};

mod segments;

/// Six-week (42 day) grid covering a month.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct MonthGrid {
    /// First day of the month.
    pub first: Date,
    /// Grid days in row-major order, starting on the first weekday.
    pub days: [Date; 42],
}

impl MonthGrid {
    /// Six weeks around the month of `date`, starting on `first_weekday`; saturates at `Date::MIN`.
    pub fn new(date: Date, first_weekday: Weekday) -> Self {
        let first = date.first_of_month();
        let offset = first_weekday.until(first.weekday());
        let start = first.saturating_sub(i64::from(offset).days());
        let mut days = [start; 42];
        for (d, i) in days.iter_mut().zip(0_i64..) {
            *d = start.saturating_add(i.days());
        }
        Self { first, days }
    }

    /// `d` falls in the grid's month, not the leading or trailing weeks.
    pub fn in_month(&self, d: Date) -> bool {
        d.year() == self.first.year() && d.month() == self.first.month()
    }

    /// Six rows of seven days.
    pub fn weeks(&self) -> impl Iterator<Item = &[Date]> {
        self.days.chunks(7)
    }
}

/// Seven consecutive weekdays from `first`.
pub fn weekdays_from(first: Weekday) -> [Weekday; 7] {
    let mut day = first;
    std::array::from_fn(|_| {
        let current = day;
        day = day.next();
        current
    })
}

const MAX_MONTHS: i32 = 19_998 * 12;

/// Saturating month arithmetic; keeps the day where the month has it.
pub fn add_months(d: Date, n: i32) -> Date {
    d.saturating_add(n.clamp(-MAX_MONTHS, MAX_MONTHS).months())
}

/// Inclusive; `min <= max` when both are set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DateBounds {
    min: Option<Date>,
    max: Option<Date>,
}

impl DateBounds {
    /// Either bound may be open; `max` before `min` is raised to `min`.
    #[track_caller]
    pub fn new(min: Option<Date>, max: Option<Date>) -> Self {
        match (min, max) {
            (Some(lo), Some(hi)) if lo > hi => {
                crate::kit::sanitize::report(format_args!("DateBounds: max {hi} is before min {lo}; raised to min"));
                Self { min, max: min }
            }
            _ => Self { min, max },
        }
    }

    /// Earliest allowed date, inclusive.
    pub fn min(&self) -> Option<Date> {
        self.min
    }

    /// Latest allowed date, inclusive.
    pub fn max(&self) -> Option<Date> {
        self.max
    }

    /// Inclusive of both bounds.
    pub fn contains(&self, d: Date) -> bool {
        self.min.is_none_or(|m| d >= m) && self.max.is_none_or(|m| d <= m)
    }

    /// Nearest allowed date.
    pub fn clamp(&self, d: Date) -> Date {
        let d = self.min.map_or(d, |m| d.max(m));
        self.max.map_or(d, |m| d.min(m))
    }
}

/// <https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/examples/datepicker-dialog/>; horizontal arrows flip in RTL.
pub fn on_key(d: Date, key: Key, modifiers: Modifiers, dir: Direction, first_weekday: Weekday) -> Option<Date> {
    let step = |n: i64| d.saturating_add(n.days());
    Some(match key {
        Key::ArrowLeft | Key::ArrowRight => step(if key == dir.forward_key() { 1 } else { -1 }),
        Key::ArrowUp => step(-7),
        Key::ArrowDown => step(7),
        Key::PageUp => add_months(d, if modifiers.shift { -12 } else { -1 }),
        Key::PageDown => add_months(d, if modifiers.shift { 12 } else { 1 }),
        Key::Home | Key::End => {
            let into_week = i64::from(first_weekday.until(d.weekday()));
            if key == Key::Home {
                step(into_week.saturating_neg())
            } else {
                step(6_i64.saturating_sub(into_week))
            }
        }
        _ => return None,
    })
}

/// Complete inclusive range; `start` is at or before `end`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DateSpan {
    /// First day.
    pub start: Date,
    /// Last day.
    pub end: Date,
}

/// Inclusive; an end implies a start at or before it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DateRange {
    start: Option<Date>,
    end: Option<Date>,
}

impl DateRange {
    /// Complete range between two picked ends, in either order.
    pub fn new(start: Date, end: Date) -> Self {
        let (start, end) = (start.min(end), start.max(end));
        Self {
            start: Some(start),
            end: Some(end),
        }
    }

    /// Range awaiting its end, as after the first pick.
    pub fn starting(start: Date) -> Self {
        Self {
            start: Some(start),
            end: None,
        }
    }

    /// `None` only for an empty range.
    pub fn start(&self) -> Option<Date> {
        self.start
    }

    /// `None` while the range awaits its end.
    pub fn end(&self) -> Option<Date> {
        self.end
    }

    /// Both ends set.
    pub fn is_complete(&self) -> bool {
        self.start.is_some() && self.end.is_some()
    }

    /// Both ends once complete.
    pub fn span(&self) -> Option<DateSpan> {
        self.start.zip(self.end).map(|(start, end)| DateSpan { start, end })
    }

    /// `false` until complete; inclusive of both ends.
    pub fn contains(&self, d: Date) -> bool {
        self.span().is_some_and(|s| d >= s.start && d <= s.end)
    }
}

/// 24-hour `hour` to 1..=12 and its period.
pub fn to_12h(hour: i8) -> (i8, DayPeriod) {
    let h = hour.rem_euclid(24);
    let period = if h >= 12 { DayPeriod::Pm } else { DayPeriod::Am };
    (if h % 12 == 0 { 12 } else { h % 12 }, period)
}

/// 1..=12 hour and period to 24-hour.
pub fn from_12h(hour12: i8, period: DayPeriod) -> i8 {
    hour12.rem_euclid(12).saturating_add(match period {
        DayPeriod::Am => 0,
        DayPeriod::Pm => 12,
    })
}

/// Today in the system time zone.
pub fn today() -> Date {
    jiff::ZonedDateTime::now().date()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_grid_and_navigation() {
        let g = MonthGrid::new(Date::new(2026, 11, 18).unwrap(), Weekday::Monday);
        // 2026-11-01 is a Sunday: grid starts on Monday 2026-10-26.
        assert_eq!(g.days[0], Date::new(2026, 10, 26).unwrap());
        assert_eq!(g.days[6], Date::new(2026, 11, 1).unwrap());
        let jan31 = Date::new(2026, 1, 31).unwrap();
        assert_eq!(add_months(jan31, 1), Date::new(2026, 2, 28).unwrap());
        let d = Date::new(2026, 11, 18).unwrap(); // Wednesday
        assert_eq!(
            on_key(d, Key::Home, Modifiers::NONE, Direction::Ltr, Weekday::Monday),
            Date::new(2026, 11, 16).ok()
        );
        assert_eq!(
            on_key(d, Key::ArrowLeft, Modifiers::NONE, Direction::Rtl, Weekday::Monday),
            Date::new(2026, 11, 19).ok()
        );
    }

    #[test]
    fn twelve_hour_conversion() {
        assert_eq!(to_12h(0), (12, DayPeriod::Am));
        assert_eq!(to_12h(13), (1, DayPeriod::Pm));
        assert_eq!(from_12h(12, DayPeriod::Am), 0);
    }

    #[test]
    fn bounds_and_range_are_inclusive() {
        let d = |day| Date::new(2026, 11, day).unwrap();
        let b = DateBounds::new(Some(d(10)), Some(d(20)));
        assert!(b.contains(d(10)) && b.contains(d(20)));
        assert!(!b.contains(d(9)) && !b.contains(d(21)));
        assert_eq!((b.clamp(d(1)), b.clamp(d(15)), b.clamp(d(30))), (d(10), d(15), d(20)));

        let r = DateRange::new(d(10), d(20));
        assert!(r.contains(d(10)) && r.contains(d(20)) && !r.contains(d(21)));
        assert!(!DateRange::starting(d(10)).contains(d(10)));
    }

    #[test]
    fn shift_page_down_moves_a_year_clamping_the_day() {
        let leap_day = Date::new(2024, 2, 29).unwrap();
        let shifted = on_key(
            leap_day,
            Key::PageDown,
            Modifiers::SHIFT,
            Direction::Ltr,
            Weekday::Monday,
        );
        assert_eq!(shifted, Date::new(2025, 2, 28).ok());
        let plain = on_key(
            leap_day,
            Key::PageDown,
            Modifiers::NONE,
            Direction::Ltr,
            Weekday::Monday,
        );
        assert_eq!(plain, Date::new(2024, 3, 29).ok());
    }

    #[test]
    fn crossed_bounds_collapse_and_ranges_order() {
        let (a, b) = (Date::constant(2024, 1, 1), Date::constant(2024, 2, 1));
        assert_eq!(DateBounds::new(Some(b), Some(a)), DateBounds::new(Some(b), Some(b)));
        assert_eq!(DateRange::new(b, a), DateRange::new(a, b));
    }

    #[test]
    fn month_arithmetic_saturates_for_any_count() {
        let d = Date::constant(2026, 11, 18);
        assert_eq!(add_months(d, i32::MAX), Date::MAX);
        assert_eq!(add_months(d, i32::MIN), Date::MIN);
    }
}

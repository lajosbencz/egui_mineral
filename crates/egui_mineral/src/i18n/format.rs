use jiff::civil::{Date, Time, Weekday};

use crate::i18n::Direction;

/// 12 or 24-hour clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HourCycle {
    /// Hours 1..=12 with a day period (CLDR `h`).
    H12,
    /// Hours 0..=23 (CLDR `H`).
    H23,
}

/// Width of month and weekday names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NameWidth {
    /// Shortest form, often one letter.
    Narrow,
    /// Short form, e.g. `Mon`.
    Abbreviated,
    /// Full name, e.g. `Monday`.
    Wide,
}

/// AM or PM.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DayPeriod {
    /// Before noon.
    Am,
    /// Noon and after.
    Pm,
}

/// Where a 12-hour clock writes the day period.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PeriodPosition {
    /// e.g. `PM 1:45`.
    BeforeTime,
    /// e.g. `1:45 PM`.
    AfterTime,
}

/// Date field skeleton (CLDR/ICU4X field sets).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DateFields {
    /// CLDR `YMD`.
    YearMonthDay,
    /// CLDR `YMDE`.
    YearMonthDayWeekday,
    /// CLDR `MD`; no year.
    MonthDay,
    /// CLDR `MDE`; no year.
    MonthDayWeekday,
    /// CLDR `DE`; day of month and weekday only.
    DayWeekday,
}

/// CLDR date length.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DateLength {
    /// CLDR short; spells the full year so it parses back with [`LocaleFormat::parse_date`].
    Numeric,
    /// CLDR medium.
    Medium,
    /// CLDR long.
    Long,
}

/// Conjunction of a formatted list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListKind {
    /// Conjunction list, e.g. `a, b, and c`.
    And,
    /// Disjunction list, e.g. `a, b, or c`.
    Or,
    /// Unit list without a conjunction, e.g. `3 ft, 7 in`.
    Unit,
}

/// Component of a calendar date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DatePart {
    /// Full year; no two-digit expansion when parsing.
    Year,
    /// 1..=12.
    Month,
    /// Day of month, 1..=31.
    Day,
}

pub(super) fn fluent_fraction_digits(n: &fluent_bundle::types::FluentNumber) -> Option<i16> {
    n.options
        .maximum_fraction_digits
        .or(n.options.minimum_fraction_digits)
        .and_then(|d| i16::try_from(d).ok())
}

pub(super) const ASCII_DIGITS: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

/// Locale-aware formatting and calendar data; `IcuFormat` (feature `icu`) and [`AsciiFormat`](super::AsciiFormat) implement it.
pub trait LocaleFormat: Send + Sync {
    /// BCP 47 locale tag, e.g. `en-US`.
    fn tag(&self) -> &str;
    /// Writing direction of the locale's script.
    fn direction(&self) -> Direction;
    /// First column of calendar grids.
    fn first_weekday(&self) -> Weekday;
    /// Locale's preferred clock; [`Self::time`] takes it explicitly.
    fn hour_cycle(&self) -> HourCycle;
    /// `month` in 1..=12; others clamp into it.
    fn month_name(&self, month: i8, width: NameWidth) -> String;
    /// Localized weekday name at `width`.
    fn weekday_name(&self, day: Weekday, width: NameWidth) -> String;
    /// Localized AM/PM marker.
    fn day_period(&self, period: DayPeriod) -> String;
    /// Localized date for the CLDR field set `fields` at `length`.
    fn date(&self, d: Date, fields: DateFields, length: DateLength) -> String;
    /// `month` in 1..=12; others clamp into it.
    fn month_year(&self, year: i16, month: i8) -> String;
    /// Hours and minutes on the `cycle` clock, no seconds.
    fn time(&self, t: Time, cycle: HourCycle) -> String;
    /// Grouped decimal; `fraction_digits: None` keeps the shortest exact form.
    fn number(&self, value: f64, fraction_digits: Option<i16>) -> String;
    /// Locale digits for 0..=9, in order.
    fn digits(&self) -> [char; 10];
    /// Decimal separator of [`Self::number`] output.
    fn decimal_separator(&self) -> char;
    /// Digit grouping separator of [`Self::number`] output; `None` where it does not group.
    fn grouping_separator(&self) -> Option<char>;
    /// Field order of [`DateLength::Numeric`] dates, e.g. `[Month, Day, Year]` in en-US.
    fn date_order(&self) -> [DatePart; 3];
    /// Locale collation order; must be a total order (std sorts may panic otherwise).
    fn compare(&self, a: &str, b: &str) -> std::cmp::Ordering;
    /// Joins `items` with locale separators and the `kind` conjunction.
    fn list(&self, items: &[&str], kind: ListKind) -> String;

    /// [`Self::number`] for app-fed values: non-finite ones print unformatted with a warning.
    fn number_lossy(&self, value: f64, fraction_digits: Option<i16>) -> String {
        if value.is_finite() {
            self.number(value, fraction_digits)
        } else {
            tracing::warn!(value, "non-finite number rendered unformatted");
            value.to_string()
        }
    }

    /// [`Self::number`] without fraction digits; magnitudes beyond 2^53 round.
    fn integer(&self, value: i64) -> String {
        #[expect(clippy::cast_precision_loss, reason = "i64 magnitudes beyond 2^53 round")]
        self.number(value as f64, Some(0))
    }

    /// [`Self::integer`] for a length or index; saturates at `i64::MAX`.
    fn count(&self, n: usize) -> String {
        self.integer(i64::try_from(n).unwrap_or(i64::MAX))
    }

    /// Maps ASCII digits in `s` to [`Self::digits`]; other chars pass through.
    fn native_digits(&self, s: &str) -> String {
        let d = self.digits();
        s.chars().map(|c| c.to_digit(10).map_or(c, |i| d[i as usize])).collect()
    }

    /// Inverse of [`Self::native_digits`].
    fn ascii_digits(&self, s: &str) -> String {
        let d = self.digits();
        s.chars()
            .map(|c| d.iter().position(|x| *x == c).map_or(c, |i| ASCII_DIGITS[i]))
            .collect()
    }

    /// Separator between hours and minutes, probed from [`Self::time`]; `:` when none is found.
    fn time_separator(&self) -> String {
        let s = self.ascii_digits(&self.time(Time::constant(13, 45, 0, 0), HourCycle::H23));
        s.find("13")
            .zip(s.find("45"))
            .and_then(|(a, b)| s.get(a + 2..b))
            .map_or_else(|| ":".to_owned(), str::to_owned)
    }

    /// `BeforeTime` in e.g. ko, zh.
    fn period_position(&self) -> PeriodPosition {
        let s = self.ascii_digits(&self.time(Time::constant(13, 45, 0, 0), HourCycle::H12));
        let digit = s.find(|c: char| c.is_ascii_digit());
        if s.find(&self.day_period(DayPeriod::Pm))
            .zip(digit)
            .is_some_and(|(p, d)| p < d)
        {
            PeriodPosition::BeforeTime
        } else {
            PeriodPosition::AfterTime
        }
    }

    /// Lenient parse of [`Self::number`] output: native digits, grouping and sign variants.
    ///
    /// Where the decimal separator is not `.`, a typed `.` without that separator is the decimal
    /// point, e.g. `0.6` in de-DE; unless the locale groups with `.` and the number is well-formed
    /// digit groups (a 1-3 digit lead without leading zero, then 3-digit groups): `1.500` stays
    /// 1500 there, `0.345` and `1234.5` are decimals.
    fn parse_number(&self, s: &str) -> Option<f64> {
        let dec = self.decimal_separator();
        let ascii = self.ascii_digits(s.trim());
        let dot_is_decimal = dec != '.' && !ascii.contains(dec) && dot_is_decimal(self, &ascii);
        let mut out = String::with_capacity(s.len());
        for c in ascii.chars() {
            match c {
                '0'..='9' | 'e' | 'E' => out.push(c),
                '-' | '\u{2212}' => out.push('-'),
                // Bidi marks ICU wraps signs in, e.g. U+061C in ar.
                '+' | '\u{061C}' | '\u{200E}' | '\u{200F}' => {}
                c if c == dec || (c == '.' && dot_is_decimal) => out.push('.'),
                c if c.is_whitespace() || matches!(c, ',' | '.' | '\'' | '\u{2019}' | '\u{066C}') => {}
                _ => return None,
            }
        }
        out.parse().ok()
    }

    /// Lenient parse of three digit groups in [`Self::date_order`]; `None` for any other shape, an invalid
    /// date or a year outside 1..=9999.
    fn parse_date(&self, s: &str) -> Option<Date> {
        let ascii = self.ascii_digits(s);
        let mut parts = ascii.split(|c: char| !c.is_ascii_digit()).filter(|p| !p.is_empty());
        let mut fields = [0i32; 3];
        for f in &mut fields {
            *f = parts.next()?.parse().ok()?;
        }
        if parts.next().is_some() {
            return None;
        }
        let (mut y, mut m, mut d) = (0, 0, 0);
        for (field, v) in self.date_order().iter().zip(fields) {
            match field {
                DatePart::Year => y = v,
                DatePart::Month => m = v,
                DatePart::Day => d = v,
            }
        }
        if !(1..=9999).contains(&y) {
            return None;
        }
        Date::new(i16::try_from(y).ok()?, i8::try_from(m).ok()?, i8::try_from(d).ok()?).ok()
    }
}

// See `parse_number`.
fn dot_is_decimal<F: LocaleFormat + ?Sized>(f: &F, ascii: &str) -> bool {
    f.grouping_separator() != Some('.') || !reads_as_dot_groups(ascii)
}

fn reads_as_dot_groups(ascii: &str) -> bool {
    let number = ascii
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .next()
        .unwrap_or_default();
    let mut groups = number.split('.');
    let lead = groups.next().unwrap_or_default();
    let mut rest = groups.peekable();
    (1..=3).contains(&lead.len()) && !lead.starts_with('0') && rest.peek().is_some() && rest.all(|g| g.len() == 3)
}

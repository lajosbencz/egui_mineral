use jiff::civil::{Date, Time, Weekday};

#[cfg(not(feature = "icu"))]
use fluent_bundle::FluentValue;
use unic_langid::{CharacterDirection, LanguageIdentifier};

use super::{
    Direction, I18nError,
    format::{ASCII_DIGITS, DateFields, DateLength, DatePart, DayPeriod, HourCycle, ListKind, LocaleFormat, NameWidth},
};

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// Data-free [`LocaleFormat`]: English names, ISO 8601 field order, 24-hour clock, `.` decimals, Monday-first
/// weeks, code point collation; keeps the locale's tag and script direction. Formatting without the `icu`
/// feature.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AsciiFormat {
    tag: String,
    direction: Direction,
}

impl AsciiFormat {
    /// For BCP 47 `tag`.
    ///
    /// # Errors
    /// [`I18nError::Locale`] when `tag` does not parse.
    pub fn new(tag: &str) -> Result<Self, I18nError> {
        let id: LanguageIdentifier = tag.parse().map_err(|e| I18nError::Locale {
            tag: tag.into(),
            source: Some(Box::new(e)),
        })?;
        let direction = match id.character_direction() {
            CharacterDirection::RTL => Direction::Rtl,
            CharacterDirection::LTR | CharacterDirection::TTB => Direction::Ltr,
        };
        Ok(Self {
            tag: id.to_string(),
            direction,
        })
    }
}

/// CLDR root (`und`), left to right.
impl Default for AsciiFormat {
    fn default() -> Self {
        Self {
            tag: super::ROOT.to_owned(),
            direction: Direction::Ltr,
        }
    }
}

#[cfg(not(feature = "icu"))]
pub(super) fn fluent_number(v: &FluentValue<'_>, _: &intl_memoizer::concurrent::IntlLangMemoizer) -> Option<String> {
    let FluentValue::Number(n) = v else {
        return None;
    };
    Some(AsciiFormat::default().number(n.value, super::format::fluent_fraction_digits(n)))
}

fn name(full: &str, width: NameWidth) -> String {
    let len = match width {
        NameWidth::Narrow => 1,
        NameWidth::Abbreviated => 3,
        NameWidth::Wide => full.len(),
    };
    full.get(..len).unwrap_or(full).to_owned()
}

fn group_thousands(int: &str) -> String {
    let mut out = String::with_capacity(int.len() + int.len() / 3);
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

impl LocaleFormat for AsciiFormat {
    fn tag(&self) -> &str {
        &self.tag
    }

    fn direction(&self) -> Direction {
        self.direction
    }

    fn first_weekday(&self) -> Weekday {
        Weekday::Monday
    }

    fn hour_cycle(&self) -> HourCycle {
        HourCycle::H23
    }

    fn month_name(&self, month: i8, width: NameWidth) -> String {
        let i = usize::try_from(month.clamp(1, 12) - 1).unwrap_or(0);
        name(MONTHS[i], width)
    }

    fn weekday_name(&self, day: Weekday, width: NameWidth) -> String {
        let i = usize::try_from(day.to_monday_zero_offset()).unwrap_or(0);
        name(WEEKDAYS[i], width)
    }

    fn day_period(&self, period: DayPeriod) -> String {
        match period {
            DayPeriod::Am => "AM",
            DayPeriod::Pm => "PM",
        }
        .to_owned()
    }

    fn date(&self, d: Date, fields: DateFields, length: DateLength) -> String {
        let (y, m, day) = (d.year(), d.month(), d.day());
        let width = match length {
            DateLength::Numeric | DateLength::Medium => NameWidth::Abbreviated,
            DateLength::Long => NameWidth::Wide,
        };
        let month = || self.month_name(m, width);
        let core = match (fields, length) {
            (DateFields::DayWeekday, _) => day.to_string(),
            (DateFields::YearMonthDay | DateFields::YearMonthDayWeekday, DateLength::Numeric) => {
                format!("{y:04}-{m:02}-{day:02}")
            }
            (DateFields::YearMonthDay | DateFields::YearMonthDayWeekday, _) => format!("{y} {} {day}", month()),
            (DateFields::MonthDay | DateFields::MonthDayWeekday, DateLength::Numeric) => format!("{m:02}-{day:02}"),
            (DateFields::MonthDay | DateFields::MonthDayWeekday, _) => format!("{} {day}", month()),
        };
        match fields {
            DateFields::YearMonthDay | DateFields::MonthDay => core,
            DateFields::YearMonthDayWeekday | DateFields::MonthDayWeekday | DateFields::DayWeekday => {
                format!("{} {core}", self.weekday_name(d.weekday(), width))
            }
        }
    }

    fn month_year(&self, year: i16, month: i8) -> String {
        format!("{year} {}", self.month_name(month, NameWidth::Wide))
    }

    fn time(&self, t: Time, cycle: HourCycle) -> String {
        let (h, m) = (t.hour(), t.minute());
        match cycle {
            HourCycle::H23 => format!("{h:02}:{m:02}"),
            HourCycle::H12 => {
                let period = self.day_period(if h < 12 { DayPeriod::Am } else { DayPeriod::Pm });
                format!("{}:{m:02} {period}", (h + 11) % 12 + 1)
            }
        }
    }

    fn number(&self, value: f64, fraction_digits: Option<i16>) -> String {
        if !value.is_finite() {
            crate::kit::sanitize::report(format_args!("number {value} is not formattable"));
            return value.to_string();
        }
        let plain = match fraction_digits {
            None => value.to_string(),
            Some(d) if d >= 0 => format!("{value:.*}", usize::from(d.unsigned_abs())),
            Some(d) => {
                let scale = 10f64.powi(i32::from(d.unsigned_abs()));
                let rounded = if scale.is_finite() {
                    (value / scale).round() * scale
                } else {
                    0.0
                };
                format!("{rounded:.0}")
            }
        };
        let (sign, unsigned) = plain.strip_prefix('-').map_or(("", plain.as_str()), |u| ("-", u));
        let (int, frac) = unsigned.split_once('.').map_or((unsigned, None), |(i, f)| (i, Some(f)));
        let mut out = format!("{sign}{}", group_thousands(int));
        if let Some(f) = frac {
            out.push('.');
            out.push_str(f);
        }
        out
    }

    fn digits(&self) -> [char; 10] {
        ASCII_DIGITS
    }

    fn decimal_separator(&self) -> char {
        '.'
    }

    fn grouping_separator(&self) -> Option<char> {
        Some(',')
    }

    fn date_order(&self) -> [DatePart; 3] {
        [DatePart::Year, DatePart::Month, DatePart::Day]
    }

    fn compare(&self, a: &str, b: &str) -> std::cmp::Ordering {
        let fold = |s: &str| s.chars().flat_map(char::to_lowercase).collect::<Vec<_>>();
        fold(a).cmp(&fold(b)).then_with(|| a.cmp(b))
    }

    fn list(&self, items: &[&str], kind: ListKind) -> String {
        let conj = match kind {
            ListKind::And => Some("and"),
            ListKind::Or => Some("or"),
            ListKind::Unit => None,
        };
        match (items, conj) {
            ([], _) => String::new(),
            ([only], _) => (*only).to_owned(),
            ([a, b], Some(c)) => format!("{a} {c} {b}"),
            ([head @ .., last], Some(c)) => format!("{}, {c} {last}", head.join(", ")),
            (_, None) => items.join(", "),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_without_locale_data() {
        let f = AsciiFormat::default();
        let d = jiff::civil::date(2026, 11, 17);
        assert_eq!(f.date(d, DateFields::YearMonthDay, DateLength::Numeric), "2026-11-17");
        assert_eq!(
            f.date(d, DateFields::YearMonthDayWeekday, DateLength::Long),
            "Tuesday 2026 November 17"
        );
        assert_eq!(f.date(d, DateFields::DayWeekday, DateLength::Medium), "Tue 17");
        assert_eq!(f.month_year(2026, 13), "2026 December", "month clamps");
        assert_eq!(f.month_name(0, NameWidth::Narrow), "J");
        assert_eq!(f.time(Time::constant(0, 5, 0, 0), HourCycle::H12), "12:05 AM");
        assert_eq!(f.time(Time::constant(13, 45, 0, 0), HourCycle::H23), "13:45");
        assert_eq!(f.time_separator(), ":");
        assert_eq!(f.number(-1_234_567.891, Some(2)), "-1,234,567.89");
        assert_eq!(f.number(1234.5, None), "1,234.5");
        assert_eq!(f.number(1250.0, Some(-2)), "1,300");
        assert_eq!(f.number(5.0, Some(-400)), "0", "overflowing magnitude");
        assert_eq!(f.list(&["a", "b", "c"], ListKind::And), "a, b, and c");
        assert_eq!(f.list(&["a", "b"], ListKind::Or), "a or b");
        assert_eq!(f.list(&["3 ft", "7 in"], ListKind::Unit), "3 ft, 7 in");
        assert!(f.compare("apple", "Banana").is_lt());
    }

    #[test]
    fn numeric_output_parses_back() {
        let f = AsciiFormat::default();
        let d = jiff::civil::date(812, 2, 3);
        assert_eq!(
            f.parse_date(&f.date(d, DateFields::YearMonthDay, DateLength::Numeric)),
            Some(d)
        );
        assert_eq!(f.parse_number(&f.number(-9_876.5, None)), Some(-9_876.5));
    }

    #[test]
    fn keeps_the_tag_and_script_direction() {
        let ar = AsciiFormat::new("ar-EG").unwrap();
        assert_eq!((ar.tag(), ar.direction()), ("ar-EG", Direction::Rtl));
        assert_eq!(AsciiFormat::new("en-US").unwrap().direction(), Direction::Ltr);
        assert_eq!(
            AsciiFormat::new("ar-Latn").unwrap().direction(),
            Direction::Ltr,
            "script wins"
        );
        assert!(AsciiFormat::new("not a tag!").is_err());
    }
}

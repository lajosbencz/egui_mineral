use std::borrow::Cow;

use fixed_decimal::{Decimal, FloatPrecision};
use fluent_bundle::FluentValue;
use icu_calendar::{Date as IcuDate, Gregorian, week::WeekInformation};
use icu_collator::{CollatorBorrowed, options::CollatorOptions};
use icu_datetime::{
    DateTimeFormatterPreferences, FixedCalendarDateTimeFormatter,
    fieldsets::{DE, MD, MDE, T, YM, YMD, YMDE, enums::DateFieldSet},
    options::{Length, YearStyle},
    pattern::{DateTimePattern, FixedCalendarDateTimeNames},
};
use icu_decimal::{DecimalFormatter, options::DecimalFormatterOptions};
use icu_list::{ListFormatter, options::ListFormatterOptions};
use icu_locale::LocaleDirectionality;
use icu_locale_core::{Locale, preferences::extensions::unicode::keywords::HourCycle as IcuHourCycle};
use icu_time::{DateTime, Time as IcuTime};
use jiff::civil::{Date, Time, Weekday};
use writeable::TryWriteable;

use super::{DateFields, DateLength, DatePart, DayPeriod, HourCycle, I18nError, ListKind, LocaleFormat, NameWidth};
use crate::i18n::Direction;

/// [`LocaleFormat`] backed by ICU4X and CLDR data.
#[derive(Debug)]
pub struct IcuFormat {
    locale: super::Locale,
    direction: Direction,
    first_weekday: Weekday,
    hour_cycle: HourCycle,
    months: [[String; 12]; 3],
    weekdays: [[String; 7]; 3],
    periods: [String; 2],
    dates: Vec<FixedCalendarDateTimeFormatter<Gregorian, DateFieldSet>>,
    month_year: FixedCalendarDateTimeFormatter<Gregorian, YM>,
    time12: FixedCalendarDateTimeFormatter<(), T>,
    time23: FixedCalendarDateTimeFormatter<(), T>,
    decimal: DecimalFormatter,
    collator: CollatorBorrowed<'static>,
    lists: [ListFormatter; 3],
    digits: [char; 10],
    decimal_separator: char,
    grouping_separator: Option<char>,
    date_order: [DatePart; 3],
}

fn err(tag: &str, e: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> I18nError {
    I18nError::Locale {
        tag: tag.to_owned(),
        source: Some(e.into()),
    }
}

fn icu_date(d: Date) -> Option<IcuDate<Gregorian>> {
    IcuDate::try_new_gregorian(i32::from(d.year()), d.month().unsigned_abs(), d.day().unsigned_abs())
        .inspect_err(|e| crate::kit::sanitize::report(format_args!("ICU rejects {d}: {e}")))
        .ok()
}

fn icu_time(t: Time) -> Option<IcuTime> {
    IcuTime::try_new(
        t.hour().unsigned_abs(),
        t.minute().unsigned_abs(),
        t.second().unsigned_abs(),
        0,
    )
    .inspect_err(|e| crate::kit::sanitize::report(format_args!("ICU rejects {t}: {e}")))
    .ok()
}

fn month_index(month: i8) -> usize {
    if !(1..=12).contains(&month) {
        crate::kit::sanitize::report(format_args!("month must be in 1..=12, got {month}"));
    }
    usize::from(month.clamp(1, 12).unsigned_abs()).saturating_sub(1)
}

const DATE_FIELDS: [DateFields; 5] = [
    DateFields::YearMonthDay,
    DateFields::YearMonthDayWeekday,
    DateFields::MonthDay,
    DateFields::MonthDayWeekday,
    DateFields::DayWeekday,
];
const DATE_LENGTHS: [DateLength; 3] = [DateLength::Numeric, DateLength::Medium, DateLength::Long];

fn date_field_set(fields: DateFields, length: DateLength) -> DateFieldSet {
    let len = match length {
        DateLength::Numeric => Length::Short,
        DateLength::Medium => Length::Medium,
        DateLength::Long => Length::Long,
    };
    let year = if length == DateLength::Numeric {
        YearStyle::Full
    } else {
        YearStyle::Auto
    };
    match fields {
        DateFields::YearMonthDay => DateFieldSet::YMD(YMD::for_length(len).with_year_style(year)),
        DateFields::YearMonthDayWeekday => DateFieldSet::YMDE(YMDE::for_length(len).with_year_style(year)),
        DateFields::MonthDay => DateFieldSet::MD(MD::for_length(len)),
        DateFields::MonthDayWeekday => DateFieldSet::MDE(MDE::for_length(len)),
        DateFields::DayWeekday => DateFieldSet::DE(DE::for_length(len)),
    }
}

fn date_index(fields: DateFields, length: DateLength) -> usize {
    let f = match fields {
        DateFields::YearMonthDay => 0,
        DateFields::YearMonthDayWeekday => 1,
        DateFields::MonthDay => 2,
        DateFields::MonthDayWeekday => 3,
        DateFields::DayWeekday => 4,
    };
    let l = match length {
        DateLength::Numeric => 0,
        DateLength::Medium => 1,
        DateLength::Long => 2,
    };
    f * DATE_LENGTHS.len() + l
}

fn width_index(w: NameWidth) -> usize {
    match w {
        NameWidth::Narrow => 0,
        NameWidth::Abbreviated => 1,
        NameWidth::Wide => 2,
    }
}

fn pattern_is_h23(p: &DateTimePattern) -> bool {
    let mut quoted = false;
    for c in p.to_string().chars() {
        match c {
            '\'' => quoted = !quoted,
            'H' | 'k' if !quoted => return true,
            'h' | 'K' if !quoted => return false,
            _ => {}
        }
    }
    true
}

type Names = ([[String; 12]; 3], [[String; 7]; 3], [String; 2]);

fn names(tag: &str, prefs: DateTimeFormatterPreferences) -> Result<Names, I18nError> {
    let name = |pattern: &str, at: DateTime<Gregorian>| -> Result<String, I18nError> {
        let pattern: DateTimePattern = pattern.parse().map_err(|e| err(tag, e))?;
        let mut names = FixedCalendarDateTimeNames::<Gregorian>::try_new(prefs).map_err(|e| err(tag, e))?;
        let f = names.include_for_pattern(&pattern).map_err(|e| err(tag, e))?;
        f.format(&at)
            .try_write_to_string()
            .map(Cow::into_owned)
            .map_err(|(e, _)| err(tag, super::Messages::boxed([e])))
    };
    let at = |m: u8, d: u8, h: u8| -> Result<DateTime<Gregorian>, I18nError> {
        Ok(DateTime {
            date: IcuDate::try_new_gregorian(2024, m, d).map_err(|e| err(tag, e))?,
            time: IcuTime::try_new(h, 0, 0, 0).map_err(|e| err(tag, e))?,
        })
    };
    let mut months: [[String; 12]; 3] = Default::default();
    let mut weekdays: [[String; 7]; 3] = Default::default();
    for (w, (mp, wp)) in [("LLLLL", "ccccc"), ("LLL", "ccc"), ("LLLL", "cccc")]
        .into_iter()
        .enumerate()
    {
        for m in 0..12u8 {
            months[w][m as usize] = name(mp, at(m + 1, 15, 12)?)?;
        }
        // 2024-01-01 is a Monday.
        for d in 0..7u8 {
            weekdays[w][d as usize] = name(wp, at(1, d + 1, 12)?)?;
        }
    }
    let periods = [name("a", at(1, 1, 1)?)?, name("a", at(1, 1, 13)?)?];
    Ok((months, weekdays, periods))
}

impl IcuFormat {
    /// Loads ICU4X data for `id`; without baked data for it, CLDR root's.
    ///
    /// # Errors
    /// [`I18nError::Locale`] when its data does not load, formats no digit or decimal separator,
    /// or has no numeric date pattern.
    pub fn new(id: &super::Locale) -> Result<Self, I18nError> {
        let tag = &id.to_string();
        let locale = Locale::try_from_str(tag).map_err(|e| err(tag, e))?;
        let prefs: DateTimeFormatterPreferences = (&locale).into();
        let with_hc = |hc| {
            let mut p = prefs;
            p.hour_cycle = Some(hc);
            p
        };

        let direction = if LocaleDirectionality::new_common().is_right_to_left(&locale.id) {
            Direction::Rtl
        } else {
            Direction::Ltr
        };
        let week = WeekInformation::try_new((&locale).into()).map_err(|e| err(tag, e))?;
        let first_weekday = Weekday::from_monday_one_offset(week.first_weekday as i8).map_err(|e| err(tag, e))?;

        let dates = DATE_FIELDS
            .iter()
            .flat_map(|f| DATE_LENGTHS.iter().map(|l| date_field_set(*f, *l)))
            .map(|set| FixedCalendarDateTimeFormatter::try_new(prefs, set).map_err(|e| err(tag, e)))
            .collect::<Result<Vec<_>, _>>()?;
        let month_year = FixedCalendarDateTimeFormatter::try_new(prefs, YM::long()).map_err(|e| err(tag, e))?;
        let time = FixedCalendarDateTimeFormatter::<(), T>::try_new(prefs, T::hm()).map_err(|e| err(tag, e))?;
        let time12 =
            FixedCalendarDateTimeFormatter::try_new(with_hc(IcuHourCycle::H12), T::hm()).map_err(|e| err(tag, e))?;
        let time23 =
            FixedCalendarDateTimeFormatter::try_new(with_hc(IcuHourCycle::H23), T::hm()).map_err(|e| err(tag, e))?;
        let one_pm = IcuTime::try_new(13, 0, 0, 0).map_err(|e| err(tag, e))?;
        let hour_cycle = if pattern_is_h23(&time.format(&one_pm).pattern()) {
            HourCycle::H23
        } else {
            HourCycle::H12
        };

        let (months, weekdays, periods) = names(tag, prefs)?;

        let decimal =
            DecimalFormatter::try_new((&locale).into(), DecimalFormatterOptions::default()).map_err(|e| err(tag, e))?;
        let mut digits = ['0'; 10];
        for (n, digit) in (0u8..).zip(&mut digits) {
            *digit = decimal
                .format(&Decimal::from(n))
                .to_string()
                .chars()
                .next()
                .ok_or_else(|| err(tag, "no digit glyph"))?;
        }
        let half =
            Decimal::try_from_f64(1.5, FloatPrecision::RoundTrip).map_err(|e| err(tag, super::Messages::boxed([e])))?;
        let decimal_separator = decimal
            .format(&half)
            .to_string()
            .chars()
            .find(|c| !digits.contains(c) && !c.is_ascii_digit())
            .ok_or_else(|| err(tag, "no decimal separator"))?;
        let grouping_separator = decimal
            .format(&Decimal::from(1_000_000u32))
            .to_string()
            .chars()
            .find(|c| !digits.contains(c) && !c.is_ascii_digit());

        let collator =
            CollatorBorrowed::try_new((&locale).into(), CollatorOptions::default()).map_err(|e| err(tag, e))?;
        let lopts = ListFormatterOptions::default();
        let lists = [
            ListFormatter::try_new_and((&locale).into(), lopts).map_err(|e| err(tag, e))?,
            ListFormatter::try_new_or((&locale).into(), lopts).map_err(|e| err(tag, e))?,
            ListFormatter::try_new_unit((&locale).into(), lopts).map_err(|e| err(tag, e))?,
        ];

        let mut this = Self {
            locale: id.clone(),
            direction,
            first_weekday,
            hour_cycle,
            months,
            weekdays,
            periods,
            dates,
            month_year,
            time12,
            time23,
            decimal,
            collator,
            lists,
            digits,
            decimal_separator,
            grouping_separator,
            date_order: [DatePart::Month, DatePart::Day, DatePart::Year],
        };
        this.date_order = this
            .detect_date_order()
            .ok_or_else(|| err(tag, "numeric date pattern"))?;
        Ok(this)
    }

    fn detect_date_order(&self) -> Option<[DatePart; 3]> {
        const PROBE: Date = Date::constant(2033, 11, 22);
        let s = self.ascii_digits(&self.date(PROBE, DateFields::YearMonthDay, DateLength::Numeric));
        let mut found = [
            (s.find("2033")?, DatePart::Year),
            (s.find("11")?, DatePart::Month),
            (s.find("22")?, DatePart::Day),
        ];
        found.sort_by_key(|(i, _)| *i);
        Some(found.map(|(_, f)| f))
    }
}

impl LocaleFormat for IcuFormat {
    fn locale(&self) -> &super::Locale {
        &self.locale
    }

    fn direction(&self) -> Direction {
        self.direction
    }

    fn first_weekday(&self) -> Weekday {
        self.first_weekday
    }

    fn hour_cycle(&self) -> HourCycle {
        self.hour_cycle
    }

    fn month_name(&self, month: i8, width: NameWidth) -> String {
        self.months[width_index(width)][month_index(month)].clone()
    }

    fn weekday_name(&self, day: Weekday, width: NameWidth) -> String {
        let i = usize::from(day.to_monday_zero_offset().unsigned_abs());
        self.weekdays[width_index(width)][i].clone()
    }

    fn day_period(&self, period: DayPeriod) -> String {
        match period {
            DayPeriod::Am => self.periods[0].clone(),
            DayPeriod::Pm => self.periods[1].clone(),
        }
    }

    fn date(&self, d: Date, fields: DateFields, length: DateLength) -> String {
        icu_date(d).map_or_else(String::new, |d| {
            self.dates[date_index(fields, length)].format(&d).to_string()
        })
    }

    fn month_year(&self, year: i16, month: i8) -> String {
        let month = u8::try_from(month_index(month).saturating_add(1)).unwrap_or(1);
        IcuDate::try_new_gregorian(i32::from(year), month, 1)
            .inspect_err(|e| crate::kit::sanitize::report(format_args!("ICU rejects year {year}: {e}")))
            .map_or_else(|_| String::new(), |d| self.month_year.format(&d).to_string())
    }

    fn time(&self, t: Time, cycle: HourCycle) -> String {
        let f = match cycle {
            HourCycle::H12 => &self.time12,
            HourCycle::H23 => &self.time23,
        };
        icu_time(t).map_or_else(String::new, |t| f.format(&t).to_string())
    }

    fn number(&self, value: f64, fraction_digits: Option<i16>) -> String {
        format_f64(&self.decimal, value, fraction_digits)
    }

    fn digits(&self) -> [char; 10] {
        self.digits
    }

    fn decimal_separator(&self) -> char {
        self.decimal_separator
    }

    fn grouping_separator(&self) -> Option<char> {
        self.grouping_separator
    }

    fn date_order(&self) -> [DatePart; 3] {
        self.date_order
    }

    fn compare(&self, a: &str, b: &str) -> std::cmp::Ordering {
        self.collator.compare(a, b)
    }

    fn list(&self, items: &[&str], kind: ListKind) -> String {
        let f = &self.lists[match kind {
            ListKind::And => 0,
            ListKind::Or => 1,
            ListKind::Unit => 2,
        }];
        f.format(items.iter().copied()).to_string()
    }
}

fn format_f64(f: &DecimalFormatter, value: f64, fraction_digits: Option<i16>) -> String {
    let precision = fraction_digits.map_or(FloatPrecision::RoundTrip, |d| {
        FloatPrecision::Magnitude(d.saturating_neg())
    });
    match Decimal::try_from_f64(value, precision) {
        Ok(d) => f.format(&d).to_string(),
        Err(e) => {
            crate::kit::sanitize::report(format_args!("number {value} is not formattable: {e}"));
            value.to_string()
        }
    }
}

struct FluentDecimal(DecimalFormatter);

impl intl_memoizer::Memoizable for FluentDecimal {
    type Args = ();
    type Error = String;

    fn construct(lang: unic_langid::LanguageIdentifier, (): ()) -> Result<Self, String> {
        let locale = Locale::try_from_str(&lang.to_string()).map_err(|e| e.to_string())?;
        DecimalFormatter::try_new((&locale).into(), DecimalFormatterOptions::default())
            .map(Self)
            .map_err(|e| e.to_string())
    }
}

pub(super) fn fluent_number(v: &FluentValue<'_>, memo: &intl_memoizer::concurrent::IntlLangMemoizer) -> Option<String> {
    let FluentValue::Number(n) = v else {
        return None;
    };
    let digits = super::format::fluent_fraction_digits(n);
    memo.with_try_get::<FluentDecimal, _, _>((), |f| format_f64(&f.0, n.value, digits))
        .inspect_err(|e| crate::kit::sanitize::report(format_args!("fluent NUMBER formatter: {e}")))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_mapping_and_round_trips() {
        let d = Date::new(2026, 11, 18).unwrap();
        let cases = [
            (
                "de-DE",
                Weekday::Monday,
                HourCycle::H23,
                [DatePart::Day, DatePart::Month, DatePart::Year],
                Direction::Ltr,
            ),
            (
                "en-US",
                Weekday::Sunday,
                HourCycle::H12,
                [DatePart::Month, DatePart::Day, DatePart::Year],
                Direction::Ltr,
            ),
            (
                "ar-EG",
                Weekday::Saturday,
                HourCycle::H12,
                [DatePart::Day, DatePart::Month, DatePart::Year],
                Direction::Rtl,
            ),
        ];
        for (locale, weekday, cycle, order, direction) in cases {
            let f = IcuFormat::new(&locale.parse().unwrap()).unwrap();
            assert_eq!(
                (f.first_weekday(), f.hour_cycle(), f.date_order(), f.direction()),
                (weekday, cycle, order, direction),
                "{locale}"
            );
            let numeric = f.date(d, DateFields::YearMonthDay, DateLength::Numeric);
            assert_eq!(f.parse_date(&numeric), Some(d), "{locale} date {numeric}");
            for n in [1_234_567.5, 0.25, -1_234.5] {
                let text = f.number(n, Some(2));
                assert_eq!(f.parse_number(&text), Some(n), "{locale} number {text}");
            }
        }
    }

    #[test]
    fn typed_dot_is_a_decimal_point_unless_it_reads_as_grouping() {
        for (locale, text, n) in [
            ("de-DE", "0.6", 0.6),
            ("de-DE", "1.25", 1.25),
            ("de-DE", "1.500", 1500.0),
            ("de-DE", "1.234.567", 1_234_567.0),
            ("de-DE", "1.500,5", 1500.5),
            ("de-DE", "-1.500", -1500.0),
            ("de-DE", "0.345", 0.345),
            ("de-DE", "1234.567", 1234.567),
            ("it-IT", "1.500", 1500.0),
            ("fr-FR", "0.6", 0.6),
            ("fr-FR", "1.500", 1.5),
            ("en-US", "1,500.5", 1500.5),
        ] {
            let f = IcuFormat::new(&locale.parse().unwrap()).unwrap();
            assert_eq!(f.parse_number(text), Some(n), "{locale} {text}");
        }
        let de = IcuFormat::new(&crate::i18n::locale!("de-DE")).unwrap();
        assert_eq!(de.parse_number("1.23.4"), None);
        assert_eq!(de.grouping_separator(), Some('.'));
        assert_eq!(
            IcuFormat::new(&crate::i18n::locale!("en-US"))
                .unwrap()
                .grouping_separator(),
            Some(',')
        );
    }

    #[test]
    fn out_of_range_arguments_are_total() {
        let f = IcuFormat::new(&crate::i18n::locale!("en-US")).unwrap();
        assert_eq!(f.month_name(0, NameWidth::Wide), f.month_name(1, NameWidth::Wide));
        assert_eq!(f.month_name(40, NameWidth::Wide), f.month_name(12, NameWidth::Wide));
        assert_ne!(f.month_year(2024, -1), "");
        assert_eq!(f.number(f64::NAN, Some(2)), "NaN");
        assert_ne!(f.number(1.5, Some(i16::MIN)), "");
        assert_eq!(f.count(usize::MAX), f.integer(i64::MAX));
    }

    #[test]
    fn parse_date_accepts_years_one_to_9999() {
        let f = IcuFormat::new(&crate::i18n::locale!("de-DE")).unwrap();
        assert_eq!(f.parse_date("1.1.0001"), Date::new(1, 1, 1).ok());
        assert_eq!(f.parse_date("31.12.9999"), Date::new(9999, 12, 31).ok());
        assert_eq!(f.parse_date("1.1.0000"), None);
        assert_eq!(f.parse_date("1.1.10000"), None);
    }
}

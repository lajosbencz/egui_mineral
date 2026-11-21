use egui::emath::Numeric;

use super::{Blank, ValidationError, Validator};
use crate::i18n::{LocaleFormat, msg};
use crate::select::Selection;

/// Fails on blank values ([`Blank`]): blank text, an unchecked checkbox, nothing picked.
#[derive(Clone, Debug)]
pub struct Required;

impl<V: Blank + ?Sized> Validator<V> for Required {
    fn validate(&self, value: &V, _locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        if value.is_blank() {
            Err(ValidationError::localized(msg::REQUIRED))
        } else {
            Ok(())
        }
    }
}

/// Fails below the given number of characters, or of picked options of a multi-select.
#[derive(Clone, Debug)]
pub struct MinLength(pub usize);

impl Validator<String> for MinLength {
    fn validate(&self, value: &String, _locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        if value.chars().count() < self.0 {
            Err(ValidationError::localized(msg::MinLength { min: self.0.into() }))
        } else {
            Ok(())
        }
    }
}

impl<K: Ord + Clone> Validator<Selection<K>> for MinLength {
    fn validate(&self, value: &Selection<K>, _locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        if value.len() < self.0 {
            Err(ValidationError::localized(msg::MinPicks { min: self.0.into() }))
        } else {
            Ok(())
        }
    }
}

/// Fails above the given number of characters, or of picked options of a multi-select.
#[derive(Clone, Debug)]
pub struct MaxLength(pub usize);

impl Validator<String> for MaxLength {
    fn validate(&self, value: &String, _locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        if value.chars().count() > self.0 {
            Err(ValidationError::localized(msg::MaxLength { max: self.0.into() }))
        } else {
            Ok(())
        }
    }
}

impl<K: Ord + Clone> Validator<Selection<K>> for MaxLength {
    fn validate(&self, value: &Selection<K>, _locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        if value.len() > self.0 {
            Err(ValidationError::localized(msg::MaxPicks { max: self.0.into() }))
        } else {
            Ok(())
        }
    }
}

/// Fails on numbers outside the optional bounds.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct NumberRange {
    /// Inclusive lower bound; `None` is open.
    pub min: Option<f64>,
    /// Inclusive upper bound; `None` is open.
    pub max: Option<f64>,
}

impl NumberRange {
    /// Fails below `min`.
    #[must_use]
    pub const fn at_least(min: f64) -> Self {
        Self {
            min: Some(min),
            max: None,
        }
    }

    /// Fails above `max`.
    #[must_use]
    pub const fn at_most(max: f64) -> Self {
        Self {
            min: None,
            max: Some(max),
        }
    }

    /// Fails outside `min..=max`.
    #[must_use]
    pub const fn between(min: f64, max: f64) -> Self {
        Self {
            min: Some(min),
            max: Some(max),
        }
    }
}

impl<N: Numeric> Validator<N> for NumberRange {
    fn validate(&self, value: &N, _locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        let n = value.to_f64();
        let err = match (self.min, self.max) {
            (Some(a), Some(b)) if n < a || n > b => ValidationError::localized(msg::RangeBetween {
                min: a.into(),
                max: b.into(),
            }),
            (Some(a), None) if n < a => ValidationError::localized(msg::RangeMin { min: a.into() }),
            (None, Some(b)) if n > b => ValidationError::localized(msg::RangeMax { max: b.into() }),
            _ => return Ok(()),
        };
        Err(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_and_range() {
        let en = crate::test_support::en_us();
        assert!(MinLength(2).validate(&"ab".to_owned(), &en).is_ok());
        assert!(MaxLength(1).validate(&"ab".to_owned(), &en).is_err());
        let picks: Selection<u8> = [1, 2].into_iter().collect();
        assert!(MaxLength(1).validate(&picks, &en).is_err(), "counts picks");
        let r = NumberRange::at_least(1.0);
        assert_eq!(r.validate(&0.5, &en).unwrap_err().code, "mineral-range-min");
        assert!(r.validate(&3u8, &en).is_ok());
        assert!(Required.validate(&false, &en).is_err());
        assert!(Required.validate(&Some(3), &en).is_ok());
    }
}

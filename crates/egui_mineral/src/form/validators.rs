use super::{ValidationError, Validator};
use crate::i18n::{LocaleFormat, msg};

/// Fails on empty or whitespace-only input.
#[derive(Clone, Debug)]
pub struct Required;

impl Validator for Required {
    fn validate(&self, value: &str, _locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        if value.trim().is_empty() {
            Err(ValidationError::localized(msg::REQUIRED))
        } else {
            Ok(())
        }
    }
}

/// Fails below the given number of characters.
#[derive(Clone, Debug)]
pub struct MinLength(pub usize);

impl Validator for MinLength {
    fn validate(&self, value: &str, _locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        if value.chars().count() >= self.0 {
            Ok(())
        } else {
            Err(ValidationError::localized(msg::MinLength { min: self.0.into() }))
        }
    }
}

/// Fails above the given number of characters.
#[derive(Clone, Debug)]
pub struct MaxLength(pub usize);

impl Validator for MaxLength {
    fn validate(&self, value: &str, _locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        if value.chars().count() <= self.0 {
            Ok(())
        } else {
            Err(ValidationError::localized(msg::MaxLength { max: self.0.into() }))
        }
    }
}

/// Fails on non-numbers and values outside the optional bounds.
#[derive(Clone, Debug, PartialEq)]
pub struct NumberRange {
    /// Inclusive lower bound; `None` is open.
    pub min: Option<f64>,
    /// Inclusive upper bound; `None` is open.
    pub max: Option<f64>,
}

impl Validator for NumberRange {
    fn validate(&self, value: &str, locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        let Some(n) = locale.parse_number(value) else {
            return Err(ValidationError::localized(msg::NUMBER));
        };
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
        assert!(MinLength(2).validate("ab", &en).is_ok());
        assert!(MaxLength(1).validate("ab", &en).is_err());
        let r = NumberRange {
            min: Some(1.0),
            max: None,
        };
        assert_eq!(r.validate("0.5", &en).unwrap_err().code, "mineral-range-min");
    }
}

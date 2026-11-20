use super::{FieldValue, ValidationError, Validator};
use crate::i18n::{LocaleFormat, msg};

/// Fails on empty values ([`FieldValue::is_empty`]): blank text, an unchecked checkbox, nothing picked.
#[derive(Clone, Debug)]
pub struct Required;

impl Validator for Required {
    fn validate(&self, value: &FieldValue, _locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        if value.is_empty() {
            Err(ValidationError::localized(msg::REQUIRED))
        } else {
            Ok(())
        }
    }
}

/// Fails below the given number of characters, or of picked options of a multi-select.
#[derive(Clone, Debug)]
pub struct MinLength(pub usize);

impl Validator for MinLength {
    fn validate(&self, value: &FieldValue, _locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        match value {
            FieldValue::Text(t) if t.chars().count() < self.0 => {
                Err(ValidationError::localized(msg::MinLength { min: self.0.into() }))
            }
            FieldValue::Choices(c) if c.len() < self.0 => {
                Err(ValidationError::localized(msg::MinPicks { min: self.0.into() }))
            }
            _ => Ok(()),
        }
    }
}

/// Fails above the given number of characters, or of picked options of a multi-select.
#[derive(Clone, Debug)]
pub struct MaxLength(pub usize);

impl Validator for MaxLength {
    fn validate(&self, value: &FieldValue, _locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        match value {
            FieldValue::Text(t) if t.chars().count() > self.0 => {
                Err(ValidationError::localized(msg::MaxLength { max: self.0.into() }))
            }
            FieldValue::Choices(c) if c.len() > self.0 => {
                Err(ValidationError::localized(msg::MaxPicks { max: self.0.into() }))
            }
            _ => Ok(()),
        }
    }
}

/// Fails on text that is not a locale number and on numbers outside the optional bounds.
#[derive(Clone, Debug, PartialEq)]
pub struct NumberRange {
    /// Inclusive lower bound; `None` is open.
    pub min: Option<f64>,
    /// Inclusive upper bound; `None` is open.
    pub max: Option<f64>,
}

impl Validator for NumberRange {
    fn validate(&self, value: &FieldValue, locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        let Some(text) = value.as_text() else {
            return Ok(());
        };
        let Some(n) = locale.parse_number(text) else {
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
        assert!(MinLength(2).validate(&"ab".into(), &en).is_ok());
        assert!(MaxLength(1).validate(&"ab".into(), &en).is_err());
        let picks = FieldValue::Choices(vec!["a".into(), "b".into()]);
        assert!(MaxLength(1).validate(&picks, &en).is_err(), "counts picks");
        let r = NumberRange {
            min: Some(1.0),
            max: None,
        };
        assert_eq!(r.validate(&"0.5".into(), &en).unwrap_err().code, "mineral-range-min");
        assert!(r.validate(&true.into(), &en).is_ok(), "text only");
    }
}

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

/// Fails on text that is not a locale number and on numbers outside the optional bounds.
#[derive(Clone, Debug, PartialEq)]
pub struct NumberRange {
    /// Inclusive lower bound; `None` is open.
    pub min: Option<f64>,
    /// Inclusive upper bound; `None` is open.
    pub max: Option<f64>,
}

impl Validator<String> for NumberRange {
    fn validate(&self, value: &String, locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
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
        assert!(MinLength(2).validate(&"ab".to_owned(), &en).is_ok());
        assert!(MaxLength(1).validate(&"ab".to_owned(), &en).is_err());
        let picks: Selection<u8> = [1, 2].into_iter().collect();
        assert!(MaxLength(1).validate(&picks, &en).is_err(), "counts picks");
        let r = NumberRange {
            min: Some(1.0),
            max: None,
        };
        assert_eq!(
            r.validate(&"0.5".to_owned(), &en).unwrap_err().code,
            "mineral-range-min"
        );
        assert!(r.validate(&"x".to_owned(), &en).is_err(), "not a number");
        assert!(Required.validate(&false, &en).is_err());
        assert!(Required.validate(&Some(3), &en).is_ok());
    }
}

use std::borrow::Cow;

use crate::{
    date::Date,
    i18n::{I18n, IntoMessage, Message},
    range::RangeModel,
    select::Selection,
};

/// Text of a label or help line: literal, or a catalog message resolved at display time.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Label {
    /// Shown as is.
    Text(Cow<'static, str>),
    /// Message id of an app catalog.
    Message(&'static str),
    /// Fluent attribute of a message: `(message id, attribute)`; optional lines are hidden while it is missing.
    Attribute(&'static str, &'static str),
}

impl Label {
    /// Attribute `attribute` of `message`: `Label::attribute(msg::SIGNUP_EMAIL, "supporting")`.
    pub fn attribute(message: impl IntoMessage<'static, 0>, attribute: &'static str) -> Self {
        Self::Attribute(message.into_message().id(), attribute)
    }

    /// Text in the active locale; a missing message shows its id.
    pub fn text(&self, i18n: &I18n) -> String {
        self.optional(i18n).unwrap_or_else(|| match self {
            Self::Text(t) => t.to_string(),
            Self::Message(id) | Self::Attribute(id, _) => (*id).to_owned(),
        })
    }

    /// Text in the active locale; `None` for a missing [`Self::Attribute`].
    pub fn optional(&self, i18n: &I18n) -> Option<String> {
        match self {
            Self::Text(t) => Some(t.to_string()),
            Self::Message(id) => Some(i18n.lookup(id, &[])),
            Self::Attribute(id, a) => i18n.attribute(crate::i18n::__private::message(id, []), a),
        }
    }
}

impl From<&'static str> for Label {
    fn from(text: &'static str) -> Self {
        Self::Text(Cow::Borrowed(text))
    }
}

impl From<String> for Label {
    fn from(text: String) -> Self {
        Self::Text(Cow::Owned(text))
    }
}

impl From<Message<'static, 0>> for Label {
    fn from(message: Message<'static, 0>) -> Self {
        Self::Message(message.id())
    }
}

/// Value type of a form field: its default widget kind, and blankness for
/// [`FieldCheck::required`](super::FieldCheck::required) and optional fields.
pub trait FieldInput {
    /// Widget kind of a field of this type unless `#[form(kind = ..)]` picks another.
    type Kind: FieldKind<Self>;

    /// Blank text, unchecked, nothing picked; optional fields skip their checks while blank.
    fn is_blank(&self) -> bool {
        false
    }
}

/// Widget family a field of value `V` renders as; stylekits implement [`FieldWidget`](super::FieldWidget) per kind
/// and value. Apps add kinds with their own marker types.
pub trait FieldKind<V: ?Sized> {
    /// Presentation options of the kind, set on [`FieldSpec`](super::FieldSpec).
    type Options: Default;
}

/// Widget kinds of the core value types.
pub mod kind {
    /// Text input.
    #[derive(Debug)]
    pub enum Text {}
    /// Checkbox.
    #[derive(Debug)]
    pub enum Check {}
    /// On/off switch.
    #[derive(Debug)]
    pub enum Switch {}
    /// Pick one of [`Choices`](super::Choices).
    #[derive(Debug)]
    pub enum Pick {}
    /// Pick any of [`Choices`](super::Choices).
    #[derive(Debug)]
    pub enum MultiPick {}
    /// Date input.
    #[derive(Debug)]
    pub enum Date {}
    /// Number input with steps.
    #[derive(Debug)]
    pub enum Number {}
    /// Slider over a range.
    #[derive(Debug)]
    pub enum Slider {}
}

/// [`FieldInput`] that can be blank, so it can be required. Numbers and dates always hold a value; a field that may
/// be empty binds `Option<V>`.
pub trait Blank: FieldInput {}

/// Options of a text field.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct TextOptions {
    /// Masked input (passwords).
    pub masked: bool,
    /// Visible lines; more than one makes Enter insert a newline instead of submitting.
    pub rows: usize,
    /// Shown while empty.
    pub placeholder: Option<Label>,
}

impl Default for TextOptions {
    fn default() -> Self {
        Self {
            masked: false,
            rows: 1,
            placeholder: None,
        }
    }
}

/// `(value, label)` options of a pick field.
#[derive(Clone, Debug, PartialEq)]
pub struct Choices<T>(pub Vec<(T, Label)>);

impl<T> Default for Choices<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T> Choices<T> {
    /// Options in display order.
    pub fn items(&self) -> &[(T, Label)] {
        &self.0
    }
}

/// Options of a number field.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct NumberOptions {
    /// Bounds and step; `None` is the stylekit's default.
    pub range: Option<RangeModel>,
}

impl FieldKind<String> for kind::Text {
    type Options = TextOptions;
}

impl FieldInput for String {
    type Kind = kind::Text;

    fn is_blank(&self) -> bool {
        self.trim().is_empty()
    }
}

impl Blank for String {}

impl FieldKind<bool> for kind::Check {
    type Options = ();
}

impl FieldKind<bool> for kind::Switch {
    type Options = ();
}

impl FieldInput for bool {
    type Kind = kind::Check;

    fn is_blank(&self) -> bool {
        !self
    }
}

impl Blank for bool {}

impl<T> FieldKind<Option<T>> for kind::Pick {
    type Options = Choices<T>;
}

impl FieldKind<Option<Date>> for kind::Date {
    type Options = ();
}

impl<T> FieldInput for Option<T> {
    type Kind = kind::Pick;

    fn is_blank(&self) -> bool {
        self.is_none()
    }
}

impl<T> Blank for Option<T> {}

impl<K: Ord + Clone> FieldKind<Selection<K>> for kind::MultiPick {
    type Options = Choices<K>;
}

impl<K: Ord + Clone> FieldInput for Selection<K> {
    type Kind = kind::MultiPick;

    fn is_blank(&self) -> bool {
        self.is_empty()
    }
}

impl<K: Ord + Clone> Blank for Selection<K> {}

impl FieldKind<Date> for kind::Date {
    type Options = ();
}

impl FieldInput for Date {
    type Kind = kind::Date;
}

macro_rules! number_input {
    ($($t:ty),*) => {$(
        impl FieldKind<$t> for kind::Number {
            type Options = NumberOptions;
        }

        impl FieldKind<$t> for kind::Slider {
            type Options = NumberOptions;
        }

        impl FieldInput for $t {
            type Kind = kind::Number;
        }
    )*};
}

number_input!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64);

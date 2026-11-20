//! Message lookup shortcuts for stylekits.

use crate::i18n::{I18n, IntoMessage};

/// Text of `message`.
pub fn tr<'a, const N: usize>(i18n: &I18n, message: impl IntoMessage<'a, N>) -> String {
    i18n.tr(message)
}

/// `over` when set, else the text of `message`.
pub fn tr_or<'a, const N: usize>(i18n: &I18n, over: Option<&str>, message: impl IntoMessage<'a, N>) -> String {
    over.map_or_else(|| i18n.tr(message), str::to_owned)
}

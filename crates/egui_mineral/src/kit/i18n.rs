//! Message lookup shortcuts for stylekits.

use crate::i18n::I18n;

/// Message `id` without arguments.
pub fn tr(i18n: &I18n, id: &str) -> String {
    i18n.tr(id, &[])
}

/// `over` when set, else message `id`.
pub fn tr_or(i18n: &I18n, over: Option<&str>, id: &str) -> String {
    over.map_or_else(|| tr(i18n, id), str::to_owned)
}

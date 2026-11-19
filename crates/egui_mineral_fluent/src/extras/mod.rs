//! Components beyond Fluent UI React v9, built from Fluent tokens ([`Origin::Other`]).
#![cfg_attr(
    feature = "unstable-catalog",
    doc = "",
    doc = "[`Origin::Other`]: crate::catalog::Origin::Other"
)]
#![cfg_attr(
    not(feature = "unstable-catalog"),
    doc = "",
    doc = "[`Origin::Other`]: https://docs.rs/egui_mineral_fluent/latest/egui_mineral_fluent/catalog/enum.Origin.html#variant.Other"
)]

pub(crate) mod pagination;
pub(crate) mod split_pane;
pub(crate) mod status_bar;

pub use pagination::Pagination;
pub use split_pane::SplitPane;
pub use status_bar::{StatusBar, StatusBarResponse, StatusItem, StatusProgress, StatusTone};

//! Actions: buttons, FABs, icon buttons, segmented buttons, split buttons ([m3](https://m3.material.io/components)).

pub(crate) mod button;
pub(crate) mod button_group;
pub(crate) mod fab;
pub(crate) mod fab_menu;
pub(crate) mod icon_button;
pub(crate) mod segmented_button;
pub(crate) mod split_button;

pub use button::{ButtonIcon, ButtonShape, ButtonVariant, ContainerVariant, PushButton, ToggleButton, text_actions};
pub use button_group::{ButtonGroup, ButtonGroupItem, ButtonGroupResponse, ButtonGroupVariant};
pub use fab::{ExtendedFab, ExtendedFabSize, Fab, FabColors, FabElevation, FabSize};
pub use fab_menu::{FabMenu, FabMenuColors, FabMenuItem, FabMenuResponse, FabMenuSize};
pub use icon_button::{IconButton, IconButtonVariant, IconButtonWidth};
pub use segmented_button::{Segment, SegmentedButton};
pub use split_button::{SplitButton, SplitButtonResponse};

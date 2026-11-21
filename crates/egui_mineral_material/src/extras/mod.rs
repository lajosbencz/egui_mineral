//! Components without an MD3 counterpart, built from MD3 tokens ([`Origin::Other`]).
#![cfg_attr(
    feature = "unstable-catalog",
    doc = "",
    doc = "[`Origin::Other`]: crate::catalog::Origin::Other"
)]
#![cfg_attr(
    not(feature = "unstable-catalog"),
    doc = "",
    doc = "[`Origin::Other`]: https://docs.rs/egui_mineral_material/latest/egui_mineral_material/catalog/enum.Origin.html#variant.Other"
)]

pub(crate) mod accordion;
pub(crate) mod alert;
pub(crate) mod avatar;
pub(crate) mod color_picker;
pub(crate) mod combobox;
pub(crate) mod command_palette;
pub(crate) mod data_display;
pub(crate) mod data_table;
pub(crate) mod date_field;
pub(crate) mod file_drop;
pub(crate) mod floating_panel;
pub(crate) mod form_grid;
pub(crate) mod image_list;
pub(crate) mod inline;
pub(crate) mod inline_edit;
pub(crate) mod menu_strip;
pub(crate) mod number_field;
pub(crate) mod otp;
pub(crate) mod pagination;
pub(crate) mod popover;
pub(crate) mod rating;
pub(crate) mod reorderable_list;
pub(crate) mod setting_row;
pub(crate) mod skeleton;
pub(crate) mod split_pane;
pub(crate) mod status_bar;
pub(crate) mod stepper;
pub(crate) mod tags;
pub(crate) mod tour;
pub(crate) mod transfer;
pub(crate) mod tree_view;

pub use accordion::{
    Accordion, AccordionPanel, AccordionPanelResponse, AccordionResponse, AccordionUi, AccordionVariant,
};
pub use alert::{Alert, AlertResponse, AlertVariant, Severity};
pub use avatar::{Avatar, AvatarGroup, AvatarSize};
pub use color_picker::{ColorPicker, ColorSpace};
pub use combobox::{Combobox, ComboboxOutput};
pub use command_palette::{CommandPalette, CommandPaletteResponse, PaletteCommand};
pub use data_display::{
    DescriptionItem, DescriptionLayout, DescriptionList, EmptyPlaceholder, EmptyPlaceholderResponse, Statistic,
    Timeline, TimelineItem, Trend,
};
pub use data_table::{DataTable, DataTableResponse, TableCell, TableColumn};
pub use date_field::{DateField, DateFieldState};
pub use file_drop::{FileDropZone, FileDropZoneOutput};
pub use floating_panel::FloatingPanel;
pub use form_grid::{FormGrid, FormGridUi};
pub use image_list::{ImageList, ImageListLayout, ImageListResponse, ImageTile};
pub use inline::{Kbd, TextLink};
pub use inline_edit::{InlineEdit, InlineEditOutput};
pub use menu_strip::{MenuStrip, MenuStripResponse};
pub use number_field::{NumberField, NumberFieldOutput};
pub use otp::{OtpField, OtpFieldOutput};
pub use pagination::{BreadcrumbItem, Breadcrumbs, BreadcrumbsResponse, Pagination};
pub use popover::{HoverCard, Popover};
pub use rating::Rating;
pub use reorderable_list::{ReorderableList, ReorderableListResponse};
pub use setting_row::{SettingRow, SettingRowResponse};
pub use skeleton::{Skeleton, SkeletonShape};
pub use split_pane::SplitPane;
pub use status_bar::{StatusBar, StatusBarResponse, StatusItem, StatusProgress};
pub use stepper::{StepLabel, Stepper};
pub use tags::{TagsField, TagsFieldOutput};
pub use tour::{Tour, TourResponse, TourStep};
pub use transfer::TransferList;
pub use tree_view::{TreeAction, TreeDrag, TreeView, TreeViewResponse};

//! Data display: text, avatars, badges, tables, lists, trees (Fluent UI React v9 `react-text`, `react-table`, ...).

mod avatar;
mod avatar_group;
mod image;
mod list;
mod persona;
mod table;
mod tag;
mod text;

pub use avatar::{
    Avatar, AvatarActive, AvatarActiveAppearance, AvatarColor, AvatarNamedColor, AvatarShape, AvatarSize,
};
pub use avatar_group::{AvatarGroup, AvatarGroupLayout, AvatarGroupResponse};
pub use image::{Image, ImageFit, ImageShape};
pub use list::{List, ListItem, ListItemResponse, ListNavigation, ListResponse, ListUi};
pub use persona::{Persona, PersonaSize, PersonaTextAlignment, PersonaTextPosition};
pub use table::{DataGrid, Table, TableCellLayout, TableColumn, TableResponse, TableRowAppearance, TableSize};
pub use tag::{
    InteractionTag, InteractionTagResponse, Tag, TagAppearance, TagGroup, TagGroupResponse, TagShape, TagSize,
};
pub use text::{Text, TextAlign, TextFont, TextMetrics, TextSize, TextWeight};

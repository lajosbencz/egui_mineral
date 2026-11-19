use std::sync::Arc;

use egui_material_icons::{MaterialIcon, icons};
use egui_mineral::icon::Icon;

use crate::fonts::icon_of;

/// Glyphs the kit draws for its own parts (chevrons, close, check, ...), one field per role named after
/// the Material Symbols icon it defaults to. [`Self::material`] is the default, other fonts fill the same fields
/// ([`Material::with_icons`](crate::Material::with_icons)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IconSet {
    /// `add`.
    pub add: Icon,
    /// `arrow_back`.
    pub arrow_back: Icon,
    /// `arrow_downward`.
    pub arrow_downward: Icon,
    /// `arrow_drop_down`.
    pub arrow_drop_down: Icon,
    /// `arrow_drop_up`.
    pub arrow_drop_up: Icon,
    /// `arrow_forward`.
    pub arrow_forward: Icon,
    /// `arrow_left`.
    pub arrow_left: Icon,
    /// `arrow_right`.
    pub arrow_right: Icon,
    /// `arrow_upward`.
    pub arrow_upward: Icon,
    /// `attach_file`.
    pub attach_file: Icon,
    /// `calendar_month`.
    pub calendar_month: Icon,
    /// `check`.
    pub check: Icon,
    /// `check_circle`.
    pub check_circle: Icon,
    /// `chevron_left`.
    pub chevron_left: Icon,
    /// `chevron_right`.
    pub chevron_right: Icon,
    /// `close`.
    pub close: Icon,
    /// `cloud_upload`.
    pub cloud_upload: Icon,
    /// `date_range`.
    pub date_range: Icon,
    /// `drag_indicator`.
    pub drag_indicator: Icon,
    /// `edit`.
    pub edit: Icon,
    /// `error`.
    pub error: Icon,
    /// `expand_more`.
    pub expand_more: Icon,
    /// `first_page`.
    pub first_page: Icon,
    /// `inbox`.
    pub inbox: Icon,
    /// `info`.
    pub info: Icon,
    /// `keyboard`.
    pub keyboard: Icon,
    /// `keyboard_arrow_down`.
    pub keyboard_arrow_down: Icon,
    /// `keyboard_arrow_left`.
    pub keyboard_arrow_left: Icon,
    /// `keyboard_arrow_right`.
    pub keyboard_arrow_right: Icon,
    /// `keyboard_double_arrow_left`.
    pub keyboard_double_arrow_left: Icon,
    /// `keyboard_double_arrow_right`.
    pub keyboard_double_arrow_right: Icon,
    /// `last_page`.
    pub last_page: Icon,
    /// `menu`.
    pub menu: Icon,
    /// `menu_open`.
    pub menu_open: Icon,
    /// `more_horiz`.
    pub more_horiz: Icon,
    /// `more_vert`.
    pub more_vert: Icon,
    /// `person`.
    pub person: Icon,
    /// `priority_high`.
    pub priority_high: Icon,
    /// `progress_activity`.
    pub progress_activity: Icon,
    /// `remove`.
    pub remove: Icon,
    /// `schedule`.
    pub schedule: Icon,
    /// `search`.
    pub search: Icon,
    /// `search_off`.
    pub search_off: Icon,
    /// `star`.
    pub star: Icon,
    /// `trending_down`.
    pub trending_down: Icon,
    /// `trending_flat`.
    pub trending_flat: Icon,
    /// `trending_up`.
    pub trending_up: Icon,
    /// `upload_file`.
    pub upload_file: Icon,
    /// `visibility`.
    pub visibility: Icon,
    /// `visibility_off`.
    pub visibility_off: Icon,
    /// `warning`.
    pub warning: Icon,
}

impl IconSet {
    /// Material Symbols Rounded, the font of [`Material::new`](crate::Material::new).
    pub fn material() -> Self {
        let m = |i: MaterialIcon| icon_of(i);
        Self {
            add: m(icons::ICON_ADD),
            arrow_back: m(icons::ICON_ARROW_BACK),
            arrow_downward: m(icons::ICON_ARROW_DOWNWARD),
            arrow_drop_down: m(icons::ICON_ARROW_DROP_DOWN),
            arrow_drop_up: m(icons::ICON_ARROW_DROP_UP),
            arrow_forward: m(icons::ICON_ARROW_FORWARD),
            arrow_left: m(icons::ICON_ARROW_LEFT),
            arrow_right: m(icons::ICON_ARROW_RIGHT),
            arrow_upward: m(icons::ICON_ARROW_UPWARD),
            attach_file: m(icons::ICON_ATTACH_FILE),
            calendar_month: m(icons::ICON_CALENDAR_MONTH),
            check: m(icons::ICON_CHECK),
            check_circle: m(icons::ICON_CHECK_CIRCLE),
            chevron_left: m(icons::ICON_CHEVRON_LEFT),
            chevron_right: m(icons::ICON_CHEVRON_RIGHT),
            close: m(icons::ICON_CLOSE),
            cloud_upload: m(icons::ICON_CLOUD_UPLOAD),
            date_range: m(icons::ICON_DATE_RANGE),
            drag_indicator: m(icons::ICON_DRAG_INDICATOR),
            edit: m(icons::ICON_EDIT),
            error: m(icons::ICON_ERROR),
            expand_more: m(icons::ICON_EXPAND_MORE),
            first_page: m(icons::ICON_FIRST_PAGE),
            inbox: m(icons::ICON_INBOX),
            info: m(icons::ICON_INFO),
            keyboard: m(icons::ICON_KEYBOARD),
            keyboard_arrow_down: m(icons::ICON_KEYBOARD_ARROW_DOWN),
            keyboard_arrow_left: m(icons::ICON_KEYBOARD_ARROW_LEFT),
            keyboard_arrow_right: m(icons::ICON_KEYBOARD_ARROW_RIGHT),
            keyboard_double_arrow_left: m(icons::ICON_KEYBOARD_DOUBLE_ARROW_LEFT),
            keyboard_double_arrow_right: m(icons::ICON_KEYBOARD_DOUBLE_ARROW_RIGHT),
            last_page: m(icons::ICON_LAST_PAGE),
            menu: m(icons::ICON_MENU),
            menu_open: m(icons::ICON_MENU_OPEN),
            more_horiz: m(icons::ICON_MORE_HORIZ),
            more_vert: m(icons::ICON_MORE_VERT),
            person: m(icons::ICON_PERSON),
            priority_high: m(icons::ICON_PRIORITY_HIGH),
            progress_activity: m(icons::ICON_PROGRESS_ACTIVITY),
            remove: m(icons::ICON_REMOVE),
            schedule: m(icons::ICON_SCHEDULE),
            search: m(icons::ICON_SEARCH),
            search_off: m(icons::ICON_SEARCH_OFF),
            star: m(icons::ICON_STAR),
            trending_down: m(icons::ICON_TRENDING_DOWN),
            trending_flat: m(icons::ICON_TRENDING_FLAT),
            trending_up: m(icons::ICON_TRENDING_UP),
            upload_file: m(icons::ICON_UPLOAD_FILE),
            visibility: m(icons::ICON_VISIBILITY),
            visibility_off: m(icons::ICON_VISIBILITY_OFF),
            warning: m(icons::ICON_WARNING),
        }
    }
}

impl Default for IconSet {
    fn default() -> Self {
        Self::material()
    }
}

pub(crate) type SharedIcons = Arc<IconSet>;

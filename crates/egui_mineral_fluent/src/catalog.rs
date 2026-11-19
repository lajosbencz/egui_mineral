/// Component group of the stylekit; each variant is the public module of the same name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    /// Buttons, links, toolbars.
    Actions,
    /// Fields, choices, sliders, labels.
    Inputs,
    /// Message bars, progress, spinners, skeletons, toasts, badges, tooltips.
    Feedback,
    /// Cards, dialogs, drawers, popovers, dividers.
    Surfaces,
    /// Breadcrumbs, nav, tabs, menus.
    Navigation,
    /// Text, avatars, tables, lists, trees.
    Data,
    /// Components beyond Fluent UI React v9 ([`Origin::Other`]); needs the `extras` feature.
    #[cfg(feature = "extras")]
    Extras,
}

impl Category {
    /// Every variant.
    #[cfg(feature = "extras")]
    pub const ALL: [Self; 7] = [
        Self::Actions,
        Self::Inputs,
        Self::Feedback,
        Self::Surfaces,
        Self::Navigation,
        Self::Data,
        Self::Extras,
    ];

    /// Every variant.
    #[cfg(not(feature = "extras"))]
    pub const ALL: [Self; 6] = [
        Self::Actions,
        Self::Inputs,
        Self::Feedback,
        Self::Surfaces,
        Self::Navigation,
        Self::Data,
    ];

    /// Display name in the context's locale.
    pub fn title(self, i18n: &egui_mineral::i18n::I18n) -> String {
        crate::tr(
            i18n,
            match self {
                Self::Actions => "mineral-category-actions",
                Self::Inputs => "mineral-category-inputs",
                Self::Feedback => "mineral-category-feedback",
                Self::Surfaces => "mineral-category-surfaces",
                Self::Navigation => "mineral-category-navigation",
                Self::Data => "mineral-category-data",
                #[cfg(feature = "extras")]
                Self::Extras => "mineral-category-extras",
            },
        )
    }
}

/// Provenance record of one component, listed in [`CATALOG`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ComponentMeta {
    /// Rust type name.
    pub name: &'static str,
    /// Category module that exports the component.
    pub category: Category,
    /// What it ports.
    pub origin: Origin,
}

/// Provenance of a component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Origin {
    /// Ports a Fluent UI React v9 component.
    ReactV9 {
        /// Component it ports.
        react: &'static str,
        /// Its styles file under `packages/react-components/` at
        /// [`SOURCE_COMMIT`](crate::spec::tokens::SOURCE_COMMIT), vendored in `tools/fluent_tokens/vendor`.
        styles: &'static str,
    },
    /// Not in Fluent UI React v9; patterned after a design system or desktop convention.
    Other {
        /// Pattern URL or name.
        pattern: &'static str,
    },
}

/// Provenance of a stylekit component.
pub trait Component {
    /// Provenance, also listed in [`CATALOG`].
    const META: ComponentMeta;
}

macro_rules! meta {
    ($ty:ident $(<$lt:lifetime>)?, $category:ident, $react:literal, $styles:literal) => {
        impl $(<$lt>)? Component for $ty $(<$lt>)? {
            const META: ComponentMeta = ComponentMeta {
                name: stringify!($ty),
                category: Category::$category,
                origin: Origin::ReactV9 {
                    react: $react,
                    styles: $styles,
                },
            };
        }
    };
}

#[cfg(feature = "extras")]
macro_rules! other {
    ($ty:ident $(<$lt:lifetime>)?, $pattern:literal) => {
        impl $(<$lt>)? Component for $ty $(<$lt>)? {
            const META: ComponentMeta = ComponentMeta {
                name: stringify!($ty),
                category: Category::Extras,
                origin: Origin::Other { pattern: $pattern },
            };
        }
    };
}

// Only the prelude: a cataloged type that it does not export fails to compile here.
use crate::prelude::*;

meta!(
    PushButton<'a>,
    Actions,
    "Button",
    "react-button/library/src/components/Button/useButtonStyles.styles.ts"
);
meta!(
    ToggleButton<'a>,
    Actions,
    "ToggleButton",
    "react-button/library/src/components/ToggleButton/useToggleButtonStyles.styles.ts"
);
meta!(
    CompoundButton,
    Actions,
    "CompoundButton",
    "react-button/library/src/components/CompoundButton/useCompoundButtonStyles.styles.ts"
);
meta!(
    MenuButton,
    Actions,
    "MenuButton",
    "react-button/library/src/components/MenuButton/useMenuButtonStyles.styles.ts"
);
meta!(
    SplitButton,
    Actions,
    "SplitButton",
    "react-button/library/src/components/SplitButton/useSplitButtonStyles.styles.ts"
);
meta!(
    TextLink,
    Actions,
    "Link",
    "react-link/library/src/components/Link/useLinkStyles.styles.ts"
);
meta!(
    FieldLabel,
    Inputs,
    "Label",
    "react-label/library/src/components/Label/useLabelStyles.styles.ts"
);
meta!(
    Checkbox<'a>,
    Inputs,
    "Checkbox",
    "react-checkbox/library/src/components/Checkbox/useCheckboxStyles.styles.ts"
);
meta!(
    Radio,
    Inputs,
    "Radio",
    "react-radio/library/src/components/Radio/useRadioStyles.styles.ts"
);
meta!(
    Switch<'a>,
    Inputs,
    "Switch",
    "react-switch/library/src/components/Switch/useSwitchStyles.styles.ts"
);
meta!(
    Slider<'a>,
    Inputs,
    "Slider",
    "react-slider/library/src/components/Slider/useSliderStyles.styles.ts"
);
meta!(
    RangeSlider<'a>,
    Inputs,
    "RangeSlider",
    "react-range-slider-preview/library/src/components/RangeSlider/useRangeSliderStyles.styles.ts"
);
meta!(
    SpinButton<'a>,
    Inputs,
    "SpinButton",
    "react-spinbutton/library/src/components/SpinButton/useSpinButtonStyles.styles.ts"
);
meta!(
    Rating<'a>,
    Inputs,
    "Rating",
    "react-rating/library/src/components/Rating/useRatingStyles.styles.ts"
);
meta!(
    RatingDisplay,
    Inputs,
    "RatingDisplay",
    "react-rating/library/src/components/RatingDisplay/useRatingDisplayStyles.styles.ts"
);
meta!(
    Field,
    Inputs,
    "Field",
    "react-field/library/src/components/Field/useFieldStyles.styles.ts"
);
meta!(
    InfoLabel,
    Inputs,
    "InfoLabel",
    "react-infolabel/library/src/components/InfoLabel/useInfoLabelStyles.styles.ts"
);
meta!(
    Input<'a>,
    Inputs,
    "Input",
    "react-input/library/src/components/Input/useInputStyles.styles.ts"
);
meta!(
    Textarea<'a>,
    Inputs,
    "Textarea",
    "react-textarea/library/src/components/Textarea/useTextareaStyles.styles.ts"
);
meta!(
    SearchBox<'a>,
    Inputs,
    "SearchBox",
    "react-search/library/src/components/SearchBox/useSearchBoxStyles.styles.ts"
);

meta!(
    Dropdown<'a>,
    Inputs,
    "Dropdown",
    "react-combobox/library/src/components/Dropdown/useDropdownStyles.styles.ts"
);
meta!(
    Combobox<'a>,
    Inputs,
    "Combobox",
    "react-combobox/library/src/components/Combobox/useComboboxStyles.styles.ts"
);
meta!(
    Select<'a>,
    Inputs,
    "Select",
    "react-select/library/src/components/Select/useSelectStyles.styles.ts"
);
meta!(
    TimePicker<'a>,
    Inputs,
    "TimePicker",
    "react-timepicker-compat/library/src/components/TimePicker/useTimePickerStyles.styles.ts"
);
meta!(
    Calendar<'a>,
    Inputs,
    "Calendar",
    "react-calendar-compat/library/src/components/Calendar/useCalendarStyles.styles.ts"
);
meta!(
    DatePicker<'a>,
    Inputs,
    "DatePicker",
    "react-datepicker-compat/library/src/components/DatePicker/useDatePickerStyles.styles.ts"
);
meta!(
    TagPicker<'a>,
    Inputs,
    "TagPicker",
    "react-tag-picker/library/src/components/TagPickerControl/useTagPickerControlStyles.styles.ts"
);
meta!(
    SwatchPicker<'a>,
    Inputs,
    "SwatchPicker",
    "react-swatch-picker/library/src/components/SwatchPicker/useSwatchPickerStyles.styles.ts"
);
meta!(
    Swatch,
    Inputs,
    "ColorSwatch",
    "react-swatch-picker/library/src/components/ColorSwatch/useColorSwatchStyles.styles.ts"
);
meta!(
    ColorPicker<'a>,
    Inputs,
    "ColorPicker",
    "react-color-picker/library/src/components/ColorPicker/useColorPickerStyles.styles.ts"
);
meta!(
    ListOption,
    Inputs,
    "Option",
    "react-combobox/library/src/components/Option/useOptionStyles.styles.ts"
);
meta!(
    OptionGroup,
    Inputs,
    "OptionGroup",
    "react-combobox/library/src/components/OptionGroup/useOptionGroupStyles.styles.ts"
);

impl<T> Component for RadioGroup<'_, T> {
    const META: ComponentMeta = ComponentMeta {
        name: "RadioGroup",
        category: Category::Inputs,
        origin: Origin::ReactV9 {
            react: "RadioGroup",
            styles: "react-radio/library/src/components/RadioGroup/useRadioGroupStyles.styles.ts",
        },
    };
}
meta!(
    Accordion<'a>,
    Surfaces,
    "Accordion",
    "react-accordion/library/src/components/AccordionHeader/useAccordionHeaderStyles.styles.ts"
);
meta!(
    Card<'a>,
    Surfaces,
    "Card",
    "react-card/library/src/components/Card/useCardStyles.styles.ts"
);
meta!(
    Divider,
    Surfaces,
    "Divider",
    "react-divider/library/src/components/Divider/useDividerStyles.styles.ts"
);
meta!(
    Tooltip,
    Surfaces,
    "Tooltip",
    "react-tooltip/library/src/components/Tooltip/useTooltipStyles.styles.ts"
);
meta!(
    Popover,
    Surfaces,
    "Popover",
    "react-popover/library/src/components/PopoverSurface/usePopoverSurfaceStyles.styles.ts"
);
meta!(
    TeachingPopover,
    Surfaces,
    "TeachingPopover",
    "react-teaching-popover/library/src/components/TeachingPopoverSurface/useTeachingPopoverSurfaceStyles.styles.ts"
);
meta!(
    Dialog,
    Surfaces,
    "Dialog",
    "react-dialog/library/src/components/DialogSurface/useDialogSurfaceStyles.styles.ts"
);
meta!(
    Drawer,
    Surfaces,
    "Drawer",
    "react-drawer/library/src/components/OverlayDrawer/useOverlayDrawerStyles.styles.ts"
);
meta!(
    Avatar<'a>,
    Data,
    "Avatar",
    "react-avatar/library/src/components/Avatar/useAvatarStyles.styles.ts"
);
meta!(
    AvatarGroup<'a>,
    Data,
    "AvatarGroup",
    "react-avatar/library/src/components/AvatarGroup/useAvatarGroupStyles.styles.ts"
);
meta!(
    Image<'a>,
    Data,
    "Image",
    "react-image/library/src/components/Image/useImageStyles.styles.ts"
);
meta!(
    List<'a>,
    Data,
    "List",
    "react-list/library/src/components/ListItem/useListItemStyles.styles.ts"
);
meta!(
    Persona<'a>,
    Data,
    "Persona",
    "react-persona/library/src/components/Persona/usePersonaStyles.styles.ts"
);
meta!(
    Tag<'a>,
    Data,
    "Tag",
    "react-tags/library/src/components/Tag/useTagStyles.styles.ts"
);
meta!(
    InteractionTag<'a>,
    Data,
    "InteractionTag",
    "react-tags/library/src/components/InteractionTag/useInteractionTagStyles.styles.ts"
);
meta!(
    TagGroup<'a>,
    Data,
    "TagGroup",
    "react-tags/library/src/components/TagGroup/useTagGroupStyles.styles.ts"
);
meta!(
    Table<'a>,
    Data,
    "Table",
    "react-table/library/src/components/Table/useTableStyles.styles.ts"
);
meta!(
    DataGrid<'a>,
    Data,
    "DataGrid",
    "react-table/library/src/components/DataGrid/useDataGridStyles.styles.ts"
);
meta!(
    TableColumn,
    Data,
    "TableHeaderCell",
    "react-table/library/src/components/TableHeaderCell/useTableHeaderCellStyles.styles.ts"
);
meta!(
    TableCellLayout<'a>,
    Data,
    "TableCellLayout",
    "react-table/library/src/components/TableCellLayout/useTableCellLayoutStyles.styles.ts"
);
meta!(
    Text,
    Data,
    "Text",
    "react-text/library/src/components/Text/useTextStyles.styles.ts"
);
meta!(
    ProgressBar,
    Feedback,
    "ProgressBar",
    "react-progress/library/src/components/ProgressBar/useProgressBarStyles.styles.ts"
);
meta!(
    Spinner,
    Feedback,
    "Spinner",
    "react-spinner/library/src/components/Spinner/useSpinnerStyles.styles.ts"
);
meta!(
    Skeleton,
    Feedback,
    "Skeleton",
    "react-skeleton/library/src/components/Skeleton/useSkeletonStyles.styles.ts"
);
meta!(
    SkeletonItem,
    Feedback,
    "SkeletonItem",
    "react-skeleton/library/src/components/SkeletonItem/useSkeletonItemStyles.styles.ts"
);
meta!(
    MessageBar,
    Feedback,
    "MessageBar",
    "react-message-bar/library/src/components/MessageBar/useMessageBarStyles.styles.ts"
);
meta!(
    MessageBarGroup,
    Feedback,
    "MessageBarGroup",
    "react-message-bar/library/src/components/MessageBarGroup/useMessageBarGroupStyles.styles.ts"
);
meta!(
    Toast,
    Feedback,
    "Toast",
    "react-toast/library/src/components/Toast/useToastStyles.styles.ts"
);
meta!(
    Toaster<'a>,
    Feedback,
    "Toaster",
    "react-toast/library/src/components/Toaster/useToasterStyles.styles.ts"
);
meta!(
    Badge,
    Feedback,
    "Badge",
    "react-badge/library/src/components/Badge/useBadgeStyles.styles.ts"
);
meta!(
    CounterBadge,
    Feedback,
    "CounterBadge",
    "react-badge/library/src/components/CounterBadge/useCounterBadgeStyles.styles.ts"
);
meta!(
    PresenceBadge,
    Feedback,
    "PresenceBadge",
    "react-badge/library/src/components/PresenceBadge/usePresenceBadgeStyles.styles.ts"
);
meta!(
    Tab,
    Navigation,
    "Tab",
    "react-tabs/library/src/components/Tab/useTabStyles.styles.ts"
);
meta!(
    TabList<'a>,
    Navigation,
    "TabList",
    "react-tabs/library/src/components/TabList/useTabListStyles.styles.ts"
);
meta!(
    Toolbar<'a>,
    Navigation,
    "Toolbar",
    "react-toolbar/library/src/components/Toolbar/useToolbarStyles.styles.ts"
);
meta!(
    ToolbarButton,
    Navigation,
    "ToolbarButton",
    "react-toolbar/library/src/components/ToolbarButton/useToolbarButtonStyles.styles.ts"
);
meta!(
    ToolbarToggleButton<'a>,
    Navigation,
    "ToolbarToggleButton",
    "react-toolbar/library/src/components/ToolbarToggleButton/useToolbarToggleButtonStyles.styles.ts"
);
meta!(
    ToolbarRadioButton,
    Navigation,
    "ToolbarRadioButton",
    "react-toolbar/library/src/components/ToolbarRadioButton/useToolbarRadioButtonStyles.styles.ts"
);
meta!(
    ToolbarGroup<'a>,
    Navigation,
    "ToolbarGroup",
    "react-toolbar/library/src/components/ToolbarGroup/useToolbarGroupStyles.styles.ts"
);
meta!(
    Breadcrumb,
    Navigation,
    "Breadcrumb",
    "react-breadcrumb/library/src/components/Breadcrumb/useBreadcrumbStyles.styles.ts"
);
meta!(
    BreadcrumbItem,
    Navigation,
    "BreadcrumbItem",
    "react-breadcrumb/library/src/components/BreadcrumbItem/useBreadcrumbItemStyles.styles.ts"
);
meta!(
    BreadcrumbButton,
    Navigation,
    "BreadcrumbButton",
    "react-breadcrumb/library/src/components/BreadcrumbButton/useBreadcrumbButtonStyles.styles.ts"
);
meta!(
    BreadcrumbDivider,
    Navigation,
    "BreadcrumbDivider",
    "react-breadcrumb/library/src/components/BreadcrumbDivider/useBreadcrumbDividerStyles.styles.ts"
);
meta!(
    Overflow,
    Navigation,
    "Overflow",
    "react-overflow/library/src/components/Overflow/useOverflowStyles.styles.ts"
);
meta!(
    Nav<'a>,
    Navigation,
    "Nav",
    "react-nav/library/src/components/Nav/useNavStyles.styles.ts"
);
meta!(
    NavDrawer<'a>,
    Navigation,
    "NavDrawer",
    "react-nav/library/src/components/NavDrawer/useNavDrawerStyles.styles.ts"
);
meta!(
    NavItem,
    Navigation,
    "NavItem",
    "react-nav/library/src/components/NavItem/useNavItemStyles.styles.ts"
);
meta!(
    NavCategory,
    Navigation,
    "NavCategory",
    "react-nav/library/src/components/NavCategoryItem/useNavCategoryItem.styles.ts"
);
meta!(
    NavSubItem,
    Navigation,
    "NavSubItem",
    "react-nav/library/src/components/NavSubItem/useNavSubItemStyles.styles.ts"
);
meta!(
    NavSectionHeader,
    Navigation,
    "NavSectionHeader",
    "react-nav/library/src/components/NavSectionHeader/useNavSectionHeaderStyles.styles.ts"
);
meta!(
    NavDivider,
    Navigation,
    "NavDivider",
    "react-nav/library/src/components/NavDivider/useNavDividerStyles.styles.ts"
);
meta!(
    Menu,
    Navigation,
    "Menu",
    "react-menu/library/src/components/MenuPopover/useMenuPopoverStyles.styles.ts"
);
meta!(
    MenuItem,
    Navigation,
    "MenuItem",
    "react-menu/library/src/components/MenuItem/useMenuItemStyles.styles.ts"
);
meta!(
    Carousel<'a>,
    Navigation,
    "Carousel",
    "react-carousel/library/src/components/Carousel/useCarouselStyles.styles.ts"
);
meta!(
    CarouselCard,
    Navigation,
    "CarouselCard",
    "react-carousel/library/src/components/CarouselCard/useCarouselCardStyles.styles.ts"
);
meta!(
    MenuGrid,
    Navigation,
    "MenuGrid",
    "react-menu-grid-preview/library/src/components/MenuGrid/useMenuGridStyles.styles.ts"
);
meta!(
    MenuGridRow,
    Navigation,
    "MenuGridRow",
    "react-menu-grid-preview/library/src/components/MenuGridRow/useMenuGridRowStyles.styles.ts"
);
meta!(
    MenuGridItem,
    Navigation,
    "MenuGridItem",
    "react-menu-grid-preview/library/src/components/MenuGridItem/useMenuGridItemStyles.styles.ts"
);
meta!(
    MenuGridAction,
    Navigation,
    "MenuGridCell",
    "react-menu-grid-preview/library/src/components/MenuGridCell/useMenuGridCellStyles.styles.ts"
);
meta!(
    TreeItem,
    Navigation,
    "TreeItem",
    "react-tree/library/src/components/TreeItem/useTreeItemStyles.styles.ts"
);
meta!(
    TreeItemLayout,
    Navigation,
    "TreeItemLayout",
    "react-tree/library/src/components/TreeItemLayout/useTreeItemLayoutStyles.styles.ts"
);
meta!(
    TreeItemPersonaLayout,
    Navigation,
    "TreeItemPersonaLayout",
    "react-tree/library/src/components/TreeItemPersonaLayout/useTreeItemPersonaLayoutStyles.styles.ts"
);

impl<N: egui_mineral::tree::TreeData> Component for Tree<'_, N> {
    const META: ComponentMeta = ComponentMeta {
        name: "Tree",
        category: Category::Navigation,
        origin: Origin::ReactV9 {
            react: "Tree",
            styles: "react-tree/library/src/components/Tree/useTreeStyles.styles.ts",
        },
    };
}

#[cfg(feature = "extras")]
mod extras {
    use super::{Category, Component, ComponentMeta, Origin};
    use crate::prelude::*;

    other!(SplitPane, "Desktop splitter: resizable panes");
    other!(StatusBar, "Desktop status bar");
    other!(Pagination<'a>, "Page selector with siblings, boundaries and gaps");
}

/// Every public component with its category and origin.
pub const CATALOG: &[ComponentMeta] = &[
    <PushButton<'static>>::META,
    <ToggleButton<'static>>::META,
    CompoundButton::META,
    MenuButton::META,
    SplitButton::META,
    TextLink::META,
    FieldLabel::META,
    <Checkbox<'static>>::META,
    Radio::META,
    <Switch<'static>>::META,
    <Slider<'static>>::META,
    <RangeSlider<'static>>::META,
    <SpinButton<'static>>::META,
    <Rating<'static>>::META,
    RatingDisplay::META,
    Field::META,
    InfoLabel::META,
    <Input<'static>>::META,
    <Textarea<'static>>::META,
    <SearchBox<'static>>::META,
    <RadioGroup<'static, ()>>::META,
    <Dropdown<'static>>::META,
    <Combobox<'static>>::META,
    <Select<'static>>::META,
    <TimePicker<'static>>::META,
    <TagPicker<'static>>::META,
    <Calendar<'static>>::META,
    <DatePicker<'static>>::META,
    <SwatchPicker<'static>>::META,
    Swatch::META,
    <ColorPicker<'static>>::META,
    ListOption::META,
    OptionGroup::META,
    Accordion::META,
    Card::META,
    Divider::META,
    Tooltip::META,
    Popover::META,
    TeachingPopover::META,
    Dialog::META,
    Drawer::META,
    <Avatar<'static>>::META,
    <AvatarGroup<'static>>::META,
    Image::META,
    List::META,
    <Persona<'static>>::META,
    <Tag<'static>>::META,
    <InteractionTag<'static>>::META,
    <TagGroup<'static>>::META,
    <Table<'static>>::META,
    <DataGrid<'static>>::META,
    TableColumn::META,
    <TableCellLayout<'static>>::META,
    Text::META,
    ProgressBar::META,
    Spinner::META,
    Skeleton::META,
    SkeletonItem::META,
    MessageBar::META,
    MessageBarGroup::META,
    Toast::META,
    <Toaster<'static>>::META,
    Badge::META,
    CounterBadge::META,
    PresenceBadge::META,
    Tab::META,
    <TabList<'static>>::META,
    <Toolbar<'static>>::META,
    ToolbarButton::META,
    <ToolbarToggleButton<'static>>::META,
    ToolbarRadioButton::META,
    <ToolbarGroup<'static>>::META,
    Breadcrumb::META,
    BreadcrumbItem::META,
    BreadcrumbButton::META,
    BreadcrumbDivider::META,
    Overflow::META,
    <Nav<'static>>::META,
    <NavDrawer<'static>>::META,
    NavItem::META,
    NavCategory::META,
    NavSubItem::META,
    NavSectionHeader::META,
    NavDivider::META,
    Menu::META,
    MenuItem::META,
    // `MenuSplitGroup` is the `MenuUi::split_item` row, not a type.
    ComponentMeta {
        name: "MenuUi::split_item",
        category: Category::Navigation,
        origin: Origin::ReactV9 {
            react: "MenuSplitGroup",
            styles: "react-menu/library/src/components/MenuSplitGroup/useMenuSplitGroupStyles.styles.ts",
        },
    },
    <Carousel<'static>>::META,
    CarouselCard::META,
    MenuGrid::META,
    MenuGridRow::META,
    MenuGridItem::META,
    MenuGridAction::META,
    <Tree<'static, TreeItem>>::META,
    TreeItem::META,
    TreeItemLayout::META,
    TreeItemPersonaLayout::META,
    #[cfg(feature = "extras")]
    SplitPane::META,
    #[cfg(feature = "extras")]
    StatusBar::META,
    #[cfg(feature = "extras")]
    <Pagination<'static>>::META,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_styles_file_is_vendored() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/fluent_tokens/vendor/",);
        let vendored = std::fs::read_to_string(format!("{root}{}/STYLES.txt", crate::spec::tokens::SOURCE_COMMIT))
            .expect("STYLES.txt");
        for c in CATALOG {
            let Origin::ReactV9 { styles, .. } = c.origin else {
                continue;
            };
            assert!(
                vendored
                    .lines()
                    .any(|l| l == format!("packages/react-components/{styles}")),
                "{}: {styles} not vendored",
                c.name
            );
        }
    }
}

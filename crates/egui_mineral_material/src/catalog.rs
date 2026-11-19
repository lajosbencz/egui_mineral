/// Non-MD3 design system a component's pattern is taken from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Kit {
    /// [MUI](https://mui.com).
    Mui,
    /// [Flutter](https://flutter.dev) widgets.
    Flutter,
    /// Microsoft [Fluent](https://fluent2.microsoft.design).
    Fluent,
    /// [Ant Design](https://ant.design).
    Ant,
    /// [Radix](https://www.radix-ui.com) primitives.
    Radix,
    /// [shadcn/ui](https://ui.shadcn.com).
    Shadcn,
    /// [Chakra UI](https://chakra-ui.com).
    Chakra,
    /// Apple [Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines).
    AppleHig,
    /// [Bootstrap](https://getbootstrap.com).
    Bootstrap,
}

/// Spec provenance of a component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Origin {
    /// Baseline MD3 component.
    Md3,
    /// Component added or reworked by M3 Expressive.
    Md3Expressive,
    /// Not in MD3; patterned after these kits.
    Other(&'static [Kit]),
}

/// Component group of [m3.material.io](https://m3.material.io/components); `Extras` holds the
/// [`Origin::Other`] components. Each variant is the public module of the same name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Category {
    /// Buttons, FABs, icon, segmented and split buttons.
    Actions,
    /// Badges, progress, snackbars, tooltips.
    Communication,
    /// Cards, dialogs, sheets, lists, dividers.
    Containment,
    /// Bars, rails, drawers, tabs, search.
    Navigation,
    /// Checkboxes, chips, menus, pickers, sliders.
    Selection,
    /// Text fields.
    TextInputs,
    /// Non-MD3 components ([`Origin::Other`]); needs the `extras` feature.
    #[cfg(feature = "extras")]
    Extras,
}

impl Category {
    /// Every variant, in m3.material.io order.
    #[cfg(feature = "extras")]
    pub const ALL: &'static [Self] = &[
        Self::Actions,
        Self::Communication,
        Self::Containment,
        Self::Navigation,
        Self::Selection,
        Self::TextInputs,
        Self::Extras,
    ];

    /// Every variant, in m3.material.io order.
    #[cfg(not(feature = "extras"))]
    pub const ALL: &'static [Self] = &[
        Self::Actions,
        Self::Communication,
        Self::Containment,
        Self::Navigation,
        Self::Selection,
        Self::TextInputs,
    ];

    /// Display name in the context's locale.
    pub fn title(self, i18n: &egui_mineral::i18n::I18n) -> String {
        crate::tr(
            i18n,
            match self {
                Self::Actions => "mineral-category-actions",
                Self::Communication => "mineral-category-communication",
                Self::Containment => "mineral-category-containment",
                Self::Navigation => "mineral-category-navigation",
                Self::Selection => "mineral-category-selection",
                Self::TextInputs => "mineral-category-text-inputs",
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
    /// Spec provenance.
    pub origin: Origin,
    /// Spec or pattern URL.
    pub spec: &'static str,
}

/// Provenance of a stylekit component.
pub trait Component {
    /// Provenance, also listed in [`CATALOG`].
    const META: ComponentMeta;
}

macro_rules! meta {
    ($ty:ident $(<$lt:lifetime>)?, $category:ident, $origin:expr, $spec:expr) => {
        impl $(<$lt>)? Component for $ty $(<$lt>)? {
            const META: ComponentMeta = ComponentMeta {
                name: stringify!($ty),
                category: Category::$category,
                origin: $origin,
                spec: $spec,
            };
        }
    };
}

// Only the prelude: a cataloged type that it does not export fails to compile here.
use crate::prelude::*;

meta!(
    PushButton<'a>,
    Actions,
    Origin::Md3,
    "https://m3.material.io/components/buttons/specs"
);

meta!(
    IconButton<'a>,
    Actions,
    Origin::Md3,
    "https://m3.material.io/components/icon-buttons/specs"
);

meta!(
    Fab,
    Actions,
    Origin::Md3,
    "https://m3.material.io/components/floating-action-button/specs"
);

meta!(
    ExtendedFab,
    Actions,
    Origin::Md3,
    "https://m3.material.io/components/extended-fab/specs"
);

meta!(
    Card,
    Containment,
    Origin::Md3,
    "https://m3.material.io/components/cards/specs"
);

meta!(
    TopAppBar<'a>,
    Navigation,
    Origin::Md3,
    "https://m3.material.io/components/app-bars/specs"
);

meta!(
    Dialog,
    Containment,
    Origin::Md3,
    "https://m3.material.io/components/dialogs/specs"
);

meta!(
    TextField<'a>,
    TextInputs,
    Origin::Md3,
    "https://m3.material.io/components/text-fields/specs"
);

meta!(
    CheckButton<'a>,
    Selection,
    Origin::Md3,
    "https://m3.material.io/components/checkbox/specs"
);

meta!(
    Radio,
    Selection,
    Origin::Md3,
    "https://m3.material.io/components/radio-button/specs"
);

meta!(
    Switch<'a>,
    Selection,
    Origin::Md3,
    "https://m3.material.io/components/switch/specs"
);

meta!(
    ValueSlider<'a>,
    Selection,
    Origin::Md3,
    "https://m3.material.io/components/sliders/specs"
);

meta!(
    Chip<'a>,
    Selection,
    Origin::Md3,
    "https://m3.material.io/components/chips/specs"
);

meta!(
    InputChip,
    Selection,
    Origin::Md3,
    "https://m3.material.io/components/chips/specs"
);

meta!(
    Badge,
    Communication,
    Origin::Md3,
    "https://m3.material.io/components/badges/specs"
);

meta!(
    Divider,
    Containment,
    Origin::Md3,
    "https://m3.material.io/components/divider/specs"
);

meta!(
    LinearProgressIndicator,
    Communication,
    Origin::Md3,
    "https://m3.material.io/components/progress-indicators/specs"
);

meta!(
    CircularProgressIndicator,
    Communication,
    Origin::Md3,
    "https://m3.material.io/components/progress-indicators/specs"
);

meta!(
    Tabs<'a>,
    Navigation,
    Origin::Md3,
    "https://m3.material.io/components/tabs/specs"
);

meta!(
    NavigationBar<'a>,
    Navigation,
    Origin::Md3,
    "https://m3.material.io/components/navigation-bar/specs"
);

meta!(
    NavigationRail<'a>,
    Navigation,
    Origin::Md3,
    "https://m3.material.io/components/navigation-rail/specs"
);

meta!(
    NavigationDrawer<'a>,
    Navigation,
    Origin::Md3,
    "https://m3.material.io/components/navigation-drawer/specs"
);

meta!(
    Menu,
    Selection,
    Origin::Md3,
    "https://m3.material.io/components/menus/specs"
);

meta!(
    ListItem,
    Containment,
    Origin::Md3,
    "https://m3.material.io/components/lists/specs"
);

meta!(
    RichTooltip,
    Communication,
    Origin::Md3,
    "https://m3.material.io/components/tooltips/specs"
);

meta!(
    Snackbar,
    Communication,
    Origin::Md3,
    "https://m3.material.io/components/snackbar/specs"
);

meta!(
    SnackbarHost<'a>,
    Communication,
    Origin::Md3,
    "https://m3.material.io/components/snackbar/specs"
);

meta!(
    PlainTooltip,
    Communication,
    Origin::Md3,
    "https://m3.material.io/components/tooltips/specs"
);

meta!(
    BottomSheet,
    Containment,
    Origin::Md3,
    "https://m3.material.io/components/bottom-sheets/specs"
);

meta!(
    SideSheet,
    Containment,
    Origin::Md3,
    "https://m3.material.io/components/side-sheets/specs"
);

meta!(
    SearchBar<'a>,
    Navigation,
    Origin::Md3,
    "https://m3.material.io/components/search/specs"
);

meta!(
    SearchSuggestions<'a>,
    Navigation,
    Origin::Md3,
    "https://m3.material.io/components/search/specs"
);

meta!(
    DatePicker<'a>,
    Selection,
    Origin::Md3,
    "https://m3.material.io/components/date-pickers/specs"
);

meta!(
    DateRangePicker<'a>,
    Selection,
    Origin::Md3,
    "https://m3.material.io/components/date-pickers/specs"
);

meta!(
    TimePicker<'a>,
    Selection,
    Origin::Md3,
    "https://m3.material.io/components/time-pickers/specs"
);

meta!(
    Carousel<'a>,
    Containment,
    Origin::Md3,
    "https://m3.material.io/components/carousel/specs"
);

meta!(
    ButtonGroup<'a>,
    Actions,
    Origin::Md3Expressive,
    "https://m3.material.io/components/button-groups/specs"
);

meta!(
    SegmentedButton<'a>,
    Actions,
    Origin::Md3,
    "https://m3.material.io/components/segmented-buttons/specs"
);

meta!(
    SplitButton,
    Actions,
    Origin::Md3Expressive,
    "https://m3.material.io/components/split-button/specs"
);

meta!(
    FabMenu,
    Actions,
    Origin::Md3Expressive,
    "https://m3.material.io/components/fab-menu/specs"
);

meta!(
    Toolbar,
    Navigation,
    Origin::Md3Expressive,
    "https://m3.material.io/components/toolbars/specs"
);

meta!(
    LoadingIndicator,
    Communication,
    Origin::Md3Expressive,
    "https://m3.material.io/components/loading-indicator/specs"
);

meta!(
    SearchView<'a>,
    Navigation,
    Origin::Md3,
    "https://m3.material.io/components/search/specs"
);

meta!(
    PullToRefresh,
    Communication,
    Origin::Md3,
    "https://developer.android.com/reference/kotlin/androidx/compose/material3/pulltorefresh/package-summary"
);

meta!(
    Select<'a>,
    Selection,
    Origin::Md3,
    "https://m3.material.io/components/menus/specs"
);

const RADIO_GROUP: ComponentMeta = ComponentMeta {
    name: "RadioGroup",
    category: Category::Selection,
    origin: Origin::Md3,
    spec: "https://m3.material.io/components/radio-button/specs",
};

impl<T> Component for RadioGroup<'_, T> {
    const META: ComponentMeta = RADIO_GROUP;
}

#[cfg(feature = "extras")]
mod extras {
    use super::{Category, Component, ComponentMeta, Kit, Origin};
    use crate::prelude::*;

    meta!(
        ImageTile,
        Extras,
        Origin::Other(&[Kit::Mui]),
        "https://mui.com/material-ui/react-image-list/"
    );

    meta!(
        MenuStrip,
        Extras,
        Origin::Other(&[
            Kit::Mui,
            Kit::Fluent,
            Kit::Ant,
            Kit::Radix,
            Kit::Shadcn,
            Kit::AppleHig,
            Kit::Bootstrap
        ]),
        "https://www.w3.org/WAI/ARIA/apg/patterns/menubar/"
    );

    meta!(
        Popover,
        Extras,
        Origin::Other(&[
            Kit::Mui,
            Kit::Fluent,
            Kit::Ant,
            Kit::Radix,
            Kit::Shadcn,
            Kit::Chakra,
            Kit::AppleHig,
            Kit::Bootstrap
        ]),
        "https://www.radix-ui.com/primitives/docs/components/popover"
    );

    meta!(
        HoverCard,
        Extras,
        Origin::Other(&[Kit::Radix, Kit::Shadcn, Kit::Chakra]),
        "https://www.radix-ui.com/primitives/docs/components/hover-card"
    );

    meta!(
        CommandPalette<'a>,
        Extras,
        Origin::Other(&[Kit::Shadcn, Kit::Chakra]),
        "https://ui.shadcn.com/docs/components/command"
    );

    meta!(
        Tour<'a>,
        Extras,
        Origin::Other(&[Kit::Fluent, Kit::Ant, Kit::Chakra]),
        "https://ant.design/components/tour"
    );

    meta!(
        FloatingPanel,
        Extras,
        Origin::Other(&[Kit::Chakra, Kit::AppleHig]),
        "https://chakra-ui.com/docs/components/floating-panel"
    );

    meta!(
        Combobox<'a>,
        Extras,
        Origin::Other(&[Kit::Mui, Kit::Fluent, Kit::Ant, Kit::Chakra, Kit::Shadcn]),
        "https://www.w3.org/WAI/ARIA/apg/patterns/combobox/"
    );

    meta!(
        NumberField<'a>,
        Extras,
        Origin::Other(&[Kit::Mui, Kit::Fluent, Kit::Ant, Kit::Chakra, Kit::AppleHig]),
        "https://www.w3.org/WAI/ARIA/apg/patterns/spinbutton/"
    );

    meta!(
        OtpField<'a>,
        Extras,
        Origin::Other(&[Kit::Ant, Kit::Radix, Kit::Shadcn, Kit::Chakra]),
        "https://ui.shadcn.com/docs/components/input-otp"
    );

    meta!(
        TagsField<'a>,
        Extras,
        Origin::Other(&[Kit::Fluent, Kit::Ant, Kit::Chakra, Kit::AppleHig]),
        "https://chakra-ui.com/docs/components/tags-input"
    );

    meta!(
        Rating<'a>,
        Extras,
        Origin::Other(&[Kit::Mui, Kit::Fluent, Kit::Ant, Kit::Chakra]),
        "https://mui.com/material-ui/react-rating/"
    );

    meta!(
        SplitPane,
        Extras,
        Origin::Other(&[Kit::Ant, Kit::Shadcn, Kit::AppleHig]),
        "https://www.w3.org/WAI/ARIA/apg/patterns/windowsplitter/"
    );

    meta!(
        ColorPicker<'a>,
        Extras,
        Origin::Other(&[Kit::Fluent, Kit::Ant, Kit::Chakra, Kit::AppleHig]),
        "https://ant.design/components/color-picker"
    );

    meta!(
        TransferList<'a>,
        Extras,
        Origin::Other(&[Kit::Mui, Kit::Ant]),
        "https://mui.com/material-ui/react-transfer-list/"
    );

    meta!(
        StatusBar,
        Extras,
        Origin::Other(&[Kit::Fluent, Kit::AppleHig]),
        "https://learn.microsoft.com/en-us/windows/apps/design/basics/commanding-basics"
    );

    meta!(
        SettingRow,
        Extras,
        Origin::Other(&[Kit::Fluent, Kit::Ant, Kit::AppleHig]),
        "https://learn.microsoft.com/en-us/dotnet/communitytoolkit/windows/settingscontrols/settingscard"
    );

    meta!(
        InlineEdit<'a>,
        Extras,
        Origin::Other(&[Kit::Chakra, Kit::Ant]),
        "https://chakra-ui.com/docs/components/editable"
    );

    meta!(
        FileDropZone,
        Extras,
        Origin::Other(&[Kit::Ant, Kit::Chakra]),
        "https://chakra-ui.com/docs/components/file-upload"
    );

    meta!(
        DateField<'a>,
        Extras,
        Origin::Other(&[Kit::Mui, Kit::AppleHig]),
        "https://mui.com/x/react-date-pickers/date-field/"
    );

    meta!(
        DataTable<'a>,
        Extras,
        Origin::Other(&[
            Kit::Mui,
            Kit::Flutter,
            Kit::Fluent,
            Kit::Ant,
            Kit::Shadcn,
            Kit::Chakra,
            Kit::AppleHig,
            Kit::Bootstrap
        ]),
        "https://m2.material.io/components/data-tables"
    );

    meta!(
        Accordion<'a>,
        Extras,
        Origin::Other(&[
            Kit::Mui,
            Kit::Fluent,
            Kit::Ant,
            Kit::Radix,
            Kit::Shadcn,
            Kit::Chakra,
            Kit::AppleHig,
            Kit::Bootstrap
        ]),
        "https://mui.com/material-ui/react-accordion/"
    );

    meta!(
        Pagination<'a>,
        Extras,
        Origin::Other(&[Kit::Mui, Kit::Ant, Kit::Shadcn, Kit::Chakra, Kit::Bootstrap]),
        "https://mui.com/material-ui/react-pagination/"
    );

    meta!(
        Stepper<'a>,
        Extras,
        Origin::Other(&[Kit::Mui, Kit::Flutter, Kit::Ant, Kit::Chakra]),
        "https://mui.com/material-ui/react-stepper/"
    );

    meta!(
        Breadcrumbs,
        Extras,
        Origin::Other(&[
            Kit::Mui,
            Kit::Fluent,
            Kit::Ant,
            Kit::Shadcn,
            Kit::Chakra,
            Kit::Bootstrap
        ]),
        "https://mui.com/material-ui/react-breadcrumbs/"
    );

    meta!(
        Avatar,
        Extras,
        Origin::Other(&[Kit::Mui, Kit::Fluent, Kit::Ant, Kit::Radix, Kit::Shadcn, Kit::Chakra]),
        "https://mui.com/material-ui/react-avatar/"
    );

    meta!(
        AvatarGroup,
        Extras,
        Origin::Other(&[Kit::Mui, Kit::Fluent, Kit::Ant, Kit::Radix, Kit::Shadcn, Kit::Chakra]),
        "https://mui.com/material-ui/react-avatar/#grouped"
    );

    meta!(
        Skeleton,
        Extras,
        Origin::Other(&[
            Kit::Mui,
            Kit::Fluent,
            Kit::Ant,
            Kit::Shadcn,
            Kit::Chakra,
            Kit::Bootstrap
        ]),
        "https://mui.com/material-ui/react-skeleton/"
    );

    meta!(
        Timeline,
        Extras,
        Origin::Other(&[Kit::Mui, Kit::Ant, Kit::Shadcn, Kit::Chakra]),
        "https://mui.com/material-ui/react-timeline/"
    );

    meta!(
        DescriptionList,
        Extras,
        Origin::Other(&[Kit::Ant, Kit::Chakra]),
        "https://ant.design/components/descriptions"
    );

    meta!(
        Alert,
        Extras,
        Origin::Other(&[
            Kit::Mui,
            Kit::Fluent,
            Kit::Ant,
            Kit::Shadcn,
            Kit::Chakra,
            Kit::AppleHig,
            Kit::Bootstrap
        ]),
        "https://m2.material.io/components/banners"
    );

    meta!(
        EmptyPlaceholder,
        Extras,
        Origin::Other(&[Kit::Mui, Kit::Ant, Kit::Shadcn, Kit::Chakra]),
        "https://ant.design/components/empty"
    );

    meta!(
        Statistic,
        Extras,
        Origin::Other(&[Kit::Mui, Kit::Ant, Kit::Shadcn, Kit::Chakra]),
        "https://chakra-ui.com/docs/components/stat"
    );

    meta!(
        Kbd,
        Extras,
        Origin::Other(&[Kit::Ant, Kit::Shadcn, Kit::Chakra]),
        "https://chakra-ui.com/docs/components/kbd"
    );

    meta!(
        TextLink,
        Extras,
        Origin::Other(&[Kit::Mui, Kit::Fluent, Kit::Ant, Kit::Chakra, Kit::Bootstrap]),
        "https://mui.com/material-ui/react-link/"
    );

    meta!(
        ImageList<'a>,
        Extras,
        Origin::Other(&[
            Kit::Mui,
            Kit::Flutter,
            Kit::Ant,
            Kit::Chakra,
            Kit::AppleHig,
            Kit::Bootstrap
        ]),
        "https://mui.com/material-ui/react-image-list/"
    );

    pub(super) const TREE_VIEW: ComponentMeta = ComponentMeta {
        name: "TreeView",
        category: Category::Extras,
        origin: Origin::Other(&[Kit::Mui, Kit::Fluent, Kit::Ant, Kit::Chakra, Kit::AppleHig]),
        spec: "https://mui.com/x/react-tree-view/",
    };

    impl<N: egui_mineral::tree::TreeData> Component for TreeView<'_, N> {
        const META: ComponentMeta = TREE_VIEW;
    }

    pub(super) const REORDERABLE_LIST: ComponentMeta = ComponentMeta {
        name: "ReorderableList",
        category: Category::Extras,
        origin: Origin::Other(&[Kit::Flutter, Kit::Ant]),
        spec: "https://api.flutter.dev/flutter/material/ReorderableListView-class.html",
    };

    impl<T> Component for ReorderableList<'_, T> {
        const META: ComponentMeta = REORDERABLE_LIST;
    }
}

/// Every public component with its category and spec origin.
pub const CATALOG: &[ComponentMeta] = &[
    <PushButton<'static>>::META,
    <IconButton<'static>>::META,
    Fab::META,
    ExtendedFab::META,
    Card::META,
    <TopAppBar<'static>>::META,
    Dialog::META,
    <TextField<'static>>::META,
    <CheckButton<'static>>::META,
    Radio::META,
    <Switch<'static>>::META,
    <ValueSlider<'static>>::META,
    <Chip<'static>>::META,
    InputChip::META,
    Badge::META,
    Divider::META,
    LinearProgressIndicator::META,
    CircularProgressIndicator::META,
    <Tabs<'static>>::META,
    <NavigationBar<'static>>::META,
    <NavigationRail<'static>>::META,
    <NavigationDrawer<'static>>::META,
    Menu::META,
    ListItem::META,
    RichTooltip::META,
    Snackbar::META,
    <SnackbarHost<'static>>::META,
    PlainTooltip::META,
    #[cfg(feature = "extras")]
    ImageTile::META,
    RADIO_GROUP,
    #[cfg(feature = "extras")]
    extras::REORDERABLE_LIST,
    BottomSheet::META,
    SideSheet::META,
    <SearchBar<'static>>::META,
    <SearchSuggestions<'static>>::META,
    <DatePicker<'static>>::META,
    <DateRangePicker<'static>>::META,
    <TimePicker<'static>>::META,
    Carousel::META,
    <ButtonGroup<'static>>::META,
    <SegmentedButton<'static>>::META,
    SplitButton::META,
    FabMenu::META,
    Toolbar::META,
    LoadingIndicator::META,
    <SearchView<'static>>::META,
    #[cfg(feature = "extras")]
    MenuStrip::META,
    #[cfg(feature = "extras")]
    Popover::META,
    #[cfg(feature = "extras")]
    HoverCard::META,
    #[cfg(feature = "extras")]
    <CommandPalette<'static>>::META,
    #[cfg(feature = "extras")]
    <Tour<'static>>::META,
    #[cfg(feature = "extras")]
    FloatingPanel::META,
    PullToRefresh::META,
    <Select<'static>>::META,
    #[cfg(feature = "extras")]
    <Combobox<'static>>::META,
    #[cfg(feature = "extras")]
    <NumberField<'static>>::META,
    #[cfg(feature = "extras")]
    <OtpField<'static>>::META,
    #[cfg(feature = "extras")]
    <TagsField<'static>>::META,
    #[cfg(feature = "extras")]
    <Rating<'static>>::META,
    #[cfg(feature = "extras")]
    SplitPane::META,
    #[cfg(feature = "extras")]
    <ColorPicker<'static>>::META,
    #[cfg(feature = "extras")]
    <TransferList<'static>>::META,
    #[cfg(feature = "extras")]
    <InlineEdit<'static>>::META,
    #[cfg(feature = "extras")]
    SettingRow::META,
    #[cfg(feature = "extras")]
    StatusBar::META,
    #[cfg(feature = "extras")]
    FileDropZone::META,
    #[cfg(feature = "extras")]
    <DateField<'static>>::META,
    #[cfg(feature = "extras")]
    <DataTable<'static>>::META,
    #[cfg(feature = "extras")]
    extras::TREE_VIEW,
    #[cfg(feature = "extras")]
    <Accordion<'static>>::META,
    #[cfg(feature = "extras")]
    <Pagination<'static>>::META,
    #[cfg(feature = "extras")]
    <Stepper<'static>>::META,
    #[cfg(feature = "extras")]
    Breadcrumbs::META,
    #[cfg(feature = "extras")]
    Avatar::META,
    #[cfg(feature = "extras")]
    AvatarGroup::META,
    #[cfg(feature = "extras")]
    Skeleton::META,
    #[cfg(feature = "extras")]
    Timeline::META,
    #[cfg(feature = "extras")]
    DescriptionList::META,
    #[cfg(feature = "extras")]
    Alert::META,
    #[cfg(feature = "extras")]
    EmptyPlaceholder::META,
    #[cfg(feature = "extras")]
    Statistic::META,
    #[cfg(feature = "extras")]
    Kbd::META,
    #[cfg(feature = "extras")]
    TextLink::META,
    #[cfg(feature = "extras")]
    <ImageList<'static>>::META,
];

#[cfg(all(test, feature = "bundled-icons"))]
mod tests {
    use std::collections::BTreeMap;

    use super::{CATALOG, Category};

    #[test]
    fn category_titles_are_translated() {
        let rt = crate::test_support::runtime(&egui::Context::default());
        for category in Category::ALL {
            let title = category.title(rt.i18n());
            assert!(!title.starts_with("mineral-"), "{category:?}: {title}");
        }
        assert_eq!(Category::TextInputs.title(rt.i18n()), "Text inputs");
    }

    fn declared() -> Vec<&'static str> {
        let src = include_str!("catalog.rs");
        let body = src.split("pub const CATALOG").next().expect("CATALOG declared");
        let body = if cfg!(feature = "extras") {
            body
        } else {
            body.split("#[cfg(feature = \"extras\")]\nmod extras")
                .next()
                .expect("core part")
        };
        let metas = body.split("meta!(").skip(1).filter_map(|m| {
            m.split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .find(|t| !t.is_empty())
        });
        let consts = body.split("name: \"").skip(1).filter_map(|m| m.split('"').next());
        metas.chain(consts).collect()
    }

    #[test]
    fn every_component_is_cataloged_once() {
        let mut count = BTreeMap::<&str, i32>::new();
        for name in declared() {
            *count.entry(name).or_default() += 1;
        }
        for meta in CATALOG {
            *count.entry(meta.name).or_default() -= 1;
        }
        count.retain(|_, n| *n != 0);
        assert!(count.is_empty(), "declared minus cataloged: {count:?}");
    }
}

//! Navigation: app bars, navigation bar, drawer and rail, search, tabs, toolbars ([m3](https://m3.material.io/components)).

pub(crate) mod nav;
pub(crate) mod search;
pub(crate) mod tabs;
pub(crate) mod toolbar;
pub(crate) mod top_app_bar;

pub use nav::{
    NavDestination, NavLabels, NavigationBar, NavigationDrawer, NavigationDrawerModalResponse, NavigationRail,
    NavigationRailResponse,
};
pub use search::{
    SearchBar, SearchBarOutput, SearchBarState, SearchSuggestions, SearchSuggestionsOutput, SearchView, SearchViewMode,
    SearchViewResponse,
};
pub use tabs::{Tab, Tabs, TabsVariant};
pub use toolbar::{DockedToolbar, FloatingToolbar, ToolbarColors, ToolbarResponse};
pub use top_app_bar::{TopAppBar, TopAppBarResponse, TopAppBarVariant};

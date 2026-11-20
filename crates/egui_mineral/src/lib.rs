//! Headless behavior core of [egui_mineral](https://github.com/lajosbencz/egui_mineral) for
//! [egui](https://github.com/emilk/egui). It paints nothing; a stylekit supplies the visuals. Most apps use
//! [egui_mineral_material](https://crates.io/crates/egui_mineral_material). Depend on this crate to write a
//! stylekit or headless widgets.
//!
//! - Unified touch and pointer input, flexbox layout
//! - Behavior models: focus, selection, ranges, tables, dates, forms, routing
//! - Fluent and ICU4X i18n, right-to-left layout
//! - An accessibility semantic tree and an agent API
//!
//! Targets egui 0.37 on native and wasm. Pre-1.0: any minor release may break the API.
//!
//! # Writing a stylekit
//!
//! A [`Stylekit`](runtime::Stylekit) supplies policies, catalogs, setup and the style to a
//! [`Runtime`](runtime::Runtime), one per native window. Anything yielding a
//! [`WidgetResponse`](widget::WidgetResponse) is a [`MineralWidget`](widget::MineralWidget), directly or through
//! [`Composite`](widget::Composite), and is shown with `ui.mineral(..)` on the [`MineralUi`](widget::MineralUi)
//! that [`Runtime::pass`](runtime::Runtime::pass) hands out. A minimal headless pass:
//!
//! ```
//! use egui_mineral::{i18n::Catalog, policy::Policies, runtime::{Runtime, Stylekit}};
//!
//! struct Kit;
//!
//! impl Stylekit for Kit {
//!     fn policies(&self, _: &egui::Context) -> Policies {
//!         Policies::baseline()
//!     }
//!     fn catalogs(&self) -> &'static [Catalog] {
//!         static CATALOGS: [Catalog; 1] = [Catalog::new("en", "hello = Hello")];
//!         &CATALOGS
//!     }
//!     fn setup(&self, _: &egui::Context) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!         Ok(())
//!     }
//!     fn style(&self, ctx: &egui::Context, theme: egui::Theme) -> std::sync::Arc<egui::Style> {
//!         ctx.style_of(theme)
//!     }
//! }
//!
//! let ctx = egui::Context::default();
//! let rt = Runtime::new(&ctx, Kit)?;
//! let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
//!     rt.pass(ui, |ui| {
//!         let hello = ui.core().i18n().tr("hello", &[]);
//!         ui.label(hello);
//!     });
//! });
//! output.textures_delta.clear();
//! # Ok::<(), egui_mineral::runtime::InitError>(())
//! ```
//!
//! # Cargo features
//!
//! | Feature | Default | Adds |
//! |---|---|---|
//! | `icu` | yes | ICU4X formatting, collation and direction; without it [`AsciiFormat`](i18n::AsciiFormat) |
//! | `persist-json` | | `persist::Json` codec |
//! | `persist-toml` | yes | `persist::Toml` codec |
//! | `persist-fs` | yes | Native-only `persist::FsStorage`; pick a codec feature to encode values |
//! | `platform-pickers` | | File dialogs (`rfd`): `NativePlatform` implements [`PlatformPickers`](platform::PlatformPickers) |
//! | `platform-clipboard` | | OS clipboard (`arboard`): [`PlatformClipboard`](platform::PlatformClipboard) |
//! | `platform-notify` | | OS notifications (`notify-rust`, D-Bus on Linux): [`PlatformNotifier`](platform::PlatformNotifier) |
//! | `platform-native` | | `platform-pickers`, `platform-clipboard` and `platform-notify` |
//! | `web-save-stream` | | Chromium streams [`FileWriter`](platform::FileWriter) to disk; other browsers buffer |
//! | `unstable-hotpatch` | | Native only: Dioxus `subsecond` hot-patching of [`Runtime::pass`](runtime::Runtime::pass); the `dx` CLI must match `subsecond` 0.7.10; enable it from the command line only (`dx serve --hotpatch --features egui_mineral/unstable-hotpatch`), never in a manifest, since feature unification would ship it; touch the app's `main.rs` before each `dx serve` (see `docs/blockers.md`) |
//!
//! # App state
//!
//! App state is the app's own struct; there is no store. Widgets borrow fields (`&mut T`); discrete actions and
//! [`bus`] results fold through one `update`. The [`bus`] module shows the pattern, with [`undo`].
#![cfg_attr(docsrs, feature(doc_cfg))]
// docs/rules.md rule 10; tests may panic.
#![cfg_attr(
    not(test),
    deny(clippy::panic, clippy::unwrap_used, clippy::expect_used, clippy::unreachable)
)]

/// Public dependencies, re-exported so apps need not pin matching versions: `egui`, `jiff` for [`date`], `accesskit` for
/// [`agent`] and [`semantic`] (via egui), `serde` for [`persist`] and `futures_util` for [`bus`]; `egui_taffy` is
/// [`layout::flex`].
pub use egui;
pub use egui::accesskit;
pub use futures_util;
pub use jiff;
pub use serde;

// Public path: `kit::opaque_debug`.
#[doc(hidden)]
#[macro_export]
macro_rules! __opaque_debug {
    ($name:ident <$lt:lifetime, $($ty:ident),+>) => {
        impl<$($ty),+> ::core::fmt::Debug for $name<$lt, $($ty),+> {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_struct(stringify!($name)).finish_non_exhaustive()
            }
        }
    };
    ($name:ident <$($ty:ident),+>) => {
        impl<$($ty),+> ::core::fmt::Debug for $name<$($ty),+> {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_struct(stringify!($name)).finish_non_exhaustive()
            }
        }
    };
    ($name:ident $(<$($lt:lifetime),+>)?) => {
        impl ::core::fmt::Debug for $name $(<$($lt),+>)? {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_struct(stringify!($name)).finish_non_exhaustive()
            }
        }
    };
}

// Public path: `kit::delegate_widget_meta`.
#[doc(hidden)]
#[macro_export]
macro_rules! __delegate_widget_meta {
    ($field:tt) => {
        fn id_salt(&self) -> $crate::egui::IdSalt {
            $crate::widget::MineralWidget::id_salt(&self.$field)
        }

        fn sense(&self) -> $crate::egui::Sense {
            $crate::widget::MineralWidget::sense(&self.$field)
        }

        fn enabled(&self) -> bool {
            $crate::widget::MineralWidget::enabled(&self.$field)
        }

        fn min_target(&self, ui: &$crate::widget::MineralUi<'_, Self::Kit>) -> $crate::layout::MinTarget {
            $crate::widget::MineralWidget::min_target(&self.$field, ui)
        }

        fn access_key(&self) -> ::core::option::Option<$crate::shortcut::Mnemonic> {
            $crate::widget::MineralWidget::access_key(&self.$field)
        }

        fn accelerator(&self) -> ::core::option::Option<$crate::egui::KeyboardShortcut> {
            $crate::widget::MineralWidget::accelerator(&self.$field)
        }

        fn composite(&self) -> bool {
            $crate::widget::MineralWidget::composite(&self.$field)
        }

        fn pan_axis(&self) -> ::core::option::Option<$crate::layout::Axis> {
            $crate::widget::MineralWidget::pan_axis(&self.$field)
        }
    };
}

// Public path: `kit::widget_meta`; keywords in any order.
#[doc(hidden)]
#[macro_export]
macro_rules! __widget_meta {
    () => {};
    (salt $(, $($rest:tt)*)?) => {
        fn id_salt(&self) -> $crate::egui::IdSalt {
            self.salt.id_salt()
        }
        $crate::__widget_meta!($($($rest)*)?);
    };
    (enabled $(, $($rest:tt)*)?) => {
        fn enabled(&self) -> bool {
            !self.disabled
        }
        $crate::__widget_meta!($($($rest)*)?);
    };
    (no_target $(, $($rest:tt)*)?) => {
        fn min_target(&self, _: &$crate::widget::MineralUi<'_, Self::Kit>) -> $crate::layout::MinTarget {
            $crate::layout::MinTarget::None
        }
        $crate::__widget_meta!($($($rest)*)?);
    };
    (frame_sense $(, $($rest:tt)*)?) => {
        fn sense(&self) -> $crate::egui::Sense {
            $crate::kit::edit::frame_sense()
        }
        $crate::__widget_meta!($($($rest)*)?);
    };
    (hover $(, $($rest:tt)*)?) => {
        fn sense(&self) -> $crate::egui::Sense {
            $crate::egui::Sense::hover()
        }
        $crate::__widget_meta!($($($rest)*)?);
    };
}

pub mod action;
/// Agent API: snapshots and actions over raw egui input.
pub mod agent;
/// Anchored popup placement: side, alignment, flip and shift.
pub mod anchor;
pub mod bus;
pub mod combobox;
/// Calendar models over `jiff`: month grids, bounds, ranges, 12-hour conversion.
pub mod date;
pub mod dnd;
pub mod edit;
/// Expansion state for trees, accordions and disclosures.
pub mod expand;
/// Fuzzy filtering over `nucleo-matcher`.
pub mod filter;
pub mod focus;
pub mod form;
#[cfg(all(feature = "unstable-hotpatch", not(target_arch = "wasm32")))]
mod hotpatch;
pub mod i18n;
/// Icon glyphs, two-form icons and the icon font installer shared by the stylekits.
pub mod icon;
/// Input abstraction layer: unified events, gestures, key repeat, debounce.
pub mod input;
#[cfg(target_arch = "wasm32")]
mod js;
pub mod kit;
pub mod layout;
/// Character-cell input masks.
pub mod mask;
/// Springs, presence animations and reduced-motion handling.
pub mod motion;
/// Typed route stack.
pub mod nav;
/// Numeric field editing model.
pub mod number;
pub mod overlay;
/// Paging and page-item windows.
pub mod pagination;
pub mod palette;
/// Virtualized lists, shape caching and size memoization.
pub mod perf;
/// Storage and codecs for app state.
pub mod persist;
pub mod platform;
pub mod policy;
/// Value ranges, scales and slider track model.
pub mod range;
pub mod runtime;
/// Selection models and tri-state checks.
pub mod select;
/// Accessibility semantic tree and its serializers.
pub mod semantic;
mod setting;
pub mod shortcut;
pub mod splitter;
pub mod status;
/// Step flow model.
pub mod steps;
/// Table sort and column-width models.
pub mod table;
/// Tag token parsing.
pub mod tags;
#[cfg(test)]
mod test_support;
/// Timed message queue.
pub mod toast;
/// Tree model: flattening, navigation, drag moves.
pub mod tree;
/// Undo and redo history.
pub mod undo;
/// Widget trait, responses and the `ui.mineral(..)` driver.
pub mod widget;
/// Native viewport windows.
pub mod window;

/// Common imports for apps; widget authors add [`kit::prelude`].
pub mod prelude {
    pub use crate::{
        action::Clicked,
        anchor::Side,
        bus::{Bus, Spawner, SpawnerExt},
        date::{Date, DateBounds, DateRange, Time},
        dnd::Reorder,
        edit::EditEvent,
        expand::Expansion,
        form::{Form, FormState},
        icon::Icon,
        layout::{Axis, Edge},
        mask::Charset,
        nav::{Route, Router},
        number::NumberSource,
        overlay::Dismiss,
        palette::{CommandPaletteResponse, PaletteCommand},
        platform::{FileFilter, FileType},
        range::{RangeModel, Scale},
        runtime::{InitError, Runtime},
        select::{Selection, SelectionResponse},
        semantic::{KeySalt, KeyedSalt},
        shortcut::ActionId,
        status::{StatusItem, StatusProgress, StatusTone},
        steps::Steps,
        toast::ToastQueue,
        tree::{TreeData, TreeDataMut},
        widget::{MineralUi, SizeExt, WidgetResponse},
    };
}

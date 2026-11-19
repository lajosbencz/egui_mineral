//! Unopinionated, headless core of `egui_mineral`: unified touch and pointer input, flexbox layout, behavior models
//! (focus, selection, ranges, tables, dates, forms, routing), Fluent and ICU4X i18n, an accessibility semantic tree
//! and an agent API. It paints nothing; stylekits such as `egui_mineral_material` supply the visuals.
//!
//! Start with [`runtime::Runtime`] and [`widget::MineralWidget`]: anything yielding a [`widget::WidgetResponse`]
//! implements it (directly or through [`widget::Composite`]) and is shown with `ui.mineral(..)` on the
//! [`widget::MineralUi`] that [`runtime::Runtime::pass`] hands out. A [`runtime::Stylekit`] supplies policies,
//! catalogs, setup and the style; one runtime per native window. A minimal headless pass:
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
//!         &[("en", "hello = Hello")]
//!     }
//!     fn setup(&self, _: &egui::Context) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!         Ok(())
//!     }
//!     fn style(&self, ctx: &egui::Context) -> std::sync::Arc<egui::Style> {
//!         ctx.global_style()
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
//! Cargo features; `icu` and `persist-fs` are default:
//! - `icu`: ICU4X formatting, collation and direction (`IcuFormat`); without it [`i18n::AsciiFormat`].
//! - `persist-fs`: file-backed [`persist`] stores.
//! - `platform-native`: native file dialogs, clipboard and notifications for [`platform`].
//! - `web-save-stream`: Chromium streams [`platform::FileWriter`] to disk; other browsers buffer.
//!
//! App state is the app's own struct; there is no store. Widgets borrow fields (`&mut T`) for local edits;
//! discrete actions and [`bus`] results are one message type folded by one `update`, undoable ones through
//! [`undo`]:
//!
//! ```
//! use egui_mineral::{bus::Bus, undo::{UndoCommand, UndoStack}};
//!
//! #[derive(Default)]
//! struct Todos(Vec<String>);
//!
//! enum Msg {
//!     Add(String),
//!     Loaded(Vec<String>),
//! }
//!
//! enum Request {
//!     Fetch,
//! }
//!
//! struct Add(String);
//!
//! impl UndoCommand<Todos> for Add {
//!     fn apply(&mut self, s: &mut Todos) {
//!         s.0.push(self.0.clone());
//!     }
//!     fn revert(&mut self, s: &mut Todos) {
//!         s.0.pop();
//!     }
//!     fn label(&self) -> &str {
//!         "Add"
//!     }
//! }
//!
//! struct App {
//!     todos: Todos,
//!     undo: UndoStack<Todos>,
//!     bus: Bus<Request, Msg>,
//! }
//!
//! impl App {
//!     fn update(&mut self, msg: Msg) {
//!         match msg {
//!             Msg::Add(text) => self.undo.execute(Add(text), &mut self.todos),
//!             Msg::Loaded(items) => self.todos.0 = items,
//!         }
//!     }
//!
//!     /// Start of each pass, before the UI reads state.
//!     fn drain(&mut self) {
//!         let msgs: Vec<_> = self.bus.drain().collect();
//!         msgs.into_iter().for_each(|m| self.update(m));
//!     }
//! }
//!
//! let ctx = egui::Context::default();
//! let (bus, handle) = Bus::new(&ctx);
//! let mut app = App { todos: Todos::default(), undo: UndoStack::new(100), bus };
//! app.bus.send(Request::Fetch).ok();
//! // The handler, on the app's executor, answers through `handle.events` and wakes the UI.
//! handle.events.send(Msg::Loaded(vec!["a".into()])).ok();
//! app.drain();
//! app.update(Msg::Add("b".into()));
//! app.undo.undo(&mut app.todos);
//! assert_eq!(app.todos.0, ["a"]);
//! ```
//!
//! Repository: <https://github.com/lajosbencz/egui_mineral>; architecture and rules:
//! <https://github.com/lajosbencz/egui_mineral/blob/master/docs/rules.md>.
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

        fn min_target(&self) -> $crate::layout::MinTarget {
            $crate::widget::MineralWidget::min_target(&self.$field)
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
        fn min_target(&self) -> $crate::layout::MinTarget {
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

/// Agent API: snapshots and actions over raw egui input.
pub mod agent;
/// Anchored popup placement: side, alignment, flip and shift.
pub mod anchor;
/// Executor-agnostic command and event channels.
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
pub mod i18n;
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
        anchor::Side,
        bus::{Bus, Spawner, SpawnerExt},
        date::{Date, DateBounds, DateRange, Time},
        dnd::Reorder,
        edit::EditEvent,
        expand::Expansion,
        form::{Form, FormState},
        layout::{Axis, Edge},
        mask::Charset,
        nav::{Route, Router},
        number::NumberSource,
        overlay::Dismiss,
        platform::{FileFilter, FileType},
        range::{RangeModel, Scale},
        runtime::Runtime,
        select::Selection,
        semantic::{KeySalt, KeyedSalt},
        shortcut::ActionId,
        steps::Steps,
        toast::ToastQueue,
        tree::{TreeData, TreeDataMut},
        widget::{MineralUi, SizeExt, WidgetResponse},
    };
}

//! Stylekit authoring API: the building blocks the bundled Material and Fluent kits are made of, public so apps
//! and third-party kits compose their own widgets and kits on the same footing. App code reaches for the core
//! modules (`focus`, `overlay`, `semantic`, ...); widget code adds these.
//!
//! - Interaction: [`interact`] (tap-aware `interact`, press-to-focus, arrow locks, roving memory, cursors, key
//!   events), [`layer`] (library areas and modals, Escape and default action), [`scroll`].
//! - Semantics: [`nodes`] (collection checks, parent rects, culled emits).
//! - Painting: [`paint`] (glyph shapes, tabular figures, window close glyph).
//! - State: [`state`] (temp memory, memos, pass caches), [`choice`] (value-bound selection groups).
//! - Component models shared by both kits: [`command_palette`], [`menu`], [`menubar`], [`strip`], [`rating`].
//! - Arguments: [`sanitize`] (rule 10: clamp and report instead of panicking).
//! - [`prelude`] for widget authors; [`tr`], [`tr_or`] and [`scoped_id`] at the root.
//!
//! Stability: the module follows the crate's semver like any other public module; there are no `#[doc(hidden)]`
//! escape hatches. A building block lands here once a kit needs it in two places; kit-specific helpers stay in
//! the kit. Submodule names name the authoring concern and never repeat a core module's name.

pub mod choice;
pub mod command_palette;
pub mod interact;
pub mod layer;
pub mod menu;
pub mod menubar;
pub mod nodes;
pub mod paint;
pub mod rating;
pub mod sanitize;
pub mod scroll;
pub mod state;
pub mod strip;

use crate::i18n::{I18n, IntoMessage};

/// Text of `message`.
pub fn tr<'a, const N: usize>(i18n: &I18n, message: impl IntoMessage<'a, N>) -> String {
    i18n.tr(message)
}

/// `over` when set, else the text of `message`.
pub fn tr_or<'a, const N: usize>(i18n: &I18n, over: Option<&str>, message: impl IntoMessage<'a, N>) -> String {
    over.map_or_else(|| i18n.tr(message), str::to_owned)
}

/// `id` made distinct per runtime and window: `Context` data and popup ids are shared by every viewport and runtime,
/// so state keyed by an app salt alone would leak between them. Call outside `Context` lock closures.
pub fn scoped_id(core: &crate::runtime::Core, id: egui::Id) -> egui::Id {
    id.with((core.ctx().viewport_id(), core.id()))
}

/// Implements `Debug` printing the type name only, for types holding closures, trait objects or borrows.
/// Takes the type name with optional lifetime or type arguments: `opaque_debug!(Form<'_, T>)`, `opaque_debug!(Runtime<K>)`.
pub use crate::__opaque_debug as opaque_debug;

/// Forwards every `MineralWidget` method except `layout`, `update` and `semantics` to field `$field` (`0`,
/// `widget`), inside the `impl` block of a wrapping widget; new trait methods reach every wrapper.
pub use crate::__delegate_widget_meta as delegate_widget_meta;
pub use crate::__widget_meta as widget_meta;

/// Common imports for widget authors.
pub mod prelude {
    pub use crate::{
        input::{PanClaim, PanMode},
        kit::{choice::Choice, state::TempState},
        layout::{Constraints, Measure, MinTarget},
        runtime::Stylekit,
        select::{ItemKey, Selection, SelectionResponse},
        semantic::{Action, AgentKey, KeySalt, KeyedSalt, Role, SemanticNode, StableId},
        shortcut::Mnemonic,
        widget::{
            Availability, Composite, Interaction, MineralUi, MineralWidget, PanGate, SizeExt, WidgetFrame,
            WidgetResponse,
        },
    };
}

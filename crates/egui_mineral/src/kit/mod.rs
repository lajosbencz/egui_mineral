//! Stylekit plumbing: helpers that widget implementations call and apps do not. Submodules mirror the
//! core module they serve.

pub mod edit;
pub mod focus;
pub mod i18n;
pub mod icon;
pub mod input;
pub mod menubar;
pub mod overlay;
pub mod paint;
pub mod palette;
pub mod rating;
pub mod sanitize;
pub mod scroll;
pub mod semantic;
pub mod state;
pub mod strip;

/// `id` made distinct per runtime and window: `Context` data and popup ids are shared by every viewport and runtime,
/// so state keyed by an app salt alone would leak between them. Call outside `Context` lock closures.
pub fn scoped_id(core: &crate::runtime::Core, id: egui::Id) -> egui::Id {
    id.with((core.ctx().viewport_id(), core.id()))
}

/// Implements `Debug` printing the type name only, for types holding closures, trait objects or borrows.
/// Takes the type name with optional lifetime or type arguments: `opaque_debug!(Form<'_>)`, `opaque_debug!(Runtime<K>)`.
pub use crate::__opaque_debug as opaque_debug;

/// Forwards every `MineralWidget` method except `layout`, `update` and `semantics` to field `$field` (`0`,
/// `widget`), inside the `impl` block of a wrapping widget; new trait methods reach every wrapper.
pub use crate::__delegate_widget_meta as delegate_widget_meta;
pub use crate::__widget_meta as widget_meta;

/// Common imports for widget authors.
pub mod prelude {
    pub use crate::{
        input::PanClaim,
        kit::state::TempState,
        layout::{Constraints, Measure, MinTarget},
        runtime::Stylekit,
        semantic::{Action, AgentKey, KeySalt, KeyedSalt, Role, SemanticNode, StableId},
        shortcut::Mnemonic,
        widget::{
            Availability, Composite, Interaction, MineralUi, MineralWidget, PanGate, SizeExt, WidgetFrame,
            WidgetResponse,
        },
    };
}

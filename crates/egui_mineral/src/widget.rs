use std::ops::{Bound, Deref, DerefMut, RangeBounds};
use std::{cell::OnceCell, rc::Rc};

use egui::{IdSalt, KeyboardShortcut, Pos2, Sense, Ui, Vec2, vec2};

use crate::{
    i18n::I18n,
    input::{InputContext, PanClaim, PanMode, PointerKind},
    layout::{Constraints, Measure, MinTarget, Zones},
    perf,
    runtime::{Core, Runtime, Stylekit},
    semantic::{self, SemanticNode},
    shortcut::{self, ActionId, Mnemonic, Trigger},
};

/// Per-pass frame handed to [`MineralWidget::update`]: input, zones, response and constraints.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct WidgetFrame<'a> {
    /// Input of this pass, from [`Core::input`].
    pub input: &'a InputContext,
    /// Placed zones in screen points.
    pub zones: Zones,
    /// Senses `zones.hit`.
    pub response: egui::Response,
    /// Position of a gesture tap on `response` this pass; `Some` also makes `response.clicked()` true.
    pub tapped: Option<Pos2>,
    /// What [`MineralWidget::layout`] was measured against.
    pub constraints: Constraints,
    /// Touch pan arbitration of this pass, active with [`MineralWidget::pan`].
    pub pan: PanGate,
}

/// Touch pan arbitration state of a pannable ([`MineralWidget::pan`], [`Self::update`]), applied to its responses by
/// [`Self::apply`]; inactive for mouse and pen input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[must_use]
pub struct PanGate(GateState);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum GateState {
    #[default]
    Inactive,
    // Pan not claimed on the axis.
    Unclaimed,
    // Pan relayed to the scroll area.
    Relaying,
    // Claimed; the claiming pass reports a drag start.
    Claimed {
        started: bool,
    },
}

impl PanGate {
    /// Touch input on a pannable: drags are reported only once claimed.
    pub fn active(&self) -> bool {
        self.0 != GateState::Inactive
    }

    /// `r` as the widget may see it: while the pan is not claimed, drag state is cleared and
    /// the press state too once the pan is relayed to the scroll area; the claiming pass reports a drag start.
    /// The plumbing applies it to [`WidgetFrame::response`], widgets apply it to their own drag-sensing responses.
    #[must_use]
    pub fn apply(&self, mut r: egui::Response) -> egui::Response {
        use egui::response::Flags;
        match self.0 {
            GateState::Inactive => {}
            GateState::Claimed { started } => {
                if started {
                    r.flags |= Flags::DRAG_STARTED | Flags::DRAGGED;
                }
            }
            GateState::Unclaimed | GateState::Relaying => {
                r.flags
                    .remove(Flags::DRAG_STARTED | Flags::DRAGGED | Flags::DRAG_STOPPED);
                if self.0 == GateState::Relaying {
                    r.flags.remove(Flags::IS_POINTER_BUTTON_DOWN_ON);
                }
            }
        }
        r
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct PanGateState {
    claim: PanClaim,
    owned: bool,
}

impl PanGate {
    /// Per pass: arbitrates the touch pans on `response`, the drag-sensing response of an enabled pannable that
    /// is not a [`MineralWidget`]; inactive unless the modality is touch.
    pub fn update<K>(ui: &MineralUi<'_, K>, mode: PanMode, response: &egui::Response) -> Self {
        if ui.pass_input().modality != PointerKind::Touch {
            return Self::default();
        }
        let id = response.id.with("pan-gate");
        let mut st = ui.data(|d| d.get_temp::<PanGateState>(id)).unwrap_or_default();
        let was_relaying = st.claim.relaying();
        let frame = st.claim.update(ui, mode, response);
        let gate = Self(if frame.is_some() {
            GateState::Claimed { started: !st.owned }
        } else if was_relaying || st.claim.relaying() {
            GateState::Relaying
        } else {
            GateState::Unclaimed
        });
        st.owned = frame.is_some_and(|f| !f.ended);
        ui.data_mut(|d| d.insert_temp(id, st));
        gate
    }
}

impl WidgetFrame<'_> {
    /// Click or tap position on `response` this pass.
    pub fn press_pos(&self) -> Option<Pos2> {
        let r = &self.response;
        r.interact_pointer_pos().filter(|_| r.clicked()).or(self.tapped)
    }

    /// Drag, click or tap position on `response` this pass.
    pub fn drag_pos(&self) -> Option<Pos2> {
        let r = &self.response;
        r.interact_pointer_pos()
            .filter(|_| r.dragged() || r.clicked())
            .or(self.tapped)
    }
}

/// `changed` means a bound value changed; setting it marks the response changed.
#[derive(Clone, Debug)]
pub struct WidgetResponse<O = ()> {
    response: egui::Response,
    changed: bool,
    output: O,
    zones: Zones,
}

impl<O> WidgetResponse<O> {
    /// Zones are the response rect until [`MineralUi::mineral`] sets the widget's own.
    pub fn new(response: egui::Response, output: O) -> Self {
        Self {
            zones: response.rect.into(),
            response,
            changed: false,
            output,
        }
    }

    /// ORs `changed` in; `true` also calls `mark_changed` on the response.
    #[must_use]
    pub fn with_changed(mut self, changed: bool) -> Self {
        if changed {
            self.response.mark_changed();
        }
        self.changed |= changed;
        self
    }

    /// Hit-zone response, or the widget's own when it returns another.
    pub fn response(&self) -> &egui::Response {
        &self.response
    }

    /// Unlike `Response::changed`, ignores inner edits that bind nothing.
    pub fn changed(&self) -> bool {
        self.changed
    }

    /// Widget events of this pass; `Default` when culled.
    pub fn output(&self) -> &O {
        &self.output
    }

    /// Placed zones; the response rect until [`MineralUi::mineral`] sets them.
    pub fn zones(&self) -> Zones {
        self.zones
    }

    /// Drops `output` and the `changed` flag; `response.changed()` still reflects it.
    pub fn into_response(self) -> egui::Response {
        self.response
    }

    /// Drops the response.
    pub fn into_output(self) -> O {
        self.output
    }

    /// Response and output; `changed` survives as `response.changed()`.
    pub fn into_parts(self) -> (egui::Response, O) {
        (self.response, self.output)
    }
}

impl<O> Deref for WidgetResponse<O> {
    type Target = egui::Response;

    fn deref(&self) -> &egui::Response {
        &self.response
    }
}

/// Result of showing a control that carries its [`egui::Response`]; containers that label or wire controls
/// they host take it.
pub trait AsResponse {
    /// Response of the control.
    fn as_response(&self) -> &egui::Response;
}

impl AsResponse for egui::Response {
    fn as_response(&self) -> &egui::Response {
        self
    }
}

impl<O> AsResponse for WidgetResponse<O> {
    fn as_response(&self) -> &egui::Response {
        &self.response
    }
}

impl<R> AsResponse for egui::InnerResponse<R> {
    fn as_response(&self) -> &egui::Response {
        &self.response
    }
}

/// Pointer and focus state of an element, read by stylekit state layers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Interaction {
    /// Pointer over the response.
    pub hovered: bool,
    /// Primary button held down on the response.
    pub pressed: bool,
    /// Focus that should be indicated ([`crate::focus::visible`]): keyboard-reached or a text edit.
    pub focused: bool,
    /// Drag in progress.
    pub dragged: bool,
    /// From [`Availability::Disabled`]; [`Self::dominant`] reports `None` while set.
    pub disabled: bool,
}

/// Interaction state a stylekit paints an indication for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InteractionKind {
    /// Top priority in [`Interaction::dominant`].
    Dragged,
    /// Outranks focus and hover.
    Pressed,
    /// Outranks hover.
    Focused,
    /// Lowest priority.
    Hovered,
}

/// Interaction state; disabled widgets ignore input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Availability {
    /// Senses input normally.
    #[default]
    Enabled,
    /// Senses hover only; stylekits paint disabled colors.
    Disabled,
}

impl Availability {
    /// [`Self::Enabled`] for `true`.
    pub const fn enabled_if(enabled: bool) -> Self {
        if enabled { Self::Enabled } else { Self::Disabled }
    }

    /// [`Self::Disabled`] for `true`.
    pub const fn disabled_if(disabled: bool) -> Self {
        Self::enabled_if(!disabled)
    }

    /// `true` for [`Self::Enabled`].
    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled)
    }
}

impl Interaction {
    /// Snapshot of `response` this pass; `disabled` from `availability`.
    pub fn of(response: &egui::Response, availability: Availability) -> Self {
        Self {
            hovered: response.hovered(),
            pressed: response.is_pointer_button_down_on(),
            focused: crate::focus::visible(&response.ctx, response.id),
            dragged: response.dragged(),
            disabled: !availability.is_enabled(),
        }
    }

    /// Pointer over the response.
    #[must_use]
    pub const fn with_hovered(self, hovered: bool) -> Self {
        Self { hovered, ..self }
    }

    /// Primary button held down on the response.
    #[must_use]
    pub const fn with_pressed(self, pressed: bool) -> Self {
        Self { pressed, ..self }
    }

    /// Focus that should be indicated.
    #[must_use]
    pub const fn with_focused(self, focused: bool) -> Self {
        Self { focused, ..self }
    }

    /// Drag in progress.
    #[must_use]
    pub const fn with_dragged(self, dragged: bool) -> Self {
        Self { dragged, ..self }
    }

    /// [`Self::dominant`] reports `None` while set.
    #[must_use]
    pub const fn with_disabled(self, disabled: bool) -> Self {
        Self { disabled, ..self }
    }

    /// Highest-priority active state in [`InteractionKind`] order; `None` while disabled or idle.
    pub fn dominant(self) -> Option<InteractionKind> {
        if self.disabled {
            None
        } else if self.dragged {
            Some(InteractionKind::Dragged)
        } else if self.pressed {
            Some(InteractionKind::Pressed)
        } else if self.focused {
            Some(InteractionKind::Focused)
        } else if self.hovered {
            Some(InteractionKind::Hovered)
        } else {
            None
        }
    }
}

/// `Ui` plus the [`Runtime`] it shows widgets with; built by [`Runtime::pass`]
/// or [`Runtime::ui`]. The only way to show a [`MineralWidget`].
pub struct MineralUi<'a, K> {
    ui: &'a mut Ui,
    rt: &'a Runtime<K>,
    input: OnceCell<Rc<InputContext>>,
}

crate::kit::opaque_debug!(MineralUi<'_, K>);

impl<'a, K> MineralUi<'a, K> {
    pub(crate) fn new(ui: &'a mut Ui, rt: &'a Runtime<K>) -> Self {
        Self {
            ui,
            rt,
            input: OnceCell::new(),
        }
    }

    // Root `Ui`s (area, window) get the kit's style.
    pub(crate) fn nested(ui: &'a mut Ui, rt: &'a Runtime<K>, input: OnceCell<Rc<InputContext>>) -> Self {
        if ui.stack().is_root_ui() {
            ui.set_style(rt.style());
        }
        Self { ui, rt, input }
    }

    /// Input of this pass, read once per `MineralUi` ([`Core::input`] locks per call).
    pub fn pass_input(&self) -> Rc<InputContext> {
        Rc::clone(self.input.get_or_init(|| Rc::new(self.rt.core().input())))
    }

    /// Outlives this borrow of the `Ui`; wraps the plain `Ui` of egui closures via [`Runtime::ui`] where no
    /// layout method of `MineralUi` applies.
    pub fn rt(&self) -> &'a Runtime<K> {
        self.rt
    }

    /// Kit-independent configuration and per-pass input.
    pub fn core(&self) -> &'a Core {
        self.rt.core()
    }

    /// Shortcut for `core().i18n()`.
    pub fn i18n(&self) -> &'a I18n {
        self.rt.core().i18n()
    }

    /// Stylekit state.
    pub fn kit(&self) -> &'a K {
        self.rt.kit()
    }

    /// Value of type `T` of the nearest enclosing [`Self::with_scoped`]; `None` outside any.
    pub fn scoped<T: Send + Sync + 'static>(&self) -> Option<&T> {
        let key = std::any::type_name::<T>();
        self.ui.stack().iter().find_map(|s| s.info.tags.get_downcast::<T>(key))
    }

    /// Runs `add_contents` in a child `Ui` whose subtree reads `value` through [`Self::scoped`]; the slot is
    /// the type `T`, so a kit keeps one private or newtype `T` per setting. Popups and dialogs opened from it
    /// are separate roots and do not inherit it.
    pub fn with_scoped<T: Send + Sync + 'static, R>(
        &mut self,
        value: T,
        add_contents: impl FnOnce(&mut MineralUi<'_, K>) -> R,
    ) -> R {
        let mut info = egui::UiStackInfo::default();
        info.tags
            .insert(std::any::type_name::<T>(), Some(std::sync::Arc::new(value)));
        self.scope_builder(egui::UiBuilder::new().ui_stack_info(info), add_contents)
            .inner
    }
}

macro_rules! layout_wrappers {
    ($($(#[$doc:meta])* $name:ident($($arg:ident: $ty:ty),*);)+) => {
        /// egui layout containers whose closure gets a `MineralUi`, so nested content needs no
        /// [`Runtime::ui`] re-wrap. Each forwards to the `egui::Ui` method of the same name.
        impl<K> MineralUi<'_, K> {
            $(
                $(#[$doc])*
                pub fn $name<R>(
                    &mut self,
                    $($arg: $ty,)*
                    add_contents: impl FnOnce(&mut MineralUi<'_, K>) -> R,
                ) -> egui::InnerResponse<R> {
                    let (rt, input) = (self.rt, self.input.clone());
                    self.ui.$name($($arg,)* |ui| add_contents(&mut MineralUi { ui, rt, input }))
                }
            )+
        }
    };
}

layout_wrappers! {
    /// Row in the layout direction; see `egui::Ui::horizontal`.
    horizontal();
    /// Row centered across the full available height.
    horizontal_centered();
    /// Row aligned to the top of the full available height.
    horizontal_top();
    /// Row that wraps at the available width.
    horizontal_wrapped();
    /// Top-down column, start-aligned.
    vertical();
    /// Top-down column, centered.
    vertical_centered();
    /// Top-down column, centered, children filling the width.
    vertical_centered_justified();
    /// Single child centered and filling the available space.
    centered_and_justified();
    /// Child `Ui` allocated at `desired_size` in the current layout.
    allocate_ui(desired_size: Vec2);
    /// Child `Ui` allocated at `desired_size`, laid out by `layout`.
    allocate_ui_with_layout(desired_size: Vec2, layout: egui::Layout);
    /// Child `Ui` with its own style and layout state.
    scope();
    /// Child `Ui` configured by `ui_builder`.
    scope_builder(ui_builder: egui::UiBuilder);
    /// Child `Ui` laid out by `layout`.
    with_layout(layout: egui::Layout);
    /// Child `Ui` whose ids are salted with `id_salt`.
    push_id(id_salt: impl egui::AsIdSalt);
    /// Child `Ui` indented by the style's indent.
    indent(id_salt: impl egui::AsIdSalt);
    /// Child `Ui` in the group frame of the current egui style.
    group();
    /// Child `Ui` that is disabled unless `enabled`.
    add_enabled_ui(enabled: bool);
}

impl<K> MineralUi<'_, K> {
    /// Row in the reading direction whose items center on its tallest item of the previous pass, where
    /// [`Self::horizontal`] centers them on `interact_size.y`; the first show discards to measure, later height
    /// changes apply the next pass. Keyed by position, as egui auto ids.
    pub fn row<R>(&mut self, add_contents: impl FnOnce(&mut MineralUi<'_, K>) -> R) -> egui::InnerResponse<R> {
        let id = self.ui.next_auto_id().with("mineral::row");
        let ctx = self.ui.ctx().clone();
        let last = perf::size_or_discard(&ctx, id);
        let height = last.map_or(self.ui.spacing().interact_size.y, |s| s.y);
        let size = vec2(self.ui.available_size_before_wrap().x, height);
        let layout = self.rt.core().i18n().direction().row();
        let out = self.allocate_ui_with_layout(size, layout, add_contents);
        let tallest = vec2(0.0, out.response.rect.height());
        if last.is_none_or(|s| perf::size_changed(tallest, s)) {
            perf::store_size(&ctx, id, tallest);
        }
        out
    }
}

/// egui container shown with a [`MineralUi`] body, so its content needs no [`Runtime::ui`] re-wrap:
/// `Frame::new().show_in(ui, |ui| ui.mineral(..))`. Implemented for egui's `Ui`-taking containers and for
/// `Area` and `Window`, which open on the `Ui`'s context; their content gets the kit's style, the window decoration
/// keeps egui's under [`GlobalStyle::Egui`](crate::runtime::GlobalStyle::Egui).
pub trait ShowIn: Sized {
    /// What the container's `show` returns for a body returning `R`.
    type Output<R>;

    /// Forwards to the container's `show`; `Area` and `Window` open on `ui.ctx()`, outside `ui`'s layout.
    fn show_in<K, R>(
        self,
        ui: &mut MineralUi<'_, K>,
        add_contents: impl FnOnce(&mut MineralUi<'_, K>) -> R,
    ) -> Self::Output<R>;
}

macro_rules! show_in {
    ($($ty:ty => $out:ty, |$this:ident, $ui:ident, $body:ident, $rt:ident| $show:expr;)+) => {$(
        impl ShowIn for $ty {
            type Output<R> = $out;

            fn show_in<K, R>(
                self,
                ui: &mut MineralUi<'_, K>,
                add_contents: impl FnOnce(&mut MineralUi<'_, K>) -> R,
            ) -> Self::Output<R> {
                let (rt, input) = (ui.rt, ui.input.clone());
                let $body = |ui: &mut Ui| add_contents(&mut MineralUi::nested(ui, rt, input));
                let ($this, $ui, $rt) = (self, &mut *ui.ui, rt);
                $show
            }
        }
    )+};
}

show_in! {
    egui::Frame => egui::InnerResponse<R>, |c, ui, body, _rt| c.show(ui, body);
    egui::ScrollArea => egui::scroll_area::ScrollAreaOutput<R>, |c, ui, body, _rt| c.show(ui, body);
    egui::Grid => egui::InnerResponse<R>, |c, ui, body, _rt| c.show(ui, body);
    egui::CollapsingHeader => egui::CollapsingResponse<R>, |c, ui, body, _rt| c.show(ui, body);
    egui::Resize => R, |c, ui, body, _rt| c.show(ui, body);
    egui::Panel => egui::InnerResponse<R>, |c, ui, body, _rt| c.show(ui, body);
    egui::CentralPanel => egui::InnerResponse<R>, |c, ui, body, _rt| c.show(ui, body);
    egui::Area => egui::InnerResponse<R>, |c, ui, body, _rt| c.show(ui.ctx(), body);
    // The window's content `Ui` is a child of its root, so it gets the kit's style here.
    egui::Window<'_> => Option<egui::InnerResponse<Option<R>>>, |c, ui, body, rt| c.show(ui.ctx(), |ui| {
        ui.set_style(rt.style());
        body(ui)
    });
}

impl<K: Stylekit> MineralUi<'_, K> {
    /// Lays out, places zones, runs input and emits semantics for `widget`; culls off-screen leaves.
    pub fn mineral<W: MineralWidget<Kit = K>>(&mut self, widget: W) -> WidgetResponse<W::Output> {
        show(self, widget)
    }

    /// Minimum hit target edge in points of `target` here, as [`Self::mineral`] resolves it: the policy
    /// value for the pass's pointer, adjusted by [`Stylekit::min_target`]. For parts a widget hit-tests
    /// or reserves space for itself.
    pub fn target(&self, target: MinTarget) -> f32 {
        let points = target.points_for(self.core(), self.pass_input().modality);
        self.kit().min_target(self, points)
    }
}

impl<K> Deref for MineralUi<'_, K> {
    type Target = Ui;

    fn deref(&self) -> &Ui {
        self.ui
    }
}

impl<K> DerefMut for MineralUi<'_, K> {
    fn deref_mut(&mut self) -> &mut Ui {
        self.ui
    }
}

/// Leaf widget driven by [`MineralUi::mineral`]: layout, update and semantics.
pub trait MineralWidget {
    /// Stylekit of the [`MineralUi`] that shows it.
    type Kit;

    /// Per-widget memory in egui temp data, keyed by [`Self::id_salt`]; read again for culled widgets' semantics.
    type State: Clone + Default + Send + Sync + 'static;

    /// Widget events; `Default` when the widget is culled.
    type Output: Default;

    /// Salts the persistent id of the state and response; keys default from it.
    fn id_salt(&self) -> IdSalt;

    /// Defaults to click; ignored while disabled.
    fn sense(&self) -> Sense {
        Sense::click()
    }

    /// Drag directions of a pannable. On touch the plumbing arbitrates it with a [`PanClaim`]: the drag state of
    /// [`WidgetFrame::response`] is reported only once the mode claims a pan, any other pan scrolls the
    /// enclosing scroll area. Mouse input is unaffected.
    fn pan(&self) -> Option<PanMode> {
        None
    }

    /// Disabled widgets sense hover only, so neither clicks nor taps reach them.
    fn enabled(&self) -> bool {
        true
    }

    /// Defaults to [`MinTarget::Auto`] (policy target for the current pointer). The kit adjusts the
    /// resolved edge through [`Stylekit::min_target`].
    fn min_target(&self, _: &MineralUi<'_, Self::Kit>) -> MinTarget {
        MinTarget::Auto
    }

    /// Alt+letter activates the widget like a click while it is enabled and visible.
    fn access_key(&self) -> Option<Mnemonic> {
        None
    }

    /// Shortcut that activates the widget like a click while it is enabled and visible.
    fn accelerator(&self) -> Option<KeyboardShortcut> {
        None
    }

    /// [`Composite`] only: senses hover, takes no minimum target and runs off-screen too, so each
    /// child culls itself and still emits semantics.
    fn composite(&self) -> bool {
        false
    }

    /// `Vec2` converts into a [`Measure`] without margin; `state` is the one [`Self::update`] gets.
    fn layout(&self, ui: &MineralUi<'_, Self::Kit>, constraints: Constraints, state: &Self::State) -> Measure;

    /// Runs only when visible or targeted by AccessKit; `state` is persisted afterwards.
    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Self::Kit>,
        frame: &WidgetFrame<'_>,
        state: &mut Self::State,
    ) -> WidgetResponse<Self::Output>;

    /// `None` for widgets whose parts emit their own nodes, or that are decorative.
    fn semantics(
        &self,
        i18n: &I18n,
        response: &WidgetResponse<Self::Output>,
        state: &Self::State,
    ) -> Option<SemanticNode>;
}

fn show<K: Stylekit, W: MineralWidget<Kit = K>>(ui: &mut MineralUi<'_, K>, mut widget: W) -> WidgetResponse<W::Output> {
    let core = ui.core();
    let input = &*ui.pass_input();
    let id = ui.make_persistent_id(widget.id_salt());
    // Taken out and put back, so the state is moved instead of cloned.
    let mut state = ui.data_mut(|d| d.remove_temp::<W::State>(id)).unwrap_or_default();
    let composite = widget.composite();
    let min = if composite {
        0.0
    } else {
        ui.target(widget.min_target(ui))
    };
    // Height is unbounded: horizontal layouts report only the current row height as available.
    let constraints = Constraints::loose(Vec2::new(ui.available_width(), f32::INFINITY));
    let measure = widget.layout(ui, constraints, &state);
    let sense = if widget.enabled() && !composite {
        widget.sense()
    } else {
        Sense::hover()
    };
    let local = Zones::new(measure, min);
    let size = local.outer.size();
    let (_, slot) = ui.allocate_space(size);
    let zones = local.translate(ui.layout().align_size_within_rect(size, slot).min.to_vec2());
    // Keyed by the salt, not the layout slot, so focus and drags follow the widget when siblings move.
    let mut response = ui.interact(zones.hit, id, sense);
    response.set_intrinsic_size(size);

    let ctx = ui.ctx().clone();
    let access_key = widget.access_key();
    let accelerator = widget.accelerator();
    // Culled widgets still emit semantics, and run when AccessKit targets them, so agents reach them off-screen.
    if !composite && !ui.is_rect_visible(zones.outer) && !accesskit_targets(ui, response.id) {
        let mut out = WidgetResponse::new(response, W::Output::default());
        out.zones = zones;
        emit_node(&ctx, out.response.id, zones.hit, || {
            node_of(core, &widget, &out, &state, access_key, accelerator)
        });
        crate::kit::state::store_evicting(&ctx, ctx.cumulative_pass_nr(), id, state);
        return out;
    }

    let tapped = input.tap_on(&mut response);
    let pan = match widget.pan() {
        Some(mode) if widget.enabled() && !composite => PanGate::update(ui, mode, &response),
        _ => PanGate::default(),
    };
    let mut response = pan.apply(response);
    if sense.is_focusable() {
        crate::kit::interact::follow_press(&response);
    }
    if widget.enabled() {
        let scope = shortcut::scope_of(&ctx, response.layer_id);
        let mut activated = false;
        if let Some(mnemonic) = access_key {
            shortcut::bind_mnemonic(&ctx, scope, mnemonic, Trigger::Alt, response.id);
            activated |= shortcut::take_item(&ctx, response.id);
        }
        if let Some(accelerator) = accelerator {
            let action = ActionId::widget(id);
            shortcut::bind(&ctx, scope, accelerator, &action);
            activated |= shortcut::take_action(&ctx, &action);
        }
        if activated && response.sense.senses_click() {
            // doc(hidden) on egui 0.37; see `InputContext::tap_on`.
            response.flags |= egui::response::Flags::FAKE_PRIMARY_CLICKED;
        }
    }
    let frame = WidgetFrame {
        input,
        zones,
        response,
        tapped,
        constraints,
        pan,
    };
    let frame_id = frame.response.id;
    let scope = semantic::enter(&ctx, frame_id, ui.unique_id());
    let mut out = widget.update(ui, &frame, &mut state);
    out.zones = zones;
    // Clones of `frame.response` carry the tap already; only a foreign response needs it.
    if out.response.id != frame_id {
        input.tap_on(&mut out.response);
    }

    semantic::leave(&ctx, scope, Some(out.response.id));
    emit_node(&ctx, out.response.id, zones.hit, || {
        node_of(core, &widget, &out, &state, access_key, accelerator)
    });
    crate::kit::state::store_evicting(&ctx, ctx.cumulative_pass_nr(), id, state);
    out
}

fn accesskit_targets(ui: &Ui, id: egui::Id) -> bool {
    let target = id.accesskit_id();
    ui.input(|i| {
        i.events
            .iter()
            .any(|e| matches!(e, egui::Event::AccessKitActionRequest(r) if r.target_node == target))
    })
}

fn node_of<W: MineralWidget>(
    core: &Core,
    widget: &W,
    out: &WidgetResponse<W::Output>,
    state: &W::State,
    access_key: Option<Mnemonic>,
    accelerator: Option<KeyboardShortcut>,
) -> Option<SemanticNode> {
    let mut node = widget.semantics(core.i18n(), out, state)?;
    node.access_key = access_key.or(node.access_key);
    node.shortcut = accelerator.or(node.shortcut);
    Some(node)
}

fn emit_node(ctx: &egui::Context, id: egui::Id, rect: egui::Rect, node: impl FnOnce() -> Option<SemanticNode>) {
    if crate::kit::nodes::wanted(ctx, Some(id))
        && let Some(node) = node()
    {
        semantic::emit(ctx, Some(id), rect, || node);
    }
}

/// Widget laid out from child widgets. Implements [`MineralWidget`] with [`MineralWidget::composite`]: it
/// is sized from its previous pass (the first pass is discarded) and composed in a child ui at its slot.
pub trait Composite {
    /// Stylekit of the [`MineralUi`] that shows it.
    type Kit;

    /// Child events, read from [`WidgetResponse::output`].
    type Output: Default;

    /// Salts the child ui and the stored size of the previous pass.
    fn id_salt(&self) -> IdSalt;

    /// `constraints` bound the child ui; forward them to the child that should fill it.
    fn compose(&mut self, ui: &mut MineralUi<'_, Self::Kit>, constraints: Constraints) -> WidgetResponse<Self::Output>;
}

fn composite_size_id(ui: &Ui, id_salt: IdSalt) -> egui::Id {
    ui.make_persistent_id(id_salt).with("composite-size")
}

impl<C: Composite> MineralWidget for C {
    type Kit = C::Kit;
    type State = ();
    type Output = C::Output;

    fn id_salt(&self) -> IdSalt {
        Composite::id_salt(self)
    }

    fn composite(&self) -> bool {
        true
    }

    fn layout(&self, ui: &MineralUi<'_, C::Kit>, _: Constraints, (): &Self::State) -> Measure {
        perf::size_or_discard(ui.ctx(), composite_size_id(ui, Composite::id_salt(self)))
            .unwrap_or(Vec2::ZERO)
            .into()
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, C::Kit>,
        frame: &WidgetFrame<'_>,
        (): &mut (),
    ) -> WidgetResponse<C::Output> {
        let salt = Composite::id_salt(self);
        // From the reading-start edge, so a first pass at zero size measures on screen in RTL too.
        let dir = ui.i18n().direction();
        let outer = frame.zones.outer;
        let max = egui::Rect::from_x_y_ranges(
            dir.span_from(dir.start_edge(outer), frame.constraints.max.x),
            egui::Rangef::new(outer.top(), outer.top() + frame.constraints.max.y),
        );
        let mut child = ui.new_child(egui::UiBuilder::new().id_salt(salt).max_rect(max).layout(dir.column()));
        child.set_min_size(frame.constraints.min);
        let out = self.compose(&mut ui.rt().ui(&mut child), frame.constraints);
        let size = child.min_rect().size();
        if perf::size_changed(size, frame.zones.outer.size()) {
            perf::store_size(ui.ctx(), composite_size_id(ui, salt), size);
            // Off-screen composites adopt the new size on their next visible pass.
            if ui.is_rect_visible(outer) {
                ui.ctx().request_discard("mineral: composite resized");
            }
        }
        out
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse<C::Output>, (): &()) -> Option<SemanticNode> {
        None
    }
}

/// [`MineralWidget`] whose parent constraints are narrowed by the caller's; built by [`SizeExt`].
#[must_use = "show it with `ui.mineral(..)`"]
#[derive(Clone, Debug)]
pub struct Constrained<W> {
    widget: W,
    constraints: Constraints,
}

impl<W: MineralWidget> MineralWidget for Constrained<W> {
    type Kit = W::Kit;
    type State = W::State;
    type Output = W::Output;

    crate::kit::delegate_widget_meta!(widget);

    fn layout(&self, ui: &MineralUi<'_, W::Kit>, constraints: Constraints, state: &Self::State) -> Measure {
        self.widget.layout(ui, constraints.intersect(self.constraints), state)
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, W::Kit>,
        frame: &WidgetFrame<'_>,
        state: &mut W::State,
    ) -> WidgetResponse<W::Output> {
        let frame = WidgetFrame {
            constraints: frame.constraints.intersect(self.constraints),
            ..frame.clone()
        };
        self.widget.update(ui, &frame, state)
    }

    fn semantics(&self, i18n: &I18n, response: &WidgetResponse<W::Output>, state: &W::State) -> Option<SemanticNode> {
        self.widget.semantics(i18n, response, state)
    }
}

/// Caller size override for any [`MineralWidget`] (Compose `Modifier.width/height/size/widthIn/heightIn`), in
/// points against [`Measure::visual`] (margins are extra). Names avoid the components' own `size(Size)` setters;
/// the parent's constraints win where they conflict, and of chained calls on one axis the last (outermost) wins.
pub trait SizeExt: MineralWidget + Sized {
    /// Narrows the parent constraints. Sizes derived from layout can go negative or NaN on overflow, so
    /// `min` saturates to finite `>= 0` and `max` to `>= min` instead of panicking.
    fn constrain(self, constraints: Constraints) -> Constrained<Self> {
        let finite = |v: f32| if v.is_finite() { v.max(0.0) } else { 0.0 };
        let min = Vec2::new(finite(constraints.min.x), finite(constraints.min.y));
        Constrained {
            widget: self,
            constraints: Constraints {
                min,
                max: constraints.max.max(min),
            },
        }
    }

    /// Exact width in points, within the parent's constraints (Compose `Modifier.width`).
    fn preferred_width(self, width: f32) -> Constrained<Self> {
        self.width_range(width..=width)
    }

    /// Exact height in points, within the parent's constraints (Compose `Modifier.height`).
    fn preferred_height(self, height: f32) -> Constrained<Self> {
        self.height_range(height..=height)
    }

    /// Exact size in points, within the parent's constraints (Compose `Modifier.size`).
    fn preferred_size(self, size: Vec2) -> Constrained<Self> {
        self.constrain(Constraints::tight(size))
    }

    /// Inclusive or open-ended: `120.0..=400.0`, `..=400.0`, `120.0..`; an excluded bound counts as inclusive.
    fn width_range(self, width: impl RangeBounds<f32>) -> Constrained<Self> {
        let (min, max) = bounds(&width);
        self.constrain(Constraints {
            min: Vec2::new(min, 0.0),
            max: Vec2::new(max, f32::INFINITY),
        })
    }

    /// Inclusive or open-ended: `48.0..=96.0`, `..=96.0`, `48.0..`; an excluded bound counts as inclusive.
    fn height_range(self, height: impl RangeBounds<f32>) -> Constrained<Self> {
        let (min, max) = bounds(&height);
        self.constrain(Constraints {
            min: Vec2::new(0.0, min),
            max: Vec2::new(f32::INFINITY, max),
        })
    }
}

impl<W: MineralWidget> SizeExt for W {}

fn bounds(range: &impl RangeBounds<f32>) -> (f32, f32) {
    let bound = |b: Bound<&f32>, unbounded: f32| match b {
        Bound::Included(v) => *v,
        Bound::Unbounded => unbounded,
        Bound::Excluded(v) => {
            crate::kit::sanitize::report(format_args!("size ranges are inclusive, got an excluded bound {v}"));
            *v
        }
    };
    (bound(range.start_bound(), 0.0), bound(range.end_bound(), f32::INFINITY))
}

#[cfg(test)]
mod tests {
    use egui::{IdSalt, Vec2, vec2};

    use super::{Interaction, InteractionKind, MineralUi, MineralWidget, ShowIn, SizeExt, WidgetFrame, WidgetResponse};
    use crate::{
        layout::{Constraints, Measure, MinTarget},
        semantic::{Role, SemanticNode, StableId},
        test_support::{TestKit, harness},
    };

    struct Probe;

    impl Probe {
        const NATURAL: Vec2 = vec2(200.0, 40.0);
    }

    type Seen = Option<(Constraints, u32)>;

    impl MineralWidget for Probe {
        type Kit = TestKit;
        type State = u32;
        type Output = Seen;

        fn id_salt(&self) -> IdSalt {
            IdSalt::new("probe")
        }

        fn min_target(&self, _: &MineralUi<'_, Self::Kit>) -> MinTarget {
            MinTarget::None
        }

        fn layout(&self, _: &MineralUi<'_, TestKit>, c: Constraints, _: &Self::State) -> Measure {
            c.constrain(Self::NATURAL).into()
        }

        fn update(
            &mut self,
            _: &mut MineralUi<'_, TestKit>,
            frame: &WidgetFrame<'_>,
            n: &mut u32,
        ) -> WidgetResponse<Seen> {
            *n += 1;
            WidgetResponse::new(frame.response.clone(), Some((frame.constraints, *n)))
        }

        fn semantics(&self, _: &crate::i18n::I18n, _: &WidgetResponse<Seen>, _: &u32) -> Option<SemanticNode> {
            Some(SemanticNode::new(StableId::new("probe"), Role::Group, ""))
        }
    }

    fn shown(
        width: f32,
        add: impl Fn(&mut MineralUi<'_, TestKit>) -> WidgetResponse<Seen> + 'static,
    ) -> (Vec2, Constraints) {
        let h = harness(
            move |ui, out: &mut Option<(Vec2, Constraints)>| {
                ui.set_max_width(width);
                let r = add(ui);
                *out = Some((r.rect.size(), r.output().expect("update ran").0));
            },
            None,
        );
        h.state().value.expect("ran")
    }

    #[test]
    fn row_centers_items_on_its_tallest() {
        let mut h = harness(
            |ui, out: &mut Option<(f32, f32)>| {
                let r = ui.row(|ui| {
                    let (small, _) = ui.allocate_exact_size(vec2(20.0, 10.0), egui::Sense::hover());
                    let (tall, _) = ui.allocate_exact_size(vec2(20.0, 60.0), egui::Sense::hover());
                    (small.center().y, tall.center().y)
                });
                *out = Some(r.inner);
            },
            None,
        );
        h.run();
        let (small, tall) = h.state().value.expect("ran");
        assert!((small - tall).abs() < 0.5, "{small} vs {tall}");
    }

    struct Box(Vec2);

    impl super::Composite for Box {
        type Kit = TestKit;
        type Output = ();

        fn id_salt(&self) -> IdSalt {
            IdSalt::new("box")
        }

        fn compose(&mut self, ui: &mut MineralUi<'_, TestKit>, _: Constraints) -> WidgetResponse<()> {
            let (_, r) = ui.allocate_exact_size(self.0, egui::Sense::hover());
            WidgetResponse::new(r, ())
        }
    }

    // Passes run per frame for `Box(size)` shown after `skip` points of vertical space.
    fn composite_discards(skip: f32, sizes: &[Vec2]) -> Vec<usize> {
        let ctx = egui::Context::default();
        let rt = crate::test_support::runtime(&ctx);
        let mut counts = Vec::new();
        for size in sizes {
            let mut passes = 0;
            crate::test_support::run(&ctx, crate::test_support::input(vec![]), |ui| {
                rt.pass(ui, |ui| {
                    ui.add_space(skip);
                    ui.mineral(Box(*size));
                });
                passes += 1;
            });
            counts.push(passes);
        }
        counts
    }

    #[test]
    fn composite_discards_only_for_visible_real_resizes() {
        let (a, b) = (vec2(100.0, 40.0), vec2(100.3, 40.2));
        assert_eq!(composite_discards(0.0, &[a, a, b, vec2(100.0, 60.0)]), [2, 1, 1, 2]);
        assert_eq!(
            composite_discards(1_000_000.0, &[a, a, vec2(100.0, 60.0)]),
            [2, 1, 1],
            "off-screen resizes are stored without a discard"
        );
    }

    #[test]
    fn constrained_narrows_layout_and_update() {
        let (size, c) = shown(500.0, |ui| ui.mineral(Probe.preferred_width(120.0)));
        assert_eq!(size, vec2(120.0, Probe::NATURAL.y));
        assert_eq!((c.min.x, c.max.x), (120.0, 120.0));
        assert_eq!(c.max.y, f32::INFINITY, "the other axis keeps the parent's bounds");

        let (size, _) = shown(500.0, |ui| ui.mineral(Probe.preferred_size(vec2(64.0, 64.0))));
        assert_eq!(size, vec2(64.0, 64.0));
        let (size, _) = shown(500.0, |ui| ui.mineral(Probe.width_range(..=150.0).height_range(48.0..)));
        assert_eq!(size, vec2(150.0, 48.0));
        let (size, _) = shown(100.0, |ui| ui.mineral(Probe.preferred_width(120.0)));
        assert_eq!(size.x, 100.0, "the parent's max wins");
        let (size, _) = shown(500.0, |ui| ui.mineral(Probe));
        assert_eq!(size, Probe::NATURAL);
    }

    #[test]
    fn layout_derived_overflow_saturates_instead_of_panicking() {
        let (size, c) = shown(500.0, |ui| ui.mineral(Probe.preferred_width(-23.0)));
        assert_eq!(size.x, 0.0);
        assert_eq!((c.min.x, c.max.x), (0.0, 0.0));
        let (_, c) = shown(500.0, |ui| ui.mineral(Probe.width_range(f32::NAN..=f32::NAN)));
        assert_eq!((c.min.x, c.max.x), (0.0, 0.0));
        let (_, c) = shown(500.0, |ui| ui.mineral(Probe.width_range(80.0..=40.0)));
        assert_eq!((c.min.x, c.max.x), (80.0, 80.0), "max rises to min");
    }

    #[test]
    fn constrained_keeps_identity_and_state() {
        let mut h = harness(
            |ui, (wrapped, seen): &mut (bool, Vec<(egui::Id, u32)>)| {
                let r = if *wrapped {
                    ui.mineral(Probe.preferred_width(120.0))
                } else {
                    ui.mineral(Probe)
                };
                seen.push((r.id, r.output().expect("update ran").1));
            },
            (false, Vec::new()),
        );
        h.state_mut().value.0 = true;
        h.run();
        let seen = &h.state().value.1;
        assert!(
            seen.windows(2).all(|w| w[0].0 == w[1].0),
            "same id with and without the override"
        );
        assert!(
            seen.windows(2).all(|w| w[1].1 == w[0].1 + 1),
            "state carries over: {seen:?}"
        );
    }

    #[test]
    fn interaction_dominance() {
        let all = Interaction {
            hovered: true,
            pressed: true,
            focused: true,
            dragged: true,
            disabled: false,
        };
        assert_eq!(all.dominant(), Some(InteractionKind::Dragged));
        let i = Interaction { dragged: false, ..all };
        assert_eq!(i.dominant(), Some(InteractionKind::Pressed));
        let i = Interaction { pressed: false, ..i };
        assert_eq!(i.dominant(), Some(InteractionKind::Focused));
        let i = Interaction { focused: false, ..i };
        assert_eq!(i.dominant(), Some(InteractionKind::Hovered));
        assert_eq!(Interaction { disabled: true, ..all }.dominant(), None);
        assert_eq!(Interaction::default().dominant(), None);
    }

    #[test]
    fn containers_show_a_mineral_body() {
        let h = harness(
            |ui, seen: &mut Vec<Option<u8>>| {
                seen.clear();
                ui.with_scoped(7_u8, |ui| {
                    let scoped = |ui: &mut MineralUi<'_, TestKit>| ui.scoped::<u8>().copied();
                    seen.push(egui::Frame::NONE.show_in(ui, scoped).inner);
                    seen.push(egui::ScrollArea::vertical().show_in(ui, scoped).inner);
                    seen.push(egui::Grid::new("grid").show_in(ui, scoped).inner);
                    let r = egui::Frame::NONE.show_in(ui, |ui| ui.mineral(Probe));
                    seen.push(r.inner.output().map(|_| 0));
                });
            },
            Vec::new(),
        );
        assert_eq!(h.state().value, [Some(7), Some(7), Some(7), Some(0)]);
    }
}

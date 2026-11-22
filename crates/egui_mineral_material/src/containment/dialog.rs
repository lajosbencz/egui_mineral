use crate::Material;
use crate::MaterialExt;
use crate::Rt;
use crate::msg;
use egui::{Align, Color32, Frame, Id, Layout, Margin, Rangef, Rect, Sense, Ui, UiBuilder, pos2, vec2};
use egui_mineral::action::{self, ActionValue, NoAction, Widen};
use egui_mineral::select::ItemKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    i18n::{Direction, I18n},
    kit::{self, state::TempState},
    layout::{Axis, Constraints, Zones},
    overlay::{self, Dismiss},
    range::{RangeModel, Track},
    semantic::{self, Role, SemanticNode, StableId},
    shortcut::Mnemonic,
    widget::MineralWidget,
};

use crate::actions::{ButtonVariant, PushButton};
use crate::widget::{common, modal_focus::ModalFocus, parts};
use crate::{
    Icon,
    spec::{
        ColorRole, TypeScale, defaults,
        tokens::{app_bar, app_bar_small, dialog},
    },
    tokens::Tokens,
};

// `area` comes from `kit::layer::modal_area`.
pub(crate) fn surface_modal(
    area: egui::Area,
    t: &Tokens,
    fill: Color32,
    radius: egui::CornerRadius,
    margin: f32,
) -> egui::Modal {
    egui::Modal {
        area,
        backdrop_color: t.scrim(1.0),
        frame: Some(Frame::new().fill(fill).corner_radius(radius).inner_margin(margin)),
    }
}

pub(crate) fn dialog_modal(rt: &crate::Rt, id: Id, t: &Tokens, margin: f32) -> egui::Modal {
    surface_modal(
        kit::layer::modal_area(rt.core(), id),
        t,
        t.role(dialog::CONTAINER_COLOR),
        dialog::CONTAINER_SHAPE.corner_radius(&t.shape, Direction::Ltr),
        margin,
    )
}

// Visual order, so Tab visits them in it. Also returns the row's natural size.
fn text_actions_end<T>(
    ui: &mut MineralUi<'_, Material>,
    actions: Vec<(T, PushButton<'_>)>,
    before: impl FnOnce(&Ui),
) -> (Option<T>, egui::Vec2) {
    let rt = ui.rt();
    let size = |b: &PushButton<'_>| {
        let measure = b.layout(ui, Constraints::loose(vec2(ui.available_width(), f32::INFINITY)), &());
        Zones::new(measure, ui.target(b.min_target(ui))).outer.size()
    };
    let buttons: Vec<_> = actions
        .into_iter()
        .map(|(t, b)| (t, b.variant(ButtonVariant::Text)))
        .collect();
    let gaps = buttons.len().saturating_sub(1) as f32 * defaults::dialog::BUTTONS_SPACING;
    let sizes: Vec<egui::Vec2> = buttons.iter().map(|(_, b)| size(b)).collect();
    let total = sizes.iter().map(|s| s.x).sum::<f32>() + gaps;
    let height = sizes.iter().fold(0.0_f32, |h, s| h.max(s.y));
    // Reading order, so Tab follows the visual order in both directions.
    let layout = rt.i18n().direction().row();
    // Bounded to the buttons: a row over the rest of the surface would grow it to its last height.
    let clicked = ui
        .allocate_ui_with_layout(vec2(ui.available_width(), height), layout, |ui| {
            before(ui);
            // A sizing pass measures what the content needs.
            if !ui.is_sizing_pass() {
                let free = (ui.available_width() - total).max(0.0);
                ui.add_space(free);
            }
            crate::actions::text_actions(&mut rt.ui(ui), buttons)
        })
        .inner;
    (clicked, vec2(total, height))
}

fn type_label(ui: &mut MineralUi<'_, Material>, text: &str, scale: TypeScale, color: ColorRole) -> egui::Response {
    let rt = ui.rt();
    let t = rt.tokens();
    let g = parts::type_wrapped(ui.painter(), text, scale, t.role(color), ui.available_width(), 0);
    ui.add(egui::Label::new(g))
}

// MD3 dialog spec: 16dp body spacing
const DIALOG_BODY_GAP: f32 = 16.0;
// M2 dialog spec: 48dp from the screen edges; Compose leaves the inset to the platform window.
const SCREEN_MARGIN: f32 = 48.0;

// Largest outer size of a centered dialog: the screen minus `SCREEN_MARGIN`, never narrower than the
// MD3 minimum width unless the screen is.
pub(crate) fn max_size(ctx: &egui::Context) -> egui::Vec2 {
    let screen = ctx.content_rect().size();
    vec2(
        (screen.x - 2.0 * SCREEN_MARGIN)
            .max(defaults::dialog::MIN_WIDTH)
            .min(screen.x),
        (screen.y - 2.0 * SCREEN_MARGIN).max(0.0),
    )
}

// Shows `add` in place while its last height fits `max_height`, else in a vertical scroll area that
// fills the width (a shrinking one misplaces end-aligned RTL content).
fn scroll_when_taller<R>(ui: &mut Ui, id: Id, max_height: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let height_id = id.with("mineral::full_height");
    let last = ui.ctx().data(|d| d.get_temp::<f32>(height_id));
    let (inner, height) = if last.is_some_and(|h| h > max_height + 0.5) {
        let out = egui::ScrollArea::vertical()
            .id_salt(id)
            .max_height(max_height)
            .auto_shrink([false, true])
            .show(ui, add);
        (out.inner, out.content_size.y)
    } else {
        let out = ui.scope(add);
        (out.inner, out.response.rect.height())
    };
    ui.ctx().data_mut(|d| d.insert_temp(height_id, height));
    inner
}

fn after_gap<R>(
    ui: &mut MineralUi<'_, Material>,
    gap: f32,
    add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
) -> (R, Option<Rect>) {
    let rt = ui.rt();
    let mut max = ui.available_rect_before_wrap();
    max.min.y += gap;
    let layout = *ui.layout();
    let mut child = ui.new_child(UiBuilder::new().max_rect(max).layout(layout));
    let inner = add(&mut rt.ui(&mut child));
    let used = child.min_rect();
    let shown = (used.height() > 0.0).then_some(used);
    if shown.is_some() {
        ui.advance_cursor_after_rect(used);
    }
    (inner, shown)
}

// Content size of the last shown pass; `resize` asks the next one for a sizing pass.
enum WidthChange {
    Set(f32),
    // Back to content sizing.
    Reset,
}

#[derive(Clone, Copy, Default)]
struct DialogSize {
    resize: bool,
    grew_at: Option<u64>,
    height: Option<f32>,
    // Action row height of the last pass, reserved below the scrolling body.
    actions: Option<f32>,
}

impl DialogSize {
    // Shrunk content leaves the width it filled; taller content may wrap wider.
    // Growth within two frames of the last lags a frame instead of discarding again.
    fn measured(self, frame: u64, used: Rect, natural: f32, settled: bool) -> Self {
        let grew = self.height.is_some_and(|h| used.height() > h + 0.5);
        let continued = self.grew_at.is_some_and(|f| frame - f <= 2);
        Self {
            resize: settled && (natural + 0.5 < used.width() || (grew && !continued)),
            grew_at: if grew && settled { Some(frame) } else { self.grew_at },
            height: Some(used.height()),
            actions: self.actions,
        }
    }
}

impl TempState for DialogSize {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Choice {
    Confirm,
    Cancel,
}

pub(crate) struct ChoiceLabels {
    pub confirm: String,
    pub cancel: String,
}

pub(crate) fn confirm_cancel(
    ui: &mut MineralUi<'_, Material>,
    salt: &KeyedSalt,
    labels: &ChoiceLabels,
    can_confirm: bool,
) -> Option<Choice> {
    text_actions_end(
        ui,
        vec![
            (Choice::Cancel, PushButton::new(salt.child("cancel"), &labels.cancel)),
            (
                Choice::Confirm,
                PushButton::new(salt.child("confirm"), &labels.confirm).disabled(!can_confirm),
            ),
        ],
        |_| {},
    )
    .0
}

// App overrides of the picker modal texts; `None` takes the message.
#[derive(Clone, Debug, Default)]
pub(crate) struct PickerLabels {
    pub headline: Option<String>,
    pub confirm: Option<String>,
    pub cancel: Option<String>,
}

impl PickerLabels {
    pub(crate) fn headline(&self, i18n: &I18n, id: crate::Msg) -> String {
        crate::tr_or(i18n, self.headline.as_deref(), id)
    }

    pub(crate) fn choice(&self, i18n: &I18n) -> ChoiceLabels {
        ChoiceLabels {
            confirm: crate::tr_or(i18n, self.confirm.as_deref(), msg::OK),
            cancel: crate::tr_or(i18n, self.cancel.as_deref(), msg::CANCEL),
        }
    }
}

/// `headline`, `confirm_label` and `cancel_label` builders over a `PickerLabels` field path.
macro_rules! picker_label_setters {
    ($($field:ident).+) => {
        /// Modal title in every mode.
        pub fn headline(mut self, headline: impl Into<String>) -> Self {
            self.$($field).+.headline = Some(headline.into());
            self
        }

        /// Confirm button text; default the `mineral-ok` message.
        pub fn confirm_label(mut self, confirm_label: impl Into<String>) -> Self {
            self.$($field).+.confirm = Some(confirm_label.into());
            self
        }

        /// Cancel button text; default the `mineral-cancel` message.
        pub fn cancel_label(mut self, cancel_label: impl Into<String>) -> Self {
            self.$($field).+.cancel = Some(cancel_label.into());
            self
        }
    };
}
pub(crate) use picker_label_setters;

/// Action button of a [`Dialog`]; the dialog pairs it with the app value it reports. A label converts.
#[derive(Clone, Debug)]
#[must_use]
pub struct DialogAction {
    label: String,
    mnemonic: Option<Mnemonic>,
    disabled: bool,
    salt: Option<egui_mineral::semantic::KeyedSalt>,
}

impl DialogAction {
    crate::widget::setters! {
        /// Default the action's value.
        item_salt
    }

    /// `label` is the button text and accessible name.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            mnemonic: None,
            disabled: false,
            salt: None,
        }
    }

    /// Alt+letter activates the action while the dialog is the topmost modal.
    pub fn mnemonic(mut self, mnemonic: impl Into<Mnemonic>) -> Self {
        self.mnemonic = Some(mnemonic.into());
        self
    }
    crate::widget::setters! {
        /// Greys out the button; Enter skips it as the default action.
        disabled
    }

    fn button(&self, salt: KeyedSalt) -> PushButton<'static> {
        let b = PushButton::new(salt, &self.label).disabled(self.disabled);
        match self.mnemonic {
            Some(m) => b.mnemonic(m.char()),
            None => b,
        }
    }
}

/// MD3 basic dialog (Compose `AlertDialog`); modal. [`Dialog::full_screen`] builds the full-screen kind.
#[derive(Clone, Debug)]
#[must_use = "call .show_modal(..)"]
pub struct Dialog<A = NoAction> {
    salt: KeyedSalt,
    id: Id,
    headline: String,
    icon: Option<Icon>,
    supporting: Option<String>,
    actions: Vec<(A, DialogAction)>,
    default_action: Option<A>,
    width: Rangef,
    resizable: bool,
}

/// Result of [`Dialog::show_modal`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DialogResponse<R, A = NoAction> {
    /// `None` while closed.
    pub response: Option<egui::Response>,
    /// Output of the content closure; `None` while closed.
    pub inner: Option<R>,
    /// Value of the clicked action.
    pub clicked: Option<A>,
    /// Why it closed without an action.
    pub dismissed: Option<Dismiss>,
}

impl<R, A> DialogResponse<R, A> {
    fn closed() -> Self {
        Self {
            response: None,
            inner: None,
            clicked: None,
            dismissed: None,
        }
    }
}

// Value-bound modal surface of the pickers: `W` is the working copy, written back on confirm.
pub(crate) struct PickerModal<'a, W> {
    pub(crate) id: Id,
    pub(crate) salt: &'a KeyedSalt,
    pub(crate) disabled: bool,
    pub(crate) initial: W,
}

impl<W: Clone + Send + Sync + 'static> PickerModal<'_, W> {
    // `content` returns what `actions` needs and the dialog name; `actions` returns the pressed action;
    // `pick` reads the value out of `W`.
    #[expect(clippy::too_many_arguments, reason = "the picker's parts")]
    pub(crate) fn show<'n, V: PartialEq, C>(
        self,
        rt: &Rt,
        open: &mut bool,
        value: &mut V,
        pick: impl FnOnce(&W) -> V,
        content: impl FnOnce(&mut MineralUi<'_, Material>, &mut W) -> (C, &'n str),
        actions: impl FnOnce(&mut MineralUi<'_, Material>, &mut W, C) -> Option<Choice>,
        forget: impl Fn(&mut egui::util::IdTypeMap),
    ) -> crate::widget::vocab::PickerModalResponse {
        use crate::widget::vocab::PickerModalResponse;
        let ctx = rt.ctx();
        let (id, work_id, actions_id) = (self.id, self.id.with("work"), self.id.with("actions_h"));
        if self.disabled {
            *open = false;
        }
        let close = |d: &mut egui::util::IdTypeMap| {
            d.remove::<W>(work_id);
            d.remove::<f32>(actions_id);
            forget(d);
        };
        ModalFocus::scope(ctx, focus_id(id), open, None, |_, open| {
            if !*open {
                ctx.data_mut(close);
                return PickerModalResponse::closed();
            }
            let t = rt.tokens();
            let mut work: W = ctx.data(|d| d.get_temp(work_id)).unwrap_or(self.initial);
            let r = dialog_modal(rt, id, &t, 0.0).show(ctx, |ui| {
                // Compose picker dialogs: `usePlatformDefaultWidth = false`, so the screen bounds them; the
                // content takes `weight(1f, fill = false)` and the buttons stay.
                let fit = ui.ctx().content_rect().size();
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                let reserve = ui.ctx().data(|d| d.get_temp::<f32>(actions_id)).unwrap_or_default();
                let (c, name) = scroll_when_taller(ui, id.with("content"), (fit.y - reserve).max(0.0), |ui| {
                    content(&mut rt.ui(ui), &mut work)
                });
                let top = ui.min_rect().bottom();
                let action = actions(&mut rt.ui(ui), &mut work, c);
                let used = ui.min_rect().bottom() - top;
                ui.ctx().data_mut(|d| d.insert_temp(actions_id, used));
                semantic::emit(ui.ctx(), Some(ui.unique_id()), ui.min_rect(), || {
                    SemanticNode::new(StableId::of(self.salt, "dialog"), Role::Dialog, name)
                });
                action.or_else(|| kit::layer::default_action_pressed(ui).then_some(Choice::Confirm))
            });
            let dismissed = if r.inner == Some(Choice::Cancel) {
                Some(Dismiss::Close)
            } else {
                overlay::modal(&r)
            };
            let confirmed = r.inner == Some(Choice::Confirm);
            let changed = confirmed && {
                let picked = pick(&work);
                let changed = picked != *value;
                *value = picked;
                changed
            };
            if confirmed || dismissed.is_some() {
                *open = false;
                ctx.data_mut(close);
            } else {
                ctx.data_mut(|d| d.insert_temp(work_id, work));
            }
            let mut response = r.response;
            if changed {
                response.mark_changed();
            }
            PickerModalResponse {
                response: Some(response),
                changed,
                dismissed,
            }
        })
    }
}

fn run_modal<R, A>(
    id: Id,
    ctx: &egui::Context,
    open: &mut bool,
    run: impl FnOnce(&mut ModalFocus) -> DialogResponse<R, A>,
) -> DialogResponse<R, A> {
    ModalFocus::scope(ctx, focus_id(id), open, None, |focus, open| {
        if !*open {
            return DialogResponse::closed();
        }
        let r = run(focus);
        if r.clicked.is_some() || r.dismissed.is_some() {
            *open = false;
        }
        r
    })
}

impl Dialog {
    /// `headline` is the title and accessible name; agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, headline: impl Into<String>) -> Self {
        let salt: KeyedSalt = id_salt.into();
        Self {
            id: dialog_id(salt.id_salt()),
            salt,
            headline: headline.into(),
            icon: None,
            supporting: None,
            actions: Vec::new(),
            default_action: None,
            width: Rangef::new(defaults::dialog::MIN_WIDTH, defaults::dialog::MAX_WIDTH),
            resizable: false,
        }
    }

    /// Full-screen dialog: a top app bar with a close button, the headline and one confirm action.
    pub fn full_screen(id_salt: impl Into<KeyedSalt>, headline: impl Into<String>) -> FullScreenDialog {
        FullScreenDialog::new(id_salt, headline)
    }
}

impl<A: ActionValue> Dialog<A> {
    /// End-edge handle resizes the dialog within [`Self::width`] (arrow keys when focused);
    /// the chosen width persists in egui memory and replaces content sizing until a double click
    /// on the handle. Default `false`.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Width bounds in dp; the dialog follows its content within them. Default MD3 280..=560.
    /// Equal bounds fix the width; a reversed range is swapped.
    #[track_caller]
    pub fn width(mut self, width: impl Into<Rangef>) -> Self {
        let w = width.into();
        let min = egui_mineral::kit::sanitize::at_least(w.min.min(w.max), 0.0.., "width");
        self.width = Rangef::new(min, w.max.max(w.min).max(min));
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Hero icon above the headline.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Body text below the headline.
    pub fn supporting(mut self, supporting: impl Into<String>) -> Self {
        self.supporting = Some(supporting.into());
        self
    }

    fn action_salt(&self, index: usize) -> KeyedSalt {
        self.actions.get(index).map_or_else(
            || self.salt.child("action"),
            |(v, a)| crate::widget::action_salt(&self.salt, a.salt.as_ref(), None, v).child("action"),
        )
    }

    /// Appends the text button reporting `value` to the action row.
    pub fn action<B: ItemKey>(self, value: B, action: impl Into<DialogAction>) -> Dialog<B>
    where
        A: Widen<B>,
    {
        self.actions([(value, action)])
    }

    /// Appends `(value, text button)` pairs; labels convert into [`DialogAction`].
    pub fn actions<B: ItemKey>(self, actions: impl IntoIterator<Item = (B, impl Into<DialogAction>)>) -> Dialog<B>
    where
        A: Widen<B>,
    {
        let mut dialog = Dialog {
            salt: self.salt,
            id: self.id,
            headline: self.headline,
            icon: self.icon,
            supporting: self.supporting,
            actions: action::widen(self.actions),
            default_action: self.default_action.map(Widen::widen),
            width: self.width,
            resizable: self.resizable,
        };
        dialog.actions.extend(actions.into_iter().map(|(v, a)| (v, a.into())));
        dialog
    }

    /// Value of the action that Enter triggers, see [`Self::show_modal`]; a value of no action sets none.
    pub fn default_action(mut self, value: A) -> Self {
        self.default_action = Some(value);
        self
    }

    /// Shown while `open`; an action click or a dismissal closes it. Unmodified Enter reports the enabled
    /// default action unless focus is on a widget that handles Enter itself: a multi-line text edit or a
    /// button (which activates instead). Enter in a single-line text edit triggers it.
    ///
    /// Opening focuses the default action, else the first enabled action; closing returns focus to the
    /// widget focused before opening.
    pub fn show_modal<R>(
        mut self,
        rt: &Rt,
        open: &mut bool,
        add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
    ) -> DialogResponse<R, A> {
        let ctx = rt.ctx();
        self.id = kit::scoped_id(rt.core(), self.id);
        run_modal(self.id, ctx, open, |focus| self.run(rt, focus, add))
    }

    fn default_index(&self) -> Option<usize> {
        let value = self.default_action.as_ref()?;
        self.actions.iter().position(|(v, _)| v == value)
    }

    fn run<R>(
        self,
        rt: &Rt,
        focus: &mut ModalFocus,
        add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
    ) -> DialogResponse<R, A> {
        use defaults::dialog as d;
        let ctx = rt.ctx();
        let t = rt.tokens();
        let mut action = None;
        let size_id = self.id.with("mineral::dialog_size");
        let mut size = DialogSize::load(ctx, size_id);
        let width_id = self.id.with("mineral::dialog_width");
        let chosen = if self.resizable {
            ctx.data_mut(|d| d.get_persisted::<f32>(width_id))
                .map(|w| self.width.clamp(w))
        } else {
            None
        };
        let fit = max_size(ctx);
        let bounds = chosen.map_or(self.width, Rangef::point);
        let bounds = Rangef::new(bounds.min.min(fit.x), bounds.max.min(fit.x));
        let mut resized = None;
        // Pass 0 only: its discard reruns the frame visibly at the measured size.
        let resize = size.resize && ctx.current_pass_index() == 0;
        if resize {
            ctx.request_discard("dialog content resized");
        }
        let mut modal = dialog_modal(rt, self.id, &t, d::PADDING);
        modal.area = modal.area.default_width(bounds.max).sizing_pass(resize);
        let r = modal.show(ctx, |ui| {
            let min = (bounds.min - 2.0 * d::PADDING).max(0.0);
            ui.set_min_width(min);
            // Caps only: a wider max rect would let the action row fill past the measured width.
            ui.set_max_width(ui.max_rect().width().min(bounds.max - 2.0 * d::PADDING).max(min));
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            let mut natural = min.max(self.head(&mut rt.ui(ui), &t));
            ui.add_space(d::TITLE_BOTTOM_PADDING);
            // Compose AlertDialog: the text takes `weight(1f, fill = false)`, so the body shrinks and
            // scrolls while the headline and the actions stay.
            let actions_h = if self.actions.is_empty() {
                0.0
            } else {
                size.actions
                    .unwrap_or_else(|| crate::adaptive::min_interactive(rt.kit().density()))
            };
            let reserve = actions_h + d::TEXT_BOTTOM_PADDING;
            let body_max = (fit.y - 2.0 * d::PADDING - ui.min_rect().height() - reserve).max(0.0);
            let (inner, body_natural, body) = self.body(&mut rt.ui(ui), body_max, add);
            natural = natural.max(body_natural);
            if body {
                ui.add_space(d::TEXT_BOTTOM_PADDING);
            }
            if !self.actions.is_empty() {
                let initial = self
                    .default_index()
                    .or_else(|| self.actions.iter().position(|(_, a)| !a.disabled));
                let row;
                (action, row) = text_actions_end(
                    &mut rt.ui(ui),
                    self.actions
                        .iter()
                        .enumerate()
                        .map(|(idx, (_, a))| (idx, a.button(self.action_salt(idx))))
                        .collect(),
                    |row| {
                        if let Some(i) = initial {
                            focus.enter(ctx, row.make_persistent_id(self.action_salt(i).id_salt()));
                        }
                    },
                );
                natural = natural.max(row.x);
                size.actions = Some(row.y);
            }
            action = action.or_else(|| self.default_on_enter(ui));
            let used = ui.min_rect();
            size = size.measured(
                ctx.cumulative_frame_nr(),
                used,
                natural,
                !ui.is_sizing_pass() && chosen.is_none(),
            );
            if self.resizable && !ui.is_sizing_pass() {
                resized = self.resize_handle(&rt.ui(ui), used.expand(d::PADDING));
            }
            inner
        });
        match resized {
            Some(WidthChange::Set(w)) => ctx.data_mut(|d| d.insert_persisted(width_id, w)),
            Some(WidthChange::Reset) => ctx.data_mut(|d| d.remove::<f32>(width_id)),
            None => {}
        }
        if size.resize {
            #[expect(clippy::disallowed_methods, reason = "one pass to measure the changed content")]
            ctx.request_repaint();
        }
        size.store(ctx, size_id);
        let dismissed = overlay::modal(&r);
        DialogResponse {
            response: Some(r.response),
            inner: Some(r.inner),
            clicked: action.and_then(|i| self.actions.get(i)).map(|(v, _)| v.clone()),
            dismissed,
        }
    }

    // Supporting text and content, scrolling past `max_height`; returns the content's result, natural
    // width and whether anything showed.
    fn body<R>(
        &self,
        ui: &mut MineralUi<'_, Material>,
        max_height: f32,
        add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
    ) -> (R, f32, bool) {
        let rt = ui.rt();
        scroll_when_taller(ui, self.id.with("body"), max_height, |ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            let mut natural: f32 = 0.0;
            let mut text = false;
            if let Some(s) = &self.supporting {
                let label = type_label(
                    &mut rt.ui(ui),
                    s,
                    dialog::SUPPORTING_TEXT_FONT,
                    dialog::SUPPORTING_TEXT_COLOR,
                );
                natural = label.rect.width();
                text = true;
            }
            let gap = if text { DIALOG_BODY_GAP } else { 0.0 };
            let (inner, content) = after_gap(&mut rt.ui(ui), gap, add);
            if let Some(used) = content {
                natural = natural.max(used.width());
            }
            (inner, natural, text || content.is_some())
        })
    }

    // End edge handle; the centered dialog grows on both sides.
    fn resize_handle(&self, ui: &MineralUi<'_, Material>, frame: Rect) -> Option<WidthChange> {
        let rt = ui.rt();
        let dir = rt.i18n().direction();
        let edge = dir.reversed().start_x(frame, Rangef::point(0.0));
        let hit = crate::adaptive::resize_edge(ui);
        let rect = Rect::from_x_y_ranges(edge - hit / 2.0..=edge + hit / 2.0, frame.y_range());
        let range = RangeModel::new(f64::from(self.width.min), f64::from(self.width.max));
        let width = frame.width();
        let egui_mineral::splitter::SizeHandleResponse {
            response: handle,
            size: v,
            ..
        } = egui_mineral::splitter::SizeHandle::new(self.id.with("resize"), rect, Track::horizontal(dir), range)
            .centered(true)
            .show(ui, f64::from(width));
        let i = common::Interaction::of(&handle, egui_mineral::widget::Availability::Enabled);
        if i.hovered || i.focused || i.dragged {
            let center = pos2(edge, frame.center().y);
            crate::containment::drag_handle::paint(ui, &handle, center, Axis::Horizontal, false);
        }
        let label = crate::tr(rt.i18n(), msg::DIALOG_RESIZE);
        semantic::emit(ui.ctx(), Some(handle.id), rect, || {
            SemanticNode::new(StableId::of(&self.salt, "resize"), Role::Splitter, label)
                .ranged(range.numeric(v), egui_mineral::widget::Availability::Enabled)
        });
        if handle.double_clicked() {
            return Some(WidthChange::Reset);
        }
        #[expect(clippy::cast_possible_truncation, reason = "within the f32 width range")]
        let v = v as f32;
        ((v - width).abs() > 0.5).then_some(WidthChange::Set(v))
    }

    // Icon and headline; returns their natural width.
    fn head(&self, ui: &mut MineralUi<'_, Material>, t: &Tokens) -> f32 {
        let rt = ui.rt();
        // Compose centers the icon and, with an icon, the headline.
        let layout = if self.icon.is_some() {
            Layout::top_down(Align::Center)
        } else {
            rt.i18n().direction().column()
        };
        ui.with_layout(layout, |ui| {
            let mut width = 0.0;
            if let Some(icon) = self.icon {
                let size = dialog::ICON_SIZE;
                let (r, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
                common::icon(ui.painter(), r.center(), icon, size, t.role(dialog::ICON_COLOR));
                ui.add_space(defaults::dialog::ICON_BOTTOM_PADDING);
                width = size;
            }
            let label = type_label(
                &mut rt.ui(ui),
                &self.headline,
                dialog::HEADLINE_FONT,
                dialog::HEADLINE_COLOR,
            );
            label.rect.width().max(width)
        })
        .inner
    }

    fn default_on_enter(&self, ui: &Ui) -> Option<usize> {
        let i = self.default_index()?;
        (self.actions.get(i).is_some_and(|(_, a)| !a.disabled) && kit::layer::default_action_pressed(ui)).then_some(i)
    }
}

/// Full-screen kind of [`Dialog`]: a small top app bar (close button, headline, text confirm action)
/// over the content; modal.
#[derive(Clone, Debug)]
#[must_use = "call .show_modal(..)"]
pub struct FullScreenDialog<A = NoAction> {
    salt: KeyedSalt,
    id: Id,
    headline: String,
    supporting: Option<String>,
    confirm: Option<(A, DialogAction)>,
    enter_confirms: bool,
}

impl FullScreenDialog {
    fn new(id_salt: impl Into<KeyedSalt>, headline: impl Into<String>) -> Self {
        let salt: KeyedSalt = id_salt.into();
        Self {
            id: dialog_id(salt.id_salt()),
            salt,
            headline: headline.into(),
            supporting: None,
            confirm: None,
            enter_confirms: false,
        }
    }
}

impl<A: ActionValue> FullScreenDialog<A> {
    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Body text above the content.
    pub fn supporting(mut self, supporting: impl Into<String>) -> Self {
        self.supporting = Some(supporting.into());
        self
    }

    /// Text button at the end of the header, reporting `value` in `clicked`.
    pub fn confirm<B: ItemKey>(self, value: B, action: impl Into<DialogAction>) -> FullScreenDialog<B> {
        FullScreenDialog {
            salt: self.salt,
            id: self.id,
            headline: self.headline,
            supporting: self.supporting,
            confirm: Some((value, action.into())),
            enter_confirms: self.enter_confirms,
        }
    }

    /// Unmodified Enter triggers an enabled confirm action, see [`Dialog::show_modal`].
    pub fn enter_confirms(mut self, enter_confirms: bool) -> Self {
        self.enter_confirms = enter_confirms;
        self
    }

    /// Shown while `open`; confirming or closing dismisses it. Opening focuses the close button;
    /// closing returns focus to the widget focused before opening.
    pub fn show_modal<R>(
        mut self,
        rt: &Rt,
        open: &mut bool,
        add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
    ) -> DialogResponse<R, A> {
        let ctx = rt.ctx();
        self.id = kit::scoped_id(rt.core(), self.id);
        run_modal(self.id, ctx, open, |focus| self.run(rt, focus, add))
    }

    fn confirm_on_enter(&self, ui: &Ui) -> Option<usize> {
        let enabled = self.confirm.as_ref().is_some_and(|(_, a)| !a.disabled);
        (self.enter_confirms && enabled && kit::layer::default_action_pressed(ui)).then_some(0)
    }

    fn run<R>(
        self,
        rt: &Rt,
        focus: &mut ModalFocus,
        add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
    ) -> DialogResponse<R, A> {
        use defaults::dialog as d;
        let ctx = rt.ctx();
        let t = rt.tokens();
        let screen = ctx.content_rect();
        let mut action = None;
        let mut closed = false;
        // Compose TopAppBar in a Dialog (MD3 full-screen dialog)
        let r = egui_mineral::kit::layer::modal(rt.core(), self.id)
            .area(
                egui_mineral::kit::layer::modal_area(rt.core(), self.id)
                    .anchor(egui::Align2::LEFT_TOP, egui::Vec2::ZERO),
            )
            .frame(Frame::new().fill(t.role(app_bar::CONTAINER_COLOR)))
            .backdrop_color(egui::Color32::TRANSPARENT)
            .show(ctx, |ui| {
                ui.set_min_size(screen.size());
                ui.set_max_size(screen.size());
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                let head = self.header(ui, rt, focus, &t, screen.width());
                closed = head.closed;
                action = head.action;
                let margin = Margin::from(vec2(d::PADDING, DIALOG_BODY_GAP));
                let inner = egui::ScrollArea::vertical()
                    .id_salt(self.id.with("scroll"))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        Frame::new()
                            .inner_margin(margin)
                            .show(ui, |ui| {
                                ui.set_width(screen.width() - 2.0 * d::PADDING);
                                ui.spacing_mut().item_spacing = egui::Vec2::splat(crate::adaptive::grid(2));
                                if let Some(s) = &self.supporting {
                                    type_label(
                                        &mut rt.ui(ui),
                                        s,
                                        dialog::SUPPORTING_TEXT_FONT,
                                        dialog::SUPPORTING_TEXT_COLOR,
                                    );
                                    ui.add_space(DIALOG_BODY_GAP);
                                }
                                add(&mut rt.ui(ui))
                            })
                            .inner
                    })
                    .inner;
                action = action.or_else(|| self.confirm_on_enter(ui));
                inner
            });
        let dismissed = if closed {
            Some(Dismiss::Close)
        } else {
            overlay::modal(&r)
        };
        DialogResponse {
            response: Some(r.response),
            inner: Some(r.inner),
            clicked: action.and(self.confirm.as_ref()).map(|(v, _)| v.clone()),
            dismissed,
        }
    }

    fn header(&self, ui: &mut Ui, rt: &Rt, focus: &mut ModalFocus, t: &Tokens, width: f32) -> Header {
        use defaults::app_bar as bar;
        let ctx = rt.ctx();
        let dir = rt.i18n().direction();
        let (head, _) = ui.allocate_exact_size(vec2(width, app_bar_small::CONTAINER_HEIGHT), Sense::hover());
        // Logical x of a `w` wide part `from_start` into the bar.
        let x = |from_start: f32, w: f32| dir.start_x(head, Rangef::new(from_start, from_start + w));
        // A modal root: the app density.
        let slot = crate::adaptive::min_interactive(rt.kit().density());
        let close = parts::action_slot(dir, slot, head, app_bar::LEADING_SPACE);
        let layout = *ui.layout();
        let closed = egui_mineral::layout::child(&mut rt.ui(ui), "close", close, layout, |ui| {
            let salt = self.salt.child("close");
            focus.enter(ctx, ui.make_persistent_id(salt.id_salt()));
            parts::close_button(ui, salt)
        })
        .inner;
        let mut action = None;
        let title_start = app_bar::LEADING_SPACE + slot + bar::TITLE_HORIZONTAL_PADDING;
        let mut title_end = app_bar::TRAILING_SPACE + bar::TITLE_HORIZONTAL_PADDING;
        if let Some((v, a)) = &self.confirm {
            // The end half of the bar, inside the trailing space.
            let w = head.width() / 2.0 - app_bar::TRAILING_SPACE;
            let x0 = dir
                .reversed()
                .start_x(head, Rangef::new(app_bar::TRAILING_SPACE, app_bar::TRAILING_SPACE + w));
            let slot = Rect::from_min_size(pos2(x0, head.top()), vec2(w, head.height()));
            let layout = dir.reversed().row();
            let resp = egui_mineral::layout::child(&mut rt.ui(ui), "confirm", slot, layout, |ui| {
                let own = crate::widget::action_salt(&self.salt, a.salt.as_ref(), None, v).child("action");
                ui.mineral(a.button(own).variant(ButtonVariant::Text))
            });
            if resp.inner.clicked() {
                action = Some(0);
            }
            title_end += resp.inner.rect.width() + app_bar::TRAILING_SPACE;
        }
        let g = parts::type_wrapped(
            ui.painter(),
            &self.headline,
            app_bar_small::TITLE_FONT,
            t.role(app_bar::TITLE_COLOR),
            (head.width() - title_start - title_end).max(0.0),
            1,
        );
        ui.painter().galley(
            pos2(x(title_start, g.size().x), head.center().y - g.size().y / 2.0),
            g,
            Color32::PLACEHOLDER,
        );
        Header { closed, action }
    }
}

struct Header {
    closed: bool,
    action: Option<usize>,
}

fn focus_id(base: egui::Id) -> egui::Id {
    base.with("focus")
}

fn dialog_id(salt: egui::IdSalt) -> Id {
    Id::unique(("mineral::dialog", salt))
}

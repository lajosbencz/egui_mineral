//! Overlay plumbing without painting: dismissal reasons, outside presses, Escape, the dialog
//! default action and the anchored [`Popup`] lifecycle. Stylekits supply frames and shapes.

use crate::kit::state::TempState;
use egui::{
    Context, Id, IdSalt, InnerResponse, LayerId, Memory, ModalResponse, Rect, Response, Vec2,
    accesskit::{HasPopup, Role},
};

use crate::{
    anchor::{Anchor, Placement, Surroundings},
    input::Gesture,
    runtime::{Core, Runtime},
    semantic::KeyedSalt,
    widget::MineralUi,
};

/// Why an overlay closed without completing; overlay responses report it as `dismissed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Dismiss {
    /// Escape key while the overlay or its content has focus.
    Escape,
    /// Press outside the surface or on its scrim.
    Outside,
    /// Close, cancel or toggle control of the overlay, or `Ui::close` from its content.
    Close,
    /// Drag handle fling, or its keyboard step below the lowest detent.
    Swipe,
    /// Focus moved out of the surface, e.g. Tab from a menu.
    FocusOut,
}

/// [`ModalResponse::should_close`] with its reason; consumes Escape the same way.
pub fn modal<T>(r: &ModalResponse<T>) -> Option<Dismiss> {
    if r.backdrop_response.clicked() {
        Some(Dismiss::Outside)
    } else if r.response.should_close() {
        Some(Dismiss::Close)
    } else if r.is_top_modal && !r.any_popup_open && crate::kit::overlay::consume_escape(&r.response.ctx) {
        Some(Dismiss::Escape)
    } else {
        None
    }
}

/// Pointer press or touch tap this pass outside every rect of `inside`.
pub fn outside_press(core: &crate::runtime::Core, inside: &[Rect]) -> bool {
    let ctx = core.ctx();
    let press = ctx.input(|i| i.pointer.any_pressed().then(|| i.pointer.press_origin()).flatten());
    let tap = core.input().gestures().find_map(|g| match g {
        Gesture::Tap { pos } => Some(*pos),
        _ => None,
    });
    press.or(tap).is_some_and(|p| !inside.iter().any(|r| r.contains(p)))
}

#[derive(Clone, Default)]
struct DeclaredModals {
    pass: u64,
    layers: Vec<LayerId>,
}

fn declared_modals_id(ctx: &Context) -> Id {
    Id::unique(("mineral::declared_modals", ctx.viewport_id()))
}

/// Marks `layer` as a modal shown this pass; egui reports it as [`top_modal`] only from the next pass.
pub fn declare_modal(ctx: &Context, layer: LayerId) {
    let (pass, id) = (ctx.cumulative_pass_nr(), declared_modals_id(ctx));
    ctx.data_mut(|d| {
        let m = d.get_temp_mut_or_default::<DeclaredModals>(id);
        if m.pass != pass {
            *m = DeclaredModals {
                pass,
                layers: Vec::new(),
            };
        }
        if !m.layers.contains(&layer) {
            m.layers.push(layer);
        }
    });
}

/// `layer` is [`top_modal`] or was passed to [`declare_modal`] this pass.
pub fn is_modal(ctx: &Context, layer: LayerId) -> bool {
    let (pass, id) = (ctx.cumulative_pass_nr(), declared_modals_id(ctx));
    top_modal(ctx) == Some(layer)
        || ctx.data(|d| {
            d.get_temp::<DeclaredModals>(id)
                .is_some_and(|m| m.pass == pass && m.layers.contains(&layer))
        })
}

/// Topmost `egui::Modal` layer as of the previous pass, the one egui routes input to; the single source for
/// [`crate::shortcut::scope_of`] and [`crate::window::ModalStack::top`].
pub fn top_modal(ctx: &Context) -> Option<LayerId> {
    ctx.memory(Memory::top_modal_layer)
}

/// Foreground area anchored to a trigger, sized by its last shown size: opens with initial
/// focus, closes on Escape (returning focus) and outside presses, and marks the trigger
/// expanded. Paints nothing; the content draws its own surface.
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct Popup {
    salt: IdSalt,
    anchor: Anchor,
    role: Option<Role>,
    name: Option<String>,
    focus: Option<Id>,
    focus_first: bool,
    return_focus: Option<Id>,
    size: Option<Vec2>,
    anchor_rect: Option<Rect>,
}

/// Placed popup contents with the [`Placement`] they landed on.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PopupArea<R> {
    /// Rect, side and arrow the popup landed on.
    pub placement: Placement,
    /// Area response of the popup.
    pub response: Response,
    /// Content closure result.
    pub inner: R,
}

/// Result of showing an overlay (popup, sheet, panel); `dismissed` is why it closed.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct OverlayResponse<R> {
    /// Surface response, `Some` on passes the overlay showed (the dismissing one included).
    pub response: Option<Response>,
    /// Content closure result; `Some` on passes the overlay showed.
    pub inner: Option<R>,
    /// `Some` on the pass the overlay closed without completing.
    pub dismissed: Option<Dismiss>,
}

impl<R> OverlayResponse<R> {
    /// Shown this pass as `response`; `dismissed` when it closed on it.
    pub fn shown(response: Response, inner: Option<R>, dismissed: Option<Dismiss>) -> Self {
        Self {
            response: Some(response),
            inner,
            dismissed,
        }
    }

    /// Still showing after this pass.
    pub fn is_open(&self) -> bool {
        self.response.is_some() && self.dismissed.is_none()
    }

    /// Nothing shown this pass.
    pub fn closed() -> Self {
        Self {
            response: None,
            inner: None,
            dismissed: None,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct PopupState {
    shown: Option<u64>,
    // Pass whose content is the first settled one: the opening pass sizes the area with disabled widgets.
    settle: Option<u64>,
}

impl TempState for PopupState {}

impl Popup {
    /// `id_salt` names the area: under the trigger's id in [`Self::show`], app-wide in [`Self::show_at`].
    pub fn new(id_salt: impl Into<KeyedSalt>, anchor: Anchor) -> Self {
        Self {
            salt: id_salt.into().id_salt(),
            anchor,
            role: None,
            name: None,
            focus: None,
            focus_first: false,
            return_focus: None,
            size: None,
            anchor_rect: None,
        }
    }

    /// AccessKit role of the area; `Menu`, `ListBox`, `Tree`, `Grid` and `Dialog` also set the
    /// trigger's `has_popup`.
    pub fn role(mut self, role: Role) -> Self {
        self.role = Some(role);
        self
    }

    /// Accessible name of the area.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Widget focused when the popup opens; the content creates it.
    pub fn focus(mut self, id: Id) -> Self {
        self.focus = Some(id);
        self
    }

    /// On opening, focus the first enabled focusable widget the content shows, if any, else [`focus`](Self::focus);
    /// default `false`.
    pub fn focus_first(mut self, focus_first: bool) -> Self {
        self.focus_first = focus_first;
        self
    }

    /// Widget focused after Escape while focus was in the popup; the trigger by default.
    pub fn return_focus(mut self, return_focus: Id) -> Self {
        self.return_focus = Some(return_focus);
        self
    }

    /// Known size, instead of the size the area had when last shown; a negative or non-finite
    /// component counts as 0.
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = Some(egui::vec2(
            crate::kit::sanitize::at_least(size.x, 0.0.., "Popup size.x"),
            crate::kit::sanitize::at_least(size.y, 0.0.., "Popup size.y"),
        ));
        self
    }

    /// Rect placed against and exempt from outside presses, instead of the trigger's rect.
    pub fn anchor_rect(mut self, anchor_rect: Rect) -> Self {
        self.anchor_rect = Some(anchor_rect);
        self
    }

    /// Places and shows the area against `anchor_rect` with no open state, dismissal or trigger.
    pub fn show_at<K, R>(
        self,
        rt: &Runtime<K>,
        anchor_rect: Rect,
        add: impl FnOnce(&mut MineralUi<'_, K>, &Placement) -> R,
    ) -> PopupArea<R> {
        let id = crate::kit::scoped_id(rt.core(), Id::unique(("mineral::popup", self.salt)));
        let parent = top_modal(rt.ctx());
        self.place(rt, id, anchor_rect, parent, add)
    }

    fn place<K, R>(
        &self,
        rt: &Runtime<K>,
        id: Id,
        anchor_rect: Rect,
        parent: Option<LayerId>,
        add: impl FnOnce(&mut MineralUi<'_, K>, &Placement) -> R,
    ) -> PopupArea<R> {
        let ctx = rt.ctx();
        let size = self
            .size
            .or_else(|| ctx.memory(|m| m.area_rect(id)).map(|r| r.size()))
            .unwrap_or(Vec2::ZERO);
        let placement = self.anchor.place(
            anchor_rect,
            size,
            Surroundings {
                viewport: ctx.content_rect(),
                direction: rt.core().i18n().direction(),
            },
        );
        let mut area = crate::kit::overlay::floating(rt.core(), id, placement.rect.min);
        if let Some(role) = self.role {
            area = area.role(role);
        }
        if let Some(name) = &self.name {
            area = area.accessible_name(name.clone());
        }
        let InnerResponse { inner, response } = area.show(ctx, |ui| add(&mut rt.ui(ui), &placement));
        if let Some(parent) = parent {
            crate::kit::overlay::keep_above(ctx, response.layer_id, parent);
        }
        PopupArea {
            placement,
            response,
            inner,
        }
    }

    /// Shows the content against `anchor_rect` while `open`, which the app owns: no trigger, so
    /// nothing toggles it. Clears `open` on Escape or an outside press (`anchor_rect` is exempt).
    pub fn show_open_at<K, R>(
        self,
        rt: &Runtime<K>,
        anchor_rect: Rect,
        open: &mut bool,
        add: impl FnOnce(&mut MineralUi<'_, K>, &Placement) -> R,
    ) -> OverlayResponse<R> {
        let id = crate::kit::scoped_id(rt.core(), Id::unique(("mineral::popup", self.salt)));
        let parent = top_modal(rt.ctx());
        self.show_open(rt, id, anchor_rect, parent, self.return_focus, open, add)
    }

    #[expect(clippy::too_many_arguments, reason = "shared body of the trigger and anchor forms")]
    fn show_open<K, R>(
        &self,
        rt: &Runtime<K>,
        id: Id,
        anchor_rect: Rect,
        parent: Option<LayerId>,
        return_focus: Option<Id>,
        open: &mut bool,
        add: impl FnOnce(&mut MineralUi<'_, K>, &Placement) -> R,
    ) -> OverlayResponse<R> {
        let ctx = rt.ctx();
        let state_id = id.with("mineral::popup");
        let mut out = OverlayResponse {
            response: None,
            inner: None,
            dismissed: None,
        };
        if *open {
            let pass = ctx.cumulative_pass_nr();
            let mut st = PopupState::load(ctx, state_id);
            let opened = st.shown.is_none_or(|p| p + 1 < pass);
            if opened {
                st.settle = self.focus_first.then_some(pass + 1);
            }
            let area = self.place(rt, id, anchor_rect, parent, add);
            let mut request = opened.then_some(self.focus).flatten();
            if st.settle == Some(pass) {
                st.settle = None;
                let kept = self.focus.is_none_or(|f| ctx.memory(|m| m.has_focus(f)));
                if let Some(first) = first_focusable(ctx, area.response.layer_id, self.focus).filter(|_| kept) {
                    request = Some(first);
                }
            }
            if let Some(f) = request {
                ctx.memory_mut(|m| m.request_focus(f));
            }
            out.dismissed = dismissal(
                rt.core(),
                area.response.layer_id,
                &[area.response.rect, anchor_rect],
                return_focus,
            );
            *open = out.dismissed.is_none();
            PopupState {
                shown: Some(pass),
                ..st
            }
            .store(ctx, state_id);
            out.response = Some(area.response);
            out.inner = Some(area.inner);
        }
        out
    }

    /// Shows the content while `open`; clears `open` on Escape or an outside press.
    pub fn show<K, R>(
        self,
        rt: &Runtime<K>,
        trigger: &Response,
        open: &mut bool,
        add: impl FnOnce(&mut MineralUi<'_, K>, &Placement) -> R,
    ) -> OverlayResponse<R> {
        let ctx = rt.ctx();
        let id = trigger.id.with_salt(self.salt);
        let out = self.show_open(
            rt,
            id,
            self.anchor_rect.unwrap_or(trigger.rect),
            Some(trigger.layer_id),
            Some(self.return_focus.unwrap_or(trigger.id)),
            open,
            add,
        );
        let has_popup = self.role.and_then(has_popup);
        let expanded = *open;
        ctx.accesskit_node_builder(trigger.id, |b| {
            b.set_expanded(expanded);
            if let Some(h) = has_popup {
                b.set_has_popup(h);
            }
        });
        out
    }
}

/// Why a non-modal overlay on `layer` closes this pass: Escape (consumed) or a press outside every rect of `inside`.
/// A press in the frame the overlay opened (code may open it in the same pass as a press) is not an outside press.
/// Escape is left to a modal above `layer`.
/// On Escape, focus held outside `layer` last pass stays there; otherwise it goes to `return_focus`.
/// Call once per pass while the overlay is open: the opening pass is the first after a gap in calls.
pub fn dismissal(core: &Core, layer: LayerId, inside: &[Rect], return_focus: Option<Id>) -> Option<Dismiss> {
    let ctx = core.ctx();
    let opening = opening_frame(ctx, layer.id);
    let last_key = layer.id.with("dismissal-focus");
    // egui clears focus on Escape before any widget runs.
    let last = Option::<Id>::load(ctx, last_key);
    let above_modal = ctx.memory(|m| m.is_above_modal_layer(layer));
    let dismiss = if above_modal && crate::kit::overlay::consume_escape(ctx) {
        let to = last
            .filter(|&f| ctx.read_response(f).is_some_and(|r| r.layer_id != layer))
            .or(return_focus);
        if let Some(id) = to {
            ctx.memory_mut(|m| m.request_focus(id));
        }
        Some(Dismiss::Escape)
    } else if !opening && outside_press(core, inside) {
        Some(Dismiss::Outside)
    } else {
        None
    };
    if dismiss.is_some() {
        Option::<Id>::remove(ctx, last_key);
    } else {
        let focused = ctx.memory(Memory::focused);
        focused.store(ctx, last_key);
    }
    dismiss
}

/// [`outside_press`] that ignores the frame in which the overlay `id` opened. Same call contract as [`dismissal`].
pub fn outside_press_after_open(core: &Core, id: Id, inside: &[Rect]) -> bool {
    !opening_frame(core.ctx(), id) && outside_press(core, inside)
}

#[derive(Clone, Copy, Default)]
struct OpenedAt {
    pass: Option<u64>,
    frame: Option<u64>,
}

impl TempState for OpenedAt {}

// True in the frame of the first pass after a gap in calls for `id`.
fn opening_frame(ctx: &Context, id: Id) -> bool {
    let key = id.with("mineral::opened-at");
    let (pass, frame) = (ctx.cumulative_pass_nr(), ctx.cumulative_frame_nr());
    let mut st = OpenedAt::load(ctx, key);
    if st.pass != Some(pass) {
        if st.pass.is_none_or(|p| p + 1 < pass) {
            st.frame = Some(frame);
        }
        st.pass = Some(pass);
        st.store(ctx, key);
    }
    st.frame == Some(frame)
}

// First enabled focusable widget of `layer` this pass, other than `skip`.
fn first_focusable(ctx: &Context, layer: LayerId, skip: Option<Id>) -> Option<Id> {
    ctx.viewport(|v| {
        v.this_pass
            .widgets
            .get_layer(layer)
            .find(|w| w.enabled && w.sense.is_focusable() && Some(w.id) != skip)
            .map(|w| w.id)
    })
}

fn has_popup(role: Role) -> Option<HasPopup> {
    match role {
        Role::Menu => Some(HasPopup::Menu),
        Role::ListBox => Some(HasPopup::Listbox),
        Role::Tree => Some(HasPopup::Tree),
        Role::Grid => Some(HasPopup::Grid),
        Role::Dialog => Some(HasPopup::Dialog),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use egui::{Id, Key, Modifiers, Rect, Sense, pos2, vec2};
    use egui_kittest::Harness;

    use super::{Dismiss, OverlayResponse, Popup};
    use crate::{
        anchor::{Align, Anchor, Side},
        input::synth::key_down,
        test_support::{Run, harness, press},
    };

    struct State {
        open: bool,
        out: Option<OverlayResponse<()>>,
    }

    #[test]
    fn popup_escape_and_outside_dismiss() {
        let mut h = harness(
            |ui, s: &mut State| {
                let trigger = ui.interact(
                    Rect::from_min_size(pos2(10.0, 10.0), vec2(40.0, 20.0)),
                    Id::unique("trigger"),
                    Sense::click(),
                );
                let popup = Popup::new("popup", Anchor::new(Side::Bottom, Align::Start));
                let out = popup.show(ui.rt(), &trigger, &mut s.open, |ui, _| {
                    ui.allocate_exact_size(vec2(100.0, 50.0), Sense::hover());
                });
                s.out = Some(out);
            },
            State { open: true, out: None },
        );
        let dismissed = |h: &Harness<'_, Run<State>>| h.state().value.out.as_ref().and_then(|o| o.dismissed);
        let open = |h: &Harness<'_, Run<State>>| h.state().value.open;
        assert!(dismissed(&h).is_none() && open(&h));
        h.event(key_down(Key::Escape, Modifiers::NONE));
        h.step();
        assert_eq!(dismissed(&h), Some(Dismiss::Escape));
        assert!(!open(&h));

        h.state_mut().value.open = true;
        h.run();
        let area = Id::unique("trigger").with("popup");
        let popup = h.ctx.memory(|m| m.area_rect(area)).expect("popup shown");
        h.input_mut().events.extend(press(popup.center()));
        h.step();
        assert!(
            dismissed(&h).is_none() && open(&h),
            "a press in the popup keeps it open"
        );
        let outside = h.ctx.content_rect().max - vec2(1.0, 1.0);
        h.input_mut().events.extend(press(outside));
        h.step();
        assert_eq!(dismissed(&h), Some(Dismiss::Outside));
        assert!(!open(&h));
    }

    #[test]
    fn popup_escape_keeps_focus_held_outside_the_popup() {
        let other = Id::unique("other");
        let mut h = harness(
            |ui, s: &mut State| {
                let trigger = ui.interact(
                    Rect::from_min_size(pos2(10.0, 10.0), vec2(40.0, 20.0)),
                    Id::unique("trigger"),
                    Sense::click(),
                );
                ui.interact(
                    Rect::from_min_size(pos2(10.0, 200.0), vec2(40.0, 20.0)),
                    other,
                    Sense::click(),
                );
                let popup = Popup::new("popup", Anchor::new(Side::Bottom, Align::Start));
                s.out = Some(popup.show(ui.rt(), &trigger, &mut s.open, |_, _| ()));
            },
            State { open: true, out: None },
        );
        h.ctx.memory_mut(|m| m.request_focus(other));
        h.run();
        h.event(key_down(Key::Escape, Modifiers::NONE));
        h.step();
        assert!(!h.state().value.open);
        assert_eq!(h.ctx.memory(egui::Memory::focused), Some(other));
    }

    #[derive(Default)]
    struct InDialog {
        dialog: bool,
        open: bool,
        trigger: Option<Rect>,
        layers: Option<(egui::LayerId, egui::LayerId)>,
    }

    #[test]
    fn popup_opened_by_a_press_in_a_reopened_dialog_stays_above_it() {
        let mut h = harness(
            |ui, s: &mut InDialog| {
                if !s.dialog {
                    return;
                }
                let rt = ui.rt();
                let modal = egui::Modal::new(Id::unique("dialog")).show(rt.ctx(), |ui| {
                    let (rect, trigger) = ui.allocate_exact_size(vec2(40.0, 20.0), Sense::click());
                    s.trigger = Some(rect);
                    s.open |= trigger.is_pointer_button_down_on();
                    let popup = Popup::new("popup", Anchor::new(Side::Bottom, Align::Start));
                    popup
                        .show(rt, &trigger, &mut s.open, |ui, _| {
                            ui.allocate_exact_size(vec2(60.0, 40.0), Sense::hover());
                        })
                        .response
                        .map(|r| r.layer_id)
                });
                s.layers = modal.inner.map(|popup| (modal.response.layer_id, popup));
            },
            InDialog {
                dialog: true,
                open: true,
                ..InDialog::default()
            },
        );
        h.run();
        h.state_mut().value = InDialog::default();
        h.run();
        h.state_mut().value.dialog = true;
        h.run();
        let trigger = h.state().value.trigger.expect("trigger shown");
        h.input_mut().events.extend(press(trigger.center()));
        h.run();
        let (modal, popup) = h.state().value.layers.expect("popup open");
        let order: Vec<_> = h.ctx.memory(|m| m.layer_ids().collect());
        let at = |l| order.iter().position(|&o| o == l);
        assert!(at(popup) > at(modal), "popup above its dialog: {order:?}");
    }

    #[test]
    fn invalid_popup_size_counts_as_zero() {
        let anchor = Anchor::new(Side::Bottom, Align::Start);
        let p = Popup::new("p", anchor).size(vec2(-4.0, f32::NAN));
        assert_eq!(p.size, Some(egui::Vec2::ZERO));
    }
}

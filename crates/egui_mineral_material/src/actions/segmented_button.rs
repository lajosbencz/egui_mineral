use crate::MaterialUiExt as _;
use egui_mineral::kit::choice::Choice;
use egui_mineral::select::ItemKey;
use std::sync::Arc;

use egui::{Color32, Galley, Rangef, Rect, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};
use egui_mineral::{
    focus::{self, RovingFocus, TabStop},
    i18n::I18n,
    kit,
    layout::{Axis, Constraints, Measure},
    select::{Check, Selection},
    semantic::{self, KeyedSalt, Role, SemanticNode, StableId},
    widget::{Availability, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::widget::{common, indication, parts};
use crate::{
    Icon, Material, MaterialExt, SelectionResponse, motion,
    spec::{
        defaults::{button, segmented_button as sbx},
        tokens::outlined_segmented_button as sb,
    },
    tokens::Tokens,
};

/// Entry of a [`SegmentedButton`].
#[derive(Clone, Debug)]
#[must_use]
pub struct Segment {
    label: String,
    icon: Option<Icon>,
    disabled: bool,
    salt: Option<egui_mineral::semantic::KeyedSalt>,
}

impl Segment {
    crate::widget::setters! {
        /// Default the index.
        item_salt
    }

    /// `label` is the visible text and accessible name.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            disabled: false,
            salt: None,
        }
    }

    /// Shown while unselected; crossfades to the check mark on selection.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    crate::widget::setters! {
        /// Greys out this segment only.
        disabled
    }
}

/// Compose `SingleChoiceSegmentedButtonRow` and `MultiChoiceSegmentedButtonRow`: outlined segments
/// joined in one row, of equal width unless [`Self::equal_width`] is off. Arrow keys move focus; in
/// single mode they also select.
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct SegmentedButton<'a, K> {
    salt: KeyedSalt,
    label: String,
    choice: Choice<'a, K>,
    items: Vec<Segment>,
    disabled: bool,
    equal_width: bool,
}

impl<'a, K: ItemKey> SegmentedButton<'a, K> {
    fn with(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, choice: Choice<'a, K>) -> Self {
        Self {
            salt: id_salt.into(),
            label: label.into(),
            choice,
            items: Vec::new(),
            disabled: false,
            equal_width: true,
        }
    }

    /// Radio group selecting the segment whose value equals `selected`, none when no segment matches.
    /// `label` names the group; segment agent keys are `<id_salt>-<item>-segment`, `<item>` the segment's
    /// `salt(..)`, else its index.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, selected: &'a mut K) -> Self {
        Self::with(id_salt, label, Choice::new(selected))
    }

    /// [`Self::new`] that may have no segment selected: `None` shows none, a click selects one.
    pub fn optional(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, selected: &'a mut Option<K>) -> Self {
        Self::with(id_salt, label, Choice::optional(selected))
    }

    /// Checkbox group; a click toggles the segment's value in `selection`.
    pub fn multi(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, selection: &'a mut Selection<K>) -> Self
    where
        K: Ord,
    {
        Self::with(id_salt, label, Choice::multi(selection))
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix of the group and its segments.
        agent_key(salt)
    }

    /// Appends one segment carrying `value`.
    pub fn item(mut self, value: K, item: impl Into<Segment>) -> Self {
        self.choice.push(value);
        self.items.push(item.into());
        self
    }

    /// Appends `(value, segment)` pairs; labels convert into [`Segment`].
    pub fn items(mut self, items: impl IntoIterator<Item = (K, impl Into<Segment>)>) -> Self {
        for (value, item) in items {
            self = self.item(value, item);
        }
        self
    }

    crate::widget::setters! {
        /// Greys out every segment.
        disabled
    }

    /// `false` sizes each segment to its content instead of the widest one (Compose
    /// `weight(1f)`); see `docs/divergences.md`. Default `true`.
    pub fn equal_width(mut self, equal_width: bool) -> Self {
        self.equal_width = equal_width;
        self
    }

    /// Paints the row, narrowed to the available width.
    pub fn show(mut self, ui: &mut MineralUi<'_, Material>) -> SelectionResponse<K> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        for s in &mut self.items {
            s.disabled |= self.disabled;
        }
        let len = self.items.len();
        let single = self.choice.is_single();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let selected: Vec<bool> = (0..len).map(|i| self.choice.is_selected(i)).collect();
        let enabled = |i: usize| self.items.get(i).is_some_and(|s| !s.disabled);
        let stop = focus::tab_stop(ctx, id, selected.iter().position(|s| *s), len, enabled);

        let font = sb::LABEL_TEXT_FONT.font(ctx);
        let labels: Vec<Arc<Galley>> = self
            .items
            .iter()
            .map(|s| {
                parts::label_galley(
                    rt.core(),
                    ui.painter(),
                    &s.label,
                    font.clone(),
                    Color32::PLACEHOLDER,
                    None,
                )
            })
            .collect();
        let overlap = sb::OUTLINE_WIDTH * len.saturating_sub(1) as f32;
        let widths = segment_widths(&labels, self.equal_width, ui.available_width() + overlap);
        let height = ui.density().dense_to(
            sb::CONTAINER_HEIGHT,
            labels.iter().map(|l| l.size().y).fold(0.0, f32::max) + crate::adaptive::CONTENT_PAD,
        );
        let row = vec2(
            (widths.iter().sum::<f32>() - overlap).max(0.0),
            height.max(crate::adaptive::min_interactive(ui.density())),
        );
        let role = if single { Role::RadioButton } else { Role::CheckBox };

        let mut clicked = None;
        let mut ids = Vec::with_capacity(len);
        let outer = ui.scope(|ui| {
            // Compose `Arrangement.spacedBy(-space)`: neighbours share one outline.
            ui.spacing_mut().item_spacing = vec2(-sb::OUTLINE_WIDTH, 0.0);
            ui.allocate_ui_with_layout(row, rt.i18n().direction().row(), |ui| {
                for (i, (((segment, label), &selected), &width)) in
                    self.items.iter().zip(&labels).zip(&selected).zip(&widths).enumerate()
                {
                    let own = crate::widget::item_salt(&self.salt, segment.salt.as_ref(), None, self.choice.value(i));
                    let r = rt.ui(ui).mineral(SegmentItem {
                        salt: own.child("segment"),
                        key: StableId::of(&own, "segment"),
                        segment,
                        label,
                        role,
                        selected,
                        start: i == 0,
                        end: i + 1 == len,
                        tab_stop: TabStop::at(i, stop),
                        size: vec2(width, height),
                    });
                    if r.clicked() {
                        clicked = Some(i);
                    }
                    ids.push(r.response().id);
                }
            })
        });

        // APG radio group: arrows move focus and select; checkboxes only move focus.
        let picked = clicked.or_else(|| {
            RovingFocus::new(Axis::Horizontal)
                .update_group(rt.core(), id, &ids, enabled)
                .filter(|&i| single && !self.choice.is_selected(i))
        });

        let response = outer.response.interact(Sense::hover());
        semantic::emit(ctx, Some(response.id), response.rect, || {
            let role = if single { Role::RadioGroup } else { Role::Group };
            let mut n = SemanticNode::new(StableId::of(&self.salt, "segmented-button"), role, &self.label);
            n.state.disabled = self.disabled;
            n
        });
        self.choice.pick(response, picked)
    }
}

// Each segment's width, scaled down together to fit `available`.
fn segment_widths(labels: &[Arc<Galley>], equal: bool, available: f32) -> Vec<f32> {
    let mut widths: Vec<f32> = labels
        .iter()
        .map(|g| (2.0 * sbx::CONTENT_SPACE + sb::ICON_SIZE + sbx::ICON_SPACING + g.size().x).max(button::MIN_WIDTH))
        .collect();
    if equal {
        // Compose `weight(1f)` in an `IntrinsicSize.Min` row: every segment takes the widest one's width.
        let widest = widths.iter().copied().fold(button::MIN_WIDTH, f32::max);
        widths.fill(widest);
    }
    let fit = (available / widths.iter().sum::<f32>()).clamp(0.0, 1.0);
    if fit.is_finite() {
        for w in &mut widths {
            *w *= fit;
        }
    }
    widths
}

// Compose SegmentedButton
struct SegmentItem<'a> {
    salt: KeyedSalt,
    key: StableId,
    segment: &'a Segment,
    label: &'a Arc<Galley>,
    role: Role,
    selected: bool,
    start: bool,
    end: bool,
    tab_stop: TabStop,
    size: Vec2,
}

impl SegmentItem<'_> {
    // Compose SegmentedButtonColors
    fn colors(&self, t: &Tokens) -> (Color32, Color32, Color32) {
        let container = if self.selected {
            t.role(sb::SELECTED_CONTAINER_COLOR)
        } else {
            Color32::TRANSPARENT
        };
        if self.segment.disabled {
            return (
                container,
                t.role_alpha(sb::DISABLED_LABEL_TEXT_COLOR, sb::DISABLED_LABEL_TEXT_OPACITY),
                t.role_alpha(sb::OUTLINE_COLOR, sb::DISABLED_OUTLINE_OPACITY),
            );
        }
        let content = if self.selected {
            sb::SELECTED_LABEL_TEXT_COLOR
        } else {
            sb::UNSELECTED_LABEL_TEXT_COLOR
        };
        (container, t.role(content), t.role(sb::OUTLINE_COLOR))
    }

    // Compose SegmentedButtonDefaults.Icon, SegmentedButtonContentMeasurePolicy
    fn paint_content(&self, ui: &MineralUi<'_, Material>, id: egui::Id, rect: Rect, content: Color32) {
        let rt = ui.rt();
        let dir = rt.i18n().direction();
        let slot = sb::ICON_SIZE + sbx::ICON_SPACING;
        let inner = rect.shrink2(vec2(sbx::CONTENT_SPACE, 0.0));
        let from = (inner.width() - slot - self.label.size().x) / 2.0;
        let shown = self.selected || self.segment.icon.is_some();
        let target = if shown { 0.0 } else { -slot / 2.0 };
        let offset = egui_mineral::motion::spring(rt.core(), id.with("offset"), target, motion::FAST_SPATIAL);
        let p = ui.painter_at(rect);
        let label_from = from + slot + offset;
        let label_x = dir.start_x(inner, Rangef::new(label_from, label_from + self.label.size().x));
        p.galley(
            pos2(label_x, rect.center().y - self.label.size().y / 2.0),
            self.label.clone(),
            content,
        );

        let icon_x = dir.start_x(inner, Rangef::new(from, from + sb::ICON_SIZE));
        let icon_rect = Rect::from_min_size(
            pos2(icon_x, rect.center().y - sb::ICON_SIZE / 2.0),
            Vec2::splat(sb::ICON_SIZE),
        );
        let fade = egui_mineral::motion::spring(
            rt.core(),
            id.with("check"),
            f32::from(self.selected),
            motion::DEFAULT_EFFECTS,
        )
        .clamp(0.0, 1.0);
        if let Some(icon) = self.segment.icon {
            if fade < 1.0 {
                common::icon(
                    &p,
                    icon_rect.center(),
                    icon,
                    sb::ICON_SIZE,
                    common::with_alpha(content, 1.0 - fade),
                );
            }
            if fade > 0.0 {
                common::icon(
                    &p,
                    icon_rect.center(),
                    ui.icons().check,
                    sb::ICON_SIZE,
                    common::with_alpha(content, fade),
                );
            }
            return;
        }
        let scale = egui_mineral::motion::spring(
            rt.core(),
            id.with("check-scale"),
            f32::from(self.selected),
            motion::FAST_SPATIAL,
        )
        .max(0.0);
        // `scaleIn` from `TransformOrigin(0f, 1f)`; no exit transition.
        if self.selected && scale * sb::ICON_SIZE >= 1.0 {
            let origin = icon_rect.left_bottom();
            let center = origin + (icon_rect.center() - origin) * scale;
            common::icon(
                &p,
                center,
                ui.icons().check,
                sb::ICON_SIZE * scale,
                common::with_alpha(content, fade),
            );
        }
    }
}

impl MineralWidget for SegmentItem<'_> {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt);

    fn sense(&self) -> Sense {
        focus::roving_sense(Sense::click(), self.tab_stop)
    }

    fn enabled(&self) -> bool {
        !self.segment.disabled
    }

    fn layout(&self, _ui: &MineralUi<'_, Material>, constraints: Constraints, (): &Self::State) -> Measure {
        constraints.constrain(self.size).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let response = &frame.response;
        kit::interact::follow_press(response);
        let rect = frame.zones.visual;
        let press = parts::press(ui, frame, Availability::disabled_if(self.segment.disabled));
        let (container, content, outline) = self.colors(&t);
        let full = crate::actions::button::radius(&t, sb::SHAPE.scale(), rect.height());
        let radius = crate::actions::button::sided(
            if self.start { full } else { 0.0 },
            if self.end { full } else { 0.0 },
            ui.i18n().direction(),
        );
        ui.painter().rect_filled(rect, radius, container);
        indication::paint(ui, response, rect, radius, content, press.interaction, &t);
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(sb::OUTLINE_WIDTH, outline),
            StrokeKind::Inside,
        );
        self.paint_content(ui, response.id, rect, content);
        let changed = press.clicked && (self.role == Role::CheckBox || !self.selected);
        WidgetResponse::new(response.clone(), ()).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.clone(), self.role, &self.segment.label)
            .clickable(Availability::disabled_if(self.segment.disabled));
        n.state.checked = Some(Check::from(self.selected));
        if self.role == Role::RadioButton {
            n.state.selected = Some(self.selected);
        }
        Some(n)
    }
}

// Compose ListItemColors; focus shows the state layer, no ring (RippleDefaults OpacityFocusThemeConfiguration)

use crate::Material;
#[cfg(feature = "extras")]
use crate::MaterialUiExt as _;
use crate::adaptive::Density;
use egui::{Color32, CornerRadius, Id, Painter, Rangef, Rect, Response, Sense, pos2, vec2};
#[cfg(feature = "extras")]
use egui::{Pos2, Stroke};
use egui_mineral::semantic::{Role, SemanticNode, StableId};
use egui_mineral::widget::Availability;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    combobox::{ComboAction, ComboKind, ComboList, ComboState},
    i18n::Direction,
    kit,
    select::Check,
};

use crate::containment::ListLines;
#[cfg(feature = "extras")]
use crate::selection::controls::Control;
use crate::selection::{controls, select};
#[cfg(feature = "extras")]
use crate::spec::tokens::{divider, x_small_icon_button};
use crate::widget::{
    common::{self, Interaction},
    indication, parts,
};
use crate::{
    Leading,
    spec::{
        ColorRole, TypeScale, defaults,
        tokens::{list, menu, reorder_list},
    },
    tokens::Tokens,
};

// Dense down to the text lines and padding.
pub(crate) fn height(density: Density, lines: ListLines) -> f32 {
    let (head, sub) = (TypeScale::BodyLarge.line_height(), TypeScale::BodyMedium.line_height());
    let text = match lines {
        ListLines::One => head,
        ListLines::Two => head + sub,
        ListLines::Three => head + 2.0 * sub,
    };
    density.dense_to(lines.height(), text + 8.0)
}

pub(crate) fn row_height(density: Density) -> f32 {
    height(density, ListLines::One)
}

// Compose sizes a clickable checkbox/radio to the minimum interactive size
pub(crate) fn control_slot(density: Density) -> f32 {
    crate::adaptive::min_interactive(density)
}

pub(crate) fn control_target(
    ui: &MineralUi<'_, Material>,
    row: &Response,
    salt: impl egui::AsIdSalt,
    hit: Rect,
    disabled: bool,
) -> Response {
    kit::input::interact(
        ui,
        hit,
        row.id.with(salt),
        Sense::CLICK,
        Availability::disabled_if(disabled),
    )
}

#[cfg(feature = "extras")]
pub(crate) fn bottom_divider(p: &Painter, rect: Rect, t: &Tokens) {
    let w = divider::THICKNESS;
    p.hline(
        rect.x_range(),
        rect.bottom() - w / 2.0,
        Stroke::new(w, t.role(divider::COLOR)),
    );
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct RowState {
    pub selected: bool,
    pub disabled: bool,
    /// Lifted by a reorder drag.
    pub dragged: bool,
}

impl RowState {
    pub(crate) fn shape(self, t: &Tokens, dir: Direction) -> CornerRadius {
        if self.dragged {
            reorder_list::ITEM_SHAPE
        } else {
            list::ITEM_CONTAINER_SHAPE
        }
        .corner_radius(&t.shape, dir)
    }
}

// Compose ListItemDefaults.colors(); disabled wins over dragged, dragged over selected
#[derive(Clone, Copy)]
pub(crate) struct RowColors {
    pub container: Color32,
    pub label: Color32,
    pub supporting: Color32,
    pub overline: Color32,
    pub leading: Color32,
    /// Trailing icon and text.
    pub trailing: Color32,
}

impl RowColors {
    // Compose ListItemColors.containerColor: idle and disabled rows; disabled wins over dragged and selected.
    pub(crate) fn container(mut self, s: RowState, role: Option<ColorRole>, t: &Tokens) -> Self {
        if let Some(role) = role
            && (s.disabled || !(s.dragged || s.selected))
        {
            self.container = t.role(role);
        }
        self
    }

    pub(crate) fn of(s: RowState, t: &Tokens) -> Self {
        use list as m;
        use reorder_list as r;
        if s.disabled {
            Self {
                container: t.role(m::ITEM_CONTAINER_COLOR),
                label: t.role_alpha(m::ITEM_DISABLED_LABEL_TEXT_COLOR, m::ITEM_DISABLED_LABEL_TEXT_OPACITY),
                supporting: t.role_alpha(
                    m::ITEM_DISABLED_SUPPORTING_TEXT_COLOR,
                    m::ITEM_DISABLED_SUPPORTING_TEXT_OPACITY,
                ),
                overline: t.role_alpha(m::ITEM_DISABLED_OVERLINE_COLOR, m::ITEM_DISABLED_OVERLINE_OPACITY),
                leading: t.role_alpha(
                    m::ITEM_DISABLED_LEADING_ICON_COLOR,
                    m::ITEM_DISABLED_LEADING_ICON_OPACITY,
                ),
                trailing: t.role_alpha(
                    m::ITEM_DISABLED_TRAILING_ICON_COLOR,
                    m::ITEM_DISABLED_TRAILING_ICON_OPACITY,
                ),
            }
        } else if s.dragged {
            Self {
                container: t.role(r::ITEM_CONTAINER_COLOR),
                label: t.role(r::ITEM_LABEL_TEXT_COLOR),
                supporting: t.role(r::ITEM_SUPPORTING_TEXT_COLOR),
                overline: t.role(r::ITEM_OVERLINE_COLOR),
                leading: t.role(r::ITEM_LEADING_ICON_COLOR),
                trailing: t.role(r::ITEM_TRAILING_ICON_COLOR),
            }
        } else if s.selected {
            Self {
                container: t.role(m::ITEM_SELECTED_CONTAINER_COLOR),
                label: t.role(m::ITEM_SELECTED_LABEL_TEXT_COLOR),
                supporting: t.role(m::ITEM_SELECTED_SUPPORTING_TEXT_COLOR),
                overline: t.role(m::ITEM_SELECTED_OVERLINE_COLOR),
                leading: t.role(m::ITEM_SELECTED_LEADING_ICON_COLOR),
                trailing: t.role(m::ITEM_SELECTED_TRAILING_ICON_COLOR),
            }
        } else {
            Self {
                container: t.role(m::ITEM_CONTAINER_COLOR),
                label: t.role(m::ITEM_LABEL_TEXT_COLOR),
                supporting: t.role(m::ITEM_SUPPORTING_TEXT_COLOR),
                overline: t.role(m::ITEM_OVERLINE_COLOR),
                leading: t.role(m::ITEM_LEADING_ICON_COLOR),
                trailing: t.role(m::ITEM_TRAILING_ICON_COLOR),
            }
        }
    }
}

pub(crate) fn paint_container(
    p: &Painter,
    rect: Rect,
    s: RowState,
    shape: CornerRadius,
    colors: &RowColors,
    t: &Tokens,
) {
    if s.dragged {
        common::elevation(p, rect, shape, list::ITEM_DRAGGED_CONTAINER_ELEVATION, t);
    }
    p.rect_filled(rect, shape, colors.container);
    if s.dragged {
        p.rect_filled(rect, shape, common::with_alpha(colors.label, t.state.dragged));
    }
}

#[cfg(feature = "extras")]
pub(crate) fn paint_checkbox(
    ui: &MineralUi<'_, Material>,
    target: &Response,
    center: Pos2,
    state: Check,
    disabled: bool,
    t: &Tokens,
) {
    controls::paint_checkbox(ui, target.id, center, state, false, !disabled, t);
    let node = Control::Checkbox.node(ui.density(), center);
    Control::Checkbox.indication(ui, target, node, disabled, t);
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ListStep {
    pub moved: bool,
    pub commit: Option<usize>,
}

pub(crate) fn drive_list(ctx: &egui::Context, combo: &mut ComboState, kind: ComboKind, len: usize) -> ListStep {
    let list = ComboList {
        kind,
        selected: None,
        len,
        enabled: &|_| true,
    };
    let mut step = ListStep::default();
    for action in combo.update(ctx, list) {
        match action {
            ComboAction::Commit(i) => step.commit = Some(i),
            ComboAction::Moved(_) | ComboAction::Opened => step.moved = true,
            _ => {}
        }
    }
    step
}

// For hosts that store only the active option.
pub(crate) fn drive_embedded(ctx: &egui::Context, active: &mut Option<usize>, len: usize) -> ListStep {
    let mut combo = ComboState::default();
    // `active` was stored against an earlier, possibly longer, list.
    combo.open(active.filter(|a| *a < len), len);
    let step = drive_list(ctx, &mut combo, ComboKind::Embedded, len);
    *active = combo.active();
    step
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RowMetrics {
    pub height: f32,
    pub start: f32,
    pub end: f32,
    /// Space between the leading slot and the label.
    pub gap: f32,
    pub font: TypeScale,
    /// Container, label and leading colors of a selected row.
    pub selected: [ColorRole; 3],
}

impl RowMetrics {
    pub(crate) fn list(density: Density) -> Self {
        use defaults::list as d;
        Self {
            height: row_height(density),
            start: d::START_PADDING,
            end: d::END_PADDING,
            gap: d::LEADING_CONTENT_END_PADDING,
            font: list::ITEM_LABEL_TEXT_FONT,
            selected: [
                list::ITEM_SELECTED_CONTAINER_COLOR,
                list::ITEM_SELECTED_LABEL_TEXT_COLOR,
                list::ITEM_SELECTED_LEADING_ICON_COLOR,
            ],
        }
    }

    // Compose DropdownMenuItem, ExposedDropdownMenuDefaults.ItemContentPadding
    pub(crate) const MENU: Self = Self {
        height: defaults::menu::ITEM_HEIGHT,
        start: defaults::select::ITEM_HORIZONTAL_PADDING,
        end: defaults::select::ITEM_HORIZONTAL_PADDING,
        gap: defaults::menu::ITEM_HORIZONTAL_PADDING,
        font: defaults::menu::ITEM_FONT,
        selected: [
            menu::LIST_ITEM_SELECTED_CONTAINER_COLOR,
            menu::LIST_ITEM_SELECTED_LABEL_TEXT_COLOR,
            menu::LIST_ITEM_SELECTED_LEADING_TRAILING_ICON_COLOR,
        ],
    };
}

// Compose search samples: transparent ListItem container
#[derive(Clone, Default)]
pub(crate) struct OptionRow<'a> {
    pub label: &'a str,
    /// Char indices of `label` drawn in the hit color.
    pub hits: &'a [usize],
    pub leading: Option<Leading>,
    /// Checkbox in the leading slot (multi-select); the row toggles it. Replaces `leading`.
    pub check: Option<bool>,
    /// Trailing supporting text.
    pub trailing: Option<&'a str>,
    /// Single-select choice: the container takes the selected colors.
    pub selected: bool,
    pub active: bool,
    pub disabled: bool,
}

impl OptionRow<'_> {
    // Held-value list: selected is the choice or check state.
    pub(crate) fn value_node(&self, key: StableId) -> SemanticNode {
        self.node(key, self.check.unwrap_or(self.selected))
    }

    // Pick list (no held value): selected mirrors the active row.
    pub(crate) fn pick_node(&self, key: StableId) -> SemanticNode {
        self.node(key, self.active)
    }

    fn node(&self, key: StableId, selected: bool) -> SemanticNode {
        let mut node =
            SemanticNode::new(key, Role::ListBoxOption, self.label).pressable(Availability::disabled_if(self.disabled));
        node.state.selected = Some(selected);
        node
    }

    pub(crate) fn show(
        &self,
        ui: &mut MineralUi<'_, Material>,
        id: Id,
        width: f32,
        m: &RowMetrics,
        t: &Tokens,
    ) -> Response {
        let rt = ui.rt();
        let availability = Availability::disabled_if(self.disabled);
        let (rect, _) = ui.allocate_exact_size(vec2(width, m.height), Sense::hover());
        let response = kit::input::interact(ui, rect, id, Sense::click(), availability);
        let dir = rt.i18n().direction();
        let state = RowState {
            disabled: self.disabled,
            ..RowState::default()
        };
        let mut col = RowColors::of(state, t);
        if self.selected && self.check.is_none() {
            let [container, label, leading] = m.selected;
            ui.painter().rect_filled(rect, 0, t.role(container));
            if !self.disabled {
                (col.label, col.leading) = (t.role(label), t.role(leading));
            }
        }
        let i = Interaction::of(&response, availability).with_focused(self.active);
        indication::paint(ui, &response, rect, state.shape(t, dir), col.label, i, t);
        kit::input::hover_cursor(rt.core(), &response, availability);
        let mut cur = dir.cursor(rect);
        cur.skip(m.start);
        let icon = list::ITEM_LEADING_ICON_SIZE;
        if let Some(on) = self.check {
            let state = if on { Check::Checked } else { Check::Unchecked };
            let center = cur.take(icon, icon).center();
            controls::paint_checkbox(ui, response.id, center, state, false, !self.disabled, t);
            cur.skip(m.gap);
        } else if let Some(lead) = &self.leading {
            lead.paint(ui.painter(), cur.take(icon, icon).center(), icon, col.leading);
            cur.skip(m.gap);
        }
        let p = ui.painter();
        let mut end = m.end;
        if let Some(text) = self.trailing {
            let g = parts::type_galley(
                rt.core(),
                p,
                text,
                list::ITEM_TRAILING_SUPPORTING_TEXT_FONT,
                col.trailing,
                None,
            );
            let w = g.size().x;
            let x = dir.reversed().start_x(rect, Rangef::new(end, end + w));
            p.galley(pos2(x, rect.center().y - g.size().y / 2.0), g, col.trailing);
            end += w + defaults::list::TRAILING_CONTENT_START_PADDING;
        }
        let hit = if self.disabled {
            col.label
        } else {
            t.role(ColorRole::Primary)
        };
        let job = select::highlighted(
            rt.ctx(),
            self.label,
            self.hits,
            m.font,
            col.label,
            hit,
            cur.remaining() - end,
        );
        let g = p.layout_job(job);
        p.galley(cur.peek(g.size().x, g.size().y).min, g, col.label);
        response
    }
}

// Insertion line of a reorder or tree drop.
#[cfg(feature = "extras")]
pub(crate) fn drop_indicator(t: &crate::tokens::Tokens) -> Stroke {
    const WIDTH: f32 = 2.0;
    Stroke::new(WIDTH, t.color.primary)
}

// No MD3 counterpart: step and timeline markers use the x-small icon button
#[cfg(feature = "extras")]
pub(crate) const MARKER: f32 = x_small_icon_button::CONTAINER_HEIGHT;

#[cfg(feature = "extras")]
pub(crate) fn marker(p: &Painter, center: Pos2, diameter: f32, fill: Color32, glyph: Option<(crate::Icon, Color32)>) {
    p.circle_filled(center, diameter / 2.0, fill);
    if let Some((icon, color)) = glyph {
        common::icon(p, center, icon, x_small_icon_button::ICON_SIZE, color);
    }
}

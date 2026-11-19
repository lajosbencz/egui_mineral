use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::spec::TypeScale;
use egui_mineral::i18n::I18n;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use std::f32::consts::PI;

use egui::{Color32, CornerRadius, Id, Rangef, Rect, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder, pos2, vec2};
use egui_mineral::widget::Availability;
use egui_mineral::{
    expand::Expansion,
    focus::RovingFocus,
    i18n::Direction,
    layout::{Axis, Constraints, Measure},
    semantic::{Action, Role, SemanticNode, StableId},
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::containment::ListLines;
use crate::widget::{
    common::{self},
    indication, parts,
    row_parts::{self, RowColors, RowState},
};
use crate::{
    MaterialIcon, icons, motion,
    spec::{
        defaults,
        tokens::{divider, expanded_list, list, outlined_card, small_icon_button},
    },
};

// No MD3 counterpart: MD3 Expressive ExpandedListTokens

const TRAILING_CONTAINER: f32 = small_icon_button::CONTAINER_HEIGHT;
const CONTENT_PADDING: f32 = defaults::list::START_PADDING;

/// Header data of one [`Accordion`] panel.
#[derive(Clone, Debug)]
#[must_use]
pub struct AccordionPanel {
    title: String,
    title_style: TypeScale,
    subtitle: Option<String>,
    icon: Option<MaterialIcon>,
    disabled: bool,
}

impl AccordionPanel {
    /// `title` is the header text and accessible name.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            title_style: list::ITEM_LABEL_TEXT_FONT,
            subtitle: None,
            icon: None,
            disabled: false,
        }
    }
    /// Title type style; default [`TypeScale::BodyLarge`], the list item label. E.g.
    /// [`TypeScale::TitleSmall`] for a medium-weight heading.
    pub fn title_style(mut self, title_style: TypeScale) -> Self {
        self.title_style = title_style;
        self
    }
    /// Secondary line below the title.
    pub fn supporting(mut self, supporting: impl Into<String>) -> Self {
        self.subtitle = Some(supporting.into());
        self
    }
    /// Leading header icon.
    pub fn icon(mut self, icon: MaterialIcon) -> Self {
        self.icon = Some(icon);
        self
    }
    crate::widget::setters! {
        /// Greys out the header; the panel cannot toggle.
        disabled
    }
}

impl From<&str> for AccordionPanel {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for AccordionPanel {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

struct Header<'a> {
    salt: KeyedSalt,
    key: StableId,
    panel: &'a AccordionPanel,
    expanded: bool,
    turn: f32,
    fade: f32,
    shape: CornerRadius,
}

impl Header<'_> {
    fn lines(&self) -> ListLines {
        if self.panel.subtitle.is_some() {
            ListLines::Two
        } else {
            ListLines::One
        }
    }
}

impl MineralWidget for Header<'_> {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt);

    fn enabled(&self) -> bool {
        !self.panel.disabled
    }

    fn layout(&self, ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        c.constrain(vec2(c.max.x, row_parts::height(ui.density(), self.lines())))
            .into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        use defaults::list as d;
        let t = ui.tokens();
        let r = &frame.response;
        let rect = frame.zones.visual;
        let dis = self.panel.disabled;
        let press = parts::press(ui, frame, Availability::disabled_if(dis));
        let changed = press.clicked;
        let col = RowColors::of(
            RowState {
                disabled: dis,
                ..RowState::default()
            },
            &t,
        );
        indication::paint(ui, r, rect, self.shape, col.label, press.interaction, &t);
        let dir = ui.i18n().direction();
        let x = |from_start: f32, w: f32| dir.start_x(rect, Rangef::new(from_start, from_start + w));
        let cy = rect.center().y;
        let p = ui.painter();
        let mut start = d::START_PADDING;
        if let Some(icon) = self.panel.icon {
            let size = list::ITEM_LEADING_ICON_SIZE;
            common::icon(p, pos2(x(start, size) + size / 2.0, cy), icon, size, col.leading);
            start += size + d::LEADING_CONTENT_END_PADDING;
        }
        let end = d::END_PADDING + TRAILING_CONTAINER;
        let trailing = pos2(x(rect.width() - end, TRAILING_CONTAINER) + TRAILING_CONTAINER / 2.0, cy);
        let (bg, fg) = if dis {
            (Color32::TRANSPARENT, col.trailing)
        } else {
            (
                t.role(expanded_list::COLLAPSED_ITEM_TRAILING_ICON_CONTAINER_COLOR)
                    .lerp_to_gamma(
                        t.role(expanded_list::EXPANDED_ITEM_TRAILING_ICON_CONTAINER_COLOR),
                        self.fade,
                    ),
                t.role(expanded_list::COLLAPSED_ITEM_TRAILING_ICON_ICON_COLOR)
                    .lerp_to_gamma(t.role(expanded_list::EXPANDED_ITEM_TRAILING_ICON_ICON_COLOR), self.fade),
            )
        };
        let well = Rect::from_center_size(trailing, egui::Vec2::splat(TRAILING_CONTAINER));
        p.rect_filled(
            well,
            expanded_list::TRAILING_ICON_SHAPE.corner_radius(&t.shape, Direction::Ltr),
            bg,
        );
        parts::icon_rotated(
            p,
            trailing,
            icons::ICON_EXPAND_MORE,
            list::ITEM_TRAILING_ICON_SIZE,
            fg,
            PI * self.turn,
        );
        let text_w = rect.width() - start - end - d::TRAILING_CONTENT_START_PADDING;
        let title = parts::type_wrapped(p, &self.panel.title, self.panel.title_style, col.label, text_w, 1);
        let sub = self
            .panel
            .subtitle
            .as_ref()
            .map(|s| parts::type_wrapped(p, s, list::ITEM_SUPPORTING_TEXT_FONT, col.supporting, text_w, 1));
        let total = title.size().y + sub.as_ref().map_or(0.0, |g| g.size().y);
        let mut y = cy - total / 2.0;
        for g in std::iter::once(title).chain(sub) {
            let h = g.size().y;
            p.galley(pos2(x(start, g.size().x), y), g, Color32::PLACEHOLDER);
            y += h;
        }
        WidgetResponse::new(r.clone(), ()).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.clone(), Role::Button, &self.panel.title)
            .clickable(Availability::disabled_if(self.panel.disabled));
        n.value.clone_from(&self.panel.subtitle);
        n.state.expanded = Some(self.expanded);
        if !self.panel.disabled {
            n.actions.push(if self.expanded {
                Action::Collapse
            } else {
                Action::Expand
            });
        }
        Some(n)
    }
}

/// Result of [`Accordion::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct AccordionResponse<R> {
    /// Response of the whole stack.
    pub response: egui::Response,
    /// Output of the [`Accordion::show`] closure.
    pub inner: R,
    /// Some panel expanded or collapsed this frame.
    pub changed: bool,
    /// Index of the panel toggled this frame.
    pub toggled: Option<usize>,
}

/// Stack of collapsible panels bound to an [`Expansion`]; non-MD3.
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct Accordion<'a> {
    salt: KeyedSalt,
    expansion: &'a mut Expansion<usize>,
    disabled: bool,
}

/// Result of [`AccordionUi::panel`]; `inner` is `None` while collapsed.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct AccordionPanelResponse<R> {
    /// Header response.
    pub response: egui::Response,
    /// Output of `add`; `None` while collapsed.
    pub inner: Option<R>,
}

/// Panel adder handed to the [`Accordion::show`] closure.
pub struct AccordionUi<'u, 'e> {
    ui: &'u mut Ui,
    rt: &'u crate::Rt,
    id: Id,
    salt: KeyedSalt,
    key: StableId,
    expansion: &'e mut Expansion<usize>,
    index: usize,
    count: usize,
    radius: CornerRadius,
    headers: Vec<Id>,
    toggled: Option<usize>,
    disabled: bool,
    heights: Vec<f32>,
    heights_dirty: bool,
}

egui_mineral::kit::opaque_debug!(AccordionUi<'_, '_>);

impl AccordionUi<'_, '_> {
    /// Adds the next panel, indexed in call order, with `add` as its body.
    pub fn panel<R>(
        &mut self,
        panel: impl Into<AccordionPanel>,
        add: impl FnOnce(&mut MineralUi<'_, crate::Material>) -> R,
    ) -> AccordionPanelResponse<R> {
        let mut panel = panel.into();
        panel.disabled |= self.disabled;
        let i = self.index;
        self.index += 1;
        let rt = self.rt;
        let ctx = self.ui.ctx().clone();
        let t = rt.tokens();
        if i > 0 {
            let (r, _) = self
                .ui
                .allocate_exact_size(vec2(self.ui.available_width(), divider::THICKNESS), Sense::hover());
            self.ui.painter().rect_filled(r, 0, t.role(divider::COLOR));
        }
        let open = self.expansion.is_expanded(&i);
        let k = egui_mineral::motion::spring(
            rt.core(),
            self.id.with(("open", i)),
            f32::from(open),
            motion::DEFAULT_SPATIAL,
        );
        let fade = egui_mineral::motion::spring(
            rt.core(),
            self.id.with(("fade", i)),
            f32::from(open),
            motion::DEFAULT_EFFECTS,
        );
        let last = i + 1 == self.count && k <= 0.0;
        let r = self.radius;
        let header = Header {
            salt: self.salt.child(("header", i)),
            key: self.key.sub((i, "header")),
            panel: &panel,
            expanded: open,
            turn: k,
            fade,
            shape: CornerRadius {
                nw: if i == 0 { r.nw } else { 0 },
                ne: if i == 0 { r.ne } else { 0 },
                sw: if last { r.sw } else { 0 },
                se: if last { r.se } else { 0 },
            },
        };
        let mut hr = rt.ui(self.ui).mineral(header);
        let expand_req = egui_mineral::kit::input::accesskit_requested(&ctx, hr.id, Action::Expand);
        let collapse_req = egui_mineral::kit::input::accesskit_requested(&ctx, hr.id, Action::Collapse);
        if !panel.disabled {
            self.headers.push(hr.id);
            if hr.changed() || (expand_req && !open) || (collapse_req && open) {
                self.expansion.toggle(i);
                self.toggled = Some(i);
                hr = hr.with_changed(true);
            }
        }
        // The spatial spring overshoots; the panel never grows past its content.
        let k = k.clamp(0.0, 1.0);
        if k <= 0.0 {
            return AccordionPanelResponse {
                response: hr.into_response(),
                inner: None,
            };
        }
        let w = self.ui.available_width();
        let top = self.ui.cursor().min;
        let known = self.heights.get(i).copied().filter(|h| *h > 0.0);
        let clip_h = match known {
            Some(h) => h * k,
            None if k >= 1.0 => f32::INFINITY,
            None => 0.0,
        };
        let pad = CONTENT_PADDING;
        let mut child = self.ui.new_child(
            UiBuilder::new()
                .id_salt(self.id.with(("content", i)))
                .max_rect(Rect::from_min_size(
                    top + vec2(pad, 0.0),
                    vec2(w - 2.0 * pad, f32::INFINITY),
                ))
                .layout(rt.i18n().direction().column()),
        );
        let clip = Rect::from_min_size(top, vec2(w, clip_h)).intersect(self.ui.clip_rect());
        child.set_clip_rect(clip);
        let fade = fade.clamp(0.0, 1.0);
        if fade < 1.0 {
            child.multiply_opacity(fade);
        }
        let inner = add(&mut rt.ui(&mut child));
        let measured = child.min_rect().height() + pad;
        if known.is_none_or(|h| (h - measured).abs() > 0.5) {
            if self.heights.len() <= i {
                self.heights.resize(i + 1, 0.0);
            }
            self.heights[i] = measured;
            self.heights_dirty = true;
        }
        self.ui.allocate_exact_size(vec2(w, measured * k), Sense::hover());
        AccordionPanelResponse {
            response: hr.into_response(),
            inner: Some(inner),
        }
    }
}

impl<'a> Accordion<'a> {
    /// `expansion` is the bound set of expanded panel indices. Agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, expansion: &'a mut Expansion<usize>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            expansion,
            disabled: false,
        }
    }

    crate::widget::setters! {
        /// Greys out every panel header.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Stacks the panels `add` adds, with dividers between them.
    pub fn show<R>(
        self,
        ui: &mut MineralUi<'_, Material>,
        add: impl FnOnce(&mut AccordionUi<'_, '_>) -> R,
    ) -> AccordionResponse<R> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = rt.tokens();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let count_id = id.with("count");
        let heights_id = id.with("heights");
        let radius = expanded_list::CONTAINER_SHAPE.corner_radius(&t.shape, rt.i18n().direction());
        let bg = ui.painter().add(Shape::Noop);
        let r = ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let mut aui = AccordionUi {
                ui,
                rt,
                id,
                salt: self.salt.clone(),
                key: StableId::from(&self.salt),
                expansion: self.expansion,
                index: 0,
                count: ctx.data(|d| d.get_temp(count_id)).unwrap_or(0),
                radius,
                headers: Vec::new(),
                toggled: None,
                disabled: self.disabled,
                heights: ctx.data(|d| d.get_temp(heights_id)).unwrap_or_default(),
                heights_dirty: false,
            };
            let inner = add(&mut aui);
            RovingFocus::new(Axis::Vertical).update(rt.core(), &aui.headers, |_| true);
            if aui.index != aui.count {
                ctx.data_mut(|d| d.insert_temp(count_id, aui.index));
            }
            if aui.heights.len() > aui.index {
                aui.heights.truncate(aui.index);
                aui.heights_dirty = true;
            }
            if aui.heights_dirty {
                ctx.data_mut(|d| d.insert_temp(heights_id, std::mem::take(&mut aui.heights)));
            }
            (inner, aui.toggled)
        });
        let mut response = r.response;
        let rect = response.rect;
        ui.painter().set(
            bg,
            egui::epaint::RectShape::filled(rect, radius, t.role(expanded_list::EXPANDED_ITEM_CONTAINER_COLOR)),
        );
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(outlined_card::OUTLINE_WIDTH, t.role(outlined_card::OUTLINE_COLOR)),
            StrokeKind::Inside,
        );
        let (inner, toggled) = r.inner;
        if toggled.is_some() {
            response.mark_changed();
        }
        AccordionResponse {
            response,
            inner,
            changed: toggled.is_some(),
            toggled,
        }
    }
}

use crate::msg;
use std::sync::Arc;

use egui::{Align2, Galley, Id, Rect, Response, Sense, Ui, UiBuilder, epaint::TextShape, pos2, text::LayoutJob, vec2};
use egui_mineral::{
    expand::Expansion,
    focus::RovingFocus,
    i18n::{Direction, I18n},
    kit,
    layout::{Axis, Constraints, Measure},
    motion::CubicBezier,
    semantic::{self, Action, AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{Availability, Interaction, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::spec::ColorToken::{ColorNeutralForeground1, ColorNeutralForegroundDisabled};
use crate::{
    Fluent, FluentExt, Icon, Rt,
    spec::{
        defaults::accordion as d,
        tokens::{motion as mo, typography as ty},
    },
    typography,
    widget::{focus, paint, tween},
};

/// Accordion header `size`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AccordionSize {
    /// `small`: 32pt minimum height, 12pt text.
    Small,
    /// `medium`.
    #[default]
    Medium,
    /// `large`: 16pt text.
    Large,
    /// `extra-large`: 20pt text.
    ExtraLarge,
}

impl AccordionSize {
    /// Every variant, smallest first.
    pub const ALL: &'static [Self] = &[Self::Small, Self::Medium, Self::Large, Self::ExtraLarge];

    // useAccordionHeaderStyles.styles.ts buttonSmall, buttonLarge, buttonExtraLarge
    const fn text(self) -> (f32, f32) {
        match self {
            Self::Small => (ty::FONT_SIZE_BASE_200, ty::LINE_HEIGHT_BASE_300),
            Self::Medium => (ty::FONT_SIZE_BASE_300, ty::LINE_HEIGHT_BASE_300),
            Self::Large => (ty::FONT_SIZE_BASE_400, ty::LINE_HEIGHT_BASE_400),
            Self::ExtraLarge => (ty::FONT_SIZE_BASE_500, ty::LINE_HEIGHT_BASE_500),
        }
    }

    const fn min_height(self) -> f32 {
        match self {
            Self::Small => d::HEADER_MIN_HEIGHT_SMALL,
            _ => d::HEADER_MIN_HEIGHT,
        }
    }
}

/// Side of the header text the expand icon sits on (`expandIconPosition`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExpandIconPosition {
    /// `start`.
    #[default]
    Start,
    /// `end`.
    End,
}

/// Arrow key navigation between headers (`navigation`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AccordionNavigation {
    /// `linear`: stops at the first and last header.
    Linear,
    /// `circular`: wraps around.
    Circular,
}

/// Fluent `AccordionItem` with its header.
#[derive(Clone, Debug)]
#[must_use]
pub struct AccordionItem {
    title: String,
    icon: Option<Icon>,
    disabled: bool,
    salt: Option<KeyedSalt>,
}

impl AccordionItem {
    /// Header text.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            icon: None,
            disabled: false,
            salt: None,
        }
    }

    crate::widget::setters! {
        item_salt
    }

    /// `icon` slot before the text.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    crate::widget::setters! {
        /// Not interactive.
        disabled
    }
}

impl From<&str> for AccordionItem {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for AccordionItem {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

struct Header<'a> {
    salt: KeyedSalt,
    item: &'a AccordionItem,
    open: bool,
    size: AccordionSize,
    position: ExpandIconPosition,
    inline: bool,
    locked: bool,
}

impl Header<'_> {
    fn galley(&self, ui: &MineralUi<'_, Fluent>, wrap: f32, color: egui::Color32) -> Arc<Galley> {
        let (size, line) = self.size.text();
        let mut job = LayoutJob::single_section(
            self.item.title.clone(),
            typography::format(
                ui.ctx(),
                crate::typography::TypeSpec {
                    size,
                    line_height: line,
                    weight: ty::FONT_WEIGHT_REGULAR,
                },
                color,
            ),
        );
        job.wrap.max_width = wrap;
        ui.painter().layout_job(job)
    }

    fn text_width(&self, avail: f32) -> f32 {
        let icons = if self.item.icon.is_some() {
            d::ICON_SIZE + d::ICON_GAP
        } else {
            0.0
        };
        (avail - d::HEADER_PADDING_START - d::HEADER_PADDING_END - d::ICON_SIZE - d::ICON_GAP - icons).max(0.0)
    }
}

impl MineralWidget for Header<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt);

    fn enabled(&self) -> bool {
        !self.item.disabled
    }

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        let avail = if c.max.x.is_finite() {
            c.max.x
        } else {
            ui.available_width()
        };
        let g = self.galley(ui, self.text_width(avail), egui::Color32::PLACEHOLDER);
        let fixed = d::HEADER_PADDING_START + d::HEADER_PADDING_END + d::ICON_SIZE + d::ICON_GAP;
        let icons = if self.item.icon.is_some() {
            d::ICON_SIZE + d::ICON_GAP
        } else {
            0.0
        };
        let w = if self.inline { fixed + icons + g.size().x } else { avail };
        c.constrain(vec2(w, g.size().y.max(self.size.min_height()))).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let r = &frame.response;
        let rect = frame.zones.visual;
        let availability = Availability::disabled_if(self.item.disabled);
        kit::input::hover_cursor(ui.core(), r, availability);
        let i = Interaction::of(r, availability);
        let color = t.color(if self.item.disabled {
            ColorNeutralForegroundDisabled
        } else {
            ColorNeutralForeground1
        });
        let dir = ui.i18n().direction();
        let end = self.position == ExpandIconPosition::End;
        let rtl = dir == Direction::Rtl;
        let target = match (end, self.open, rtl) {
            (true, true, _) => -90.0,
            (true, false, _) | (false, true, _) => 90.0,
            (false, false, false) => 0.0,
            (false, false, true) => 180.0,
        };
        let degrees = tween::tween(ui.core(), r.id, target, d::ROTATE, EASE_OUT);
        let x = |from_start: f32, w: f32| dir.start_x(rect, egui::Rangef::new(from_start, from_start + w)) + w / 2.0;
        let cy = rect.center().y;
        let mut start = d::HEADER_PADDING_START;
        let mut end_edge = d::HEADER_PADDING_END;
        let chevron_x;
        if end {
            end_edge = d::HEADER_PADDING_START;
            chevron_x = x(rect.width() - end_edge - d::ICON_SIZE, d::ICON_SIZE);
            start = d::HEADER_PADDING_END;
        } else {
            chevron_x = x(start, d::ICON_SIZE);
            start += d::ICON_SIZE + d::ICON_GAP;
        }
        let p = ui.painter();
        let chevron = ui.icons().chevron_right.regular();
        let glyph = p.layout_no_wrap(chevron.ch().to_string(), chevron.font_id(d::ICON_SIZE), color);
        let at = pos2(chevron_x, cy) - glyph.size() / 2.0;
        p.add(TextShape::new(at, glyph, color).with_angle_and_anchor(degrees.to_radians(), Align2::CENTER_CENTER));
        if let Some(icon) = self.item.icon {
            paint::icon(p, pos2(x(start, d::ICON_SIZE), cy), icon, false, d::ICON_SIZE, color);
            start += d::ICON_SIZE + d::ICON_GAP;
        }
        let reserved = if end {
            d::ICON_SIZE + d::ICON_GAP + end_edge
        } else {
            end_edge
        };
        let g = self.galley(ui, (rect.width() - start - reserved).max(0.0), color);
        let tx = dir.start_x(rect, egui::Rangef::new(start, start + g.size().x));
        ui.painter().galley(pos2(tx, cy - g.size().y / 2.0), g, color);
        if i.focused {
            focus::outline(ui.painter(), rect, t_radius(rect), &t);
        }
        let ctx = ui.ctx().clone();
        let expand = kit::input::accesskit_requested(&ctx, r.id, Action::Expand);
        let collapse = kit::input::accesskit_requested(&ctx, r.id, Action::Collapse);
        let activated = !self.item.disabled && (r.clicked() || (expand && !self.open) || (collapse && self.open));
        WidgetResponse::new(r.clone(), ()).with_changed(activated)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(
            egui_mineral::agent_key!("header").id(&self.salt),
            Role::Button,
            &self.item.title,
        )
        .clickable(Availability::disabled_if(self.item.disabled));
        n.state.expanded = Some(self.open);
        n.state.disabled |= self.locked;
        if !self.item.disabled {
            n.actions
                .push(if self.open { Action::Collapse } else { Action::Expand });
        }
        Some(n)
    }
}

// CSS `ease-out`
const EASE_OUT: CubicBezier = CubicBezier::new(0.0, 0.0, 0.58, 1.0);

fn t_radius(rect: Rect) -> egui::CornerRadius {
    crate::theme::Radius::Medium.on(rect)
}

/// Result of showing an [`Accordion`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct AccordionResponse<R> {
    /// Spans every header and open panel.
    pub response: Response,
    /// Value of the content closure.
    pub inner: R,
    /// Index of the item whose header was activated.
    pub toggled: Option<usize>,
}

/// Result of one [`AccordionUi::item`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct AccordionItemResponse<R> {
    /// Focusable header; the rect excludes the panel.
    pub response: Response,
    /// Panel content, while the panel is not collapsed.
    pub inner: Option<R>,
}

/// Fluent `Accordion`: headers that reveal panels. The [`Expansion`] mode is `multiple`:
/// [`Expansion::single`] opens one item at a time, `Expansion::default` any number.
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct Accordion<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    expansion: &'a mut Expansion<usize>,
    collapsible: bool,
    size: AccordionSize,
    position: ExpandIconPosition,
    inline: bool,
    navigation: Option<AccordionNavigation>,
}

impl<'a> Accordion<'a> {
    /// Medium, expand icons at the start, not collapsible; agent key `<id_salt>-accordion`.
    pub fn new(id_salt: impl Into<KeyedSalt>, expansion: &'a mut Expansion<usize>) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("accordion"),
            expansion,
            collapsible: false,
            size: AccordionSize::Medium,
            position: ExpandIconPosition::Start,
            inline: false,
            navigation: None,
        }
    }

    /// Lets the last open item close.
    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.collapsible = collapsible;
        self
    }

    /// Default [`AccordionSize::Medium`].
    pub fn size(mut self, size: AccordionSize) -> Self {
        self.size = size;
        self
    }

    /// Default [`ExpandIconPosition::Start`].
    pub fn expand_icon_position(mut self, position: ExpandIconPosition) -> Self {
        self.position = position;
        self
    }

    /// Headers fit their content instead of the full width.
    pub fn inline(mut self, inline: bool) -> Self {
        self.inline = inline;
        self
    }

    /// Arrow keys move between headers; default off, Tab only.
    pub fn navigation(mut self, navigation: Option<AccordionNavigation>) -> Self {
        self.navigation = navigation;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-accordion` agent key.
        agent_key(key)
    }

    /// Shows the accordion; `add` places its items.
    pub fn show<R>(
        self,
        ui: &mut MineralUi<'_, Fluent>,
        add: impl FnOnce(&mut AccordionUi<'_, '_>) -> R,
    ) -> AccordionResponse<R> {
        let rt = ui.rt();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let navigation = self.navigation;
        let key = self.key.id(&self.salt);
        let scope = ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let mut aui = AccordionUi {
                ui,
                rt,
                id,
                salt: self.salt.clone(),
                expansion: self.expansion,
                collapsible: self.collapsible,
                size: self.size,
                position: self.position,
                inline: self.inline,
                index: 0,
                headers: Vec::new(),
                enabled: Vec::new(),
                toggled: None,
            };
            let inner = add(&mut aui);
            if let Some(n) = navigation {
                let enabled = aui.enabled.clone();
                RovingFocus::new(Axis::Vertical)
                    .wrap(n == AccordionNavigation::Circular)
                    .update(rt.core(), &aui.headers, |i| enabled[i]);
            }
            (inner, aui.toggled)
        });
        let response = scope.response;
        semantic::emit(ui.ctx(), None, response.rect, || {
            SemanticNode::new(key, Role::Group, crate::tr(rt.i18n(), msg::ACCORDION))
        });
        let (inner, toggled) = scope.inner;
        AccordionResponse {
            response,
            inner,
            toggled,
        }
    }
}

/// Places the items of an [`Accordion`].
pub struct AccordionUi<'u, 'e> {
    ui: &'u mut Ui,
    rt: &'u Rt,
    id: Id,
    salt: KeyedSalt,
    expansion: &'e mut Expansion<usize>,
    collapsible: bool,
    size: AccordionSize,
    position: ExpandIconPosition,
    inline: bool,
    index: usize,
    headers: Vec<Id>,
    enabled: Vec<bool>,
    toggled: Option<usize>,
}

kit::opaque_debug!(AccordionUi<'_, '_>);

impl AccordionUi<'_, '_> {
    /// One item: its header, then `add` as the panel while open (animated by `Collapse`).
    pub fn item<R>(
        &mut self,
        item: impl Into<AccordionItem>,
        add: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R,
    ) -> AccordionItemResponse<R> {
        let item = item.into();
        let i = self.index;
        self.index += 1;
        let rt = self.rt;
        let ctx = self.ui.ctx().clone();
        let open = self.expansion.is_expanded(&i);
        let locked = open && !self.collapsible && self.expansion.expanded().count() <= 1;
        let item_salt = item.salt.clone().unwrap_or_else(|| self.salt.child(("item", i)));
        let part = item_salt.id_salt();
        let header = Header {
            salt: item_salt.clone(),
            item: &item,
            open,
            size: self.size,
            position: self.position,
            inline: self.inline,
            locked,
        };
        let hr = rt.ui(self.ui).mineral(header);
        self.headers.push(hr.id);
        self.enabled.push(!item.disabled);
        let activated = hr.changed();
        let response = hr.into_response();
        if activated && !locked {
            self.expansion.toggle(i);
            self.toggled = Some(i);
        }
        let k = tween::tween(
            rt.core(),
            self.id.with(("open", part)),
            f32::from(u8::from(self.expansion.is_expanded(&i))),
            d::COLLAPSE,
            mo::CURVE_EASY_EASE_MAX,
        );
        if k <= 0.0 {
            return AccordionItemResponse { response, inner: None };
        }
        let w = self.ui.available_width();
        let top = self.ui.cursor().min;
        let height_id = self.id.with(("height", part));
        let known: Option<f32> = ctx.data(|d| d.get_temp(height_id));
        let clip_h = match known {
            Some(h) => h * k,
            None if k >= 1.0 => f32::INFINITY,
            None => 0.0,
        };
        let margin = d::PANEL_MARGIN;
        let mut child = self.ui.new_child(
            UiBuilder::new()
                .id_salt(self.id.with(("panel", part)))
                .max_rect(Rect::from_min_size(
                    top + vec2(margin, 0.0),
                    vec2(w - 2.0 * margin, f32::INFINITY),
                ))
                .layout(rt.i18n().direction().column()),
        );
        child.set_clip_rect(Rect::from_min_size(top, vec2(w, clip_h)).intersect(self.ui.clip_rect()));
        if k < 1.0 {
            child.multiply_opacity(k);
        }
        let inner = add(&mut rt.ui(&mut child));
        let measured = child.min_rect().height();
        if known.is_none_or(|h| (h - measured).abs() > 0.5) {
            ctx.data_mut(|d| d.insert_temp(height_id, measured));
        }
        let (rect, _) = self.ui.allocate_exact_size(vec2(w, measured * k), Sense::hover());
        let key = egui_mineral::agent_key!("panel").id(&item_salt);
        semantic::emit(&ctx, None, rect, || SemanticNode::new(key, Role::Region, &item.title));
        AccordionItemResponse {
            response,
            inner: Some(inner),
        }
    }
}

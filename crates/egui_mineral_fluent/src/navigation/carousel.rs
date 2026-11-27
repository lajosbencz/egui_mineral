use crate::msg;
use egui_mineral::select::ItemKey;
use std::time::Duration;

use egui::{Color32, Id, Rect, Sense, Stroke, Ui, UiBuilder, vec2};
use egui_mineral::{
    focus::{self, RovingFocus},
    i18n::I18n,
    kit::paint::FocusStroke,
    kit::{self, state::TempState},
    layout::{Axis, Constraints, Measure},
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{Availability, Interaction, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::spec::ColorToken::{
    ColorCompoundBrandBackground, ColorCompoundBrandBackgroundHover, ColorCompoundBrandBackgroundPressed,
    ColorNeutralForeground1, ColorStrokeFocus2,
};
use crate::{
    Fluent, FluentExt,
    actions::PushButton,
    spec::{defaults::carousel as d, tokens::geometry as g},
    theme::{Elevation, Radius},
    widget::{
        paint,
        vocab::{ButtonAppearance, ButtonShape},
    },
};

/// Carousel `appearance`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CarouselAppearance {
    /// `default`: slides fill the viewport.
    #[default]
    Default,
    /// `elevated`: slides are rounded with a shadow inside an inset.
    Elevated,
}

/// Fluent `CarouselCard`: one slide; `label` names it for assistive technology and its navigation dot.
#[derive(Clone, Debug)]
#[must_use]
pub struct CarouselCard {
    label: String,
}

impl CarouselCard {
    /// Accessible name of the slide.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into() }
    }
}

impl From<String> for CarouselCard {
    fn from(label: String) -> Self {
        Self::new(label)
    }
}

impl From<&str> for CarouselCard {
    fn from(label: &str) -> Self {
        Self::new(label)
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Memo {
    height: f32,
    nav_width: f32,
    width: f32,
}

impl Memo {
    fn differs(&self, other: &Self) -> bool {
        [
            (self.height, other.height),
            (self.nav_width, other.nav_width),
            (self.width, other.width),
        ]
        .into_iter()
        .any(|(a, b)| (a - b).abs() > 0.5)
    }
}

impl TempState for Memo {}

#[derive(Clone, Copy, Debug, Default)]
struct Autoplay {
    playing: bool,
    since: f64,
}

impl TempState for Autoplay {}

// Height of a medium icon-only button, the tallest part of the row
const NAV_HEIGHT: f32 = 32.0;

struct Dot {
    salt: KeyedSalt,
    label: String,
    selected: bool,
    brand: bool,
    tab_stop: focus::TabStop,
}

impl Dot {
    fn width(&self) -> f32 {
        if self.selected {
            d::SELECTED_DOT_WIDTH + 2.0 * d::SELECTED_DOT_PADDING
        } else {
            d::DOT + 2.0 * d::DOT_PADDING
        }
    }
}

impl MineralWidget for Dot {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, no_target);

    fn sense(&self) -> Sense {
        focus::roving_sense(Sense::click(), self.tab_stop)
    }

    fn layout(&self, _: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        c.constrain(vec2(self.width(), d::DOT + 2.0 * d::DOT_PADDING)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let r = &frame.response;
        kit::interact::hover_cursor(ui.core(), r, Availability::Enabled);
        kit::interact::follow_press(r);
        let i = Interaction::of(r, Availability::Enabled);
        let step = usize::from(i.hovered) + usize::from(i.pressed);
        let rect = frame.zones.visual;
        let (opacity, size, radius) = if self.selected {
            (
                d::SELECTED_OPACITY,
                vec2(d::SELECTED_DOT_WIDTH, d::DOT),
                d::SELECTED_DOT_RADIUS,
            )
        } else {
            (d::UNSELECTED_OPACITY, vec2(d::DOT, d::DOT), d::DOT / 2.0)
        };
        let token = match (self.brand && self.selected, step) {
            (true, 0) => ColorCompoundBrandBackground,
            (true, 1) => ColorCompoundBrandBackgroundHover,
            (true, _) => ColorCompoundBrandBackgroundPressed,
            _ => ColorNeutralForeground1,
        };
        let opacity = if self.brand && self.selected {
            1.0
        } else {
            opacity[step.min(2)]
        };
        let color: Color32 = t.color(token).gamma_multiply(opacity);
        ui.painter()
            .rect_filled(Rect::from_center_size(rect.center(), size), radius, color);
        if i.focused {
            ui.painter().add(
                FocusStroke::inside(Stroke::new(g::STROKE_WIDTH_THICK, t.color(ColorStrokeFocus2)))
                    .shape(rect, Radius::Medium.on(rect)),
            );
        }
        WidgetResponse::new(r.clone(), ()).with_changed(r.clicked())
    }

    fn semantics(&self, _: &I18n, response: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(
            egui_mineral::agent_key!("carouselnav").id(&self.salt),
            Role::Tab,
            &self.label,
        )
        .clickable(Availability::Enabled);
        n.state.selected = Some(self.selected);
        n.state.focused = response.has_focus();
        Some(n)
    }
}

/// Places the slides of a [`Carousel`].
pub struct CarouselUi<'u, 's, K> {
    ui: &'u mut MineralUi<'s, Fluent>,
    salt: &'u KeyedSalt,
    active: &'u K,
    appearance: CarouselAppearance,
    keys: Vec<K>,
    labels: Vec<String>,
    height: f32,
}

kit::opaque_debug!(CarouselUi<'_, '_, K>);

impl<K: ItemKey> CarouselUi<'_, '_, K> {
    /// `CarouselCard` carrying `key`; `add` runs only while it is the active slide. Agent key
    /// `<id_salt>-<key>-carouselcard`.
    pub fn card<R>(&mut self, key: K, card: impl Into<CarouselCard>, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        let card = card.into();
        self.labels.push(card.label.clone());
        let active = key == *self.active;
        let own = self.salt.child(&key);
        self.keys.push(key);
        if !active {
            return None;
        }
        let elevated = self.appearance == CarouselAppearance::Elevated;
        let t = self.ui.tokens();
        let width = self.ui.available_width();
        let inset = if elevated { d::ELEVATED_INSET } else { 0.0 };
        let top = self.ui.cursor().min;
        let area = Rect::from_min_size(top, vec2(width, self.height.max(1.0)));
        let content = area.shrink(inset);
        let mut child = self.ui.new_child(UiBuilder::new().max_rect(content));
        let out = add(&mut child);
        let used = child.min_rect().height() + 2.0 * inset;
        self.height = self.height.max(used);
        let rect = Rect::from_min_size(top, vec2(width, self.height));
        if elevated {
            let card_rect = rect.shrink(inset);
            let radius = Radius::XLarge.on(card_rect);
            paint::shadow(self.ui.painter(), card_rect, radius, Elevation::E16, &t);
        }
        let (_, response) = self.ui.allocate_exact_size(rect.size(), Sense::hover());
        let key = egui_mineral::agent_key!("carouselcard").id(&own);
        semantic::emit(self.ui.ctx(), None, response.rect, || {
            SemanticNode::new(key, Role::Group, card.label)
        });
        Some(out)
    }
}

/// Result of showing a [`Carousel`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CarouselResponse<K> {
    /// Spans the slide and the navigation; `mark_changed` when `changed`.
    pub response: egui::Response,
    /// Agrees with `response.changed()`; the active card moved this pass.
    pub changed: bool,
    /// Key of the card made active this pass.
    pub selected: Option<K>,
    /// Cards placed this pass.
    pub len: usize,
}

/// Fluent `Carousel`: slides with previous and next buttons and a navigation of dots. Only the active card runs
/// its content. The navigation dots are one Tab stop; arrows move and select. Autoplay is off unless
/// [`Self::autoplay`] adds the play button, and it never advances while reduced motion is on.
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct Carousel<'a, K> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    active: &'a mut K,
    appearance: CarouselAppearance,
    circular: bool,
    brand: bool,
    autoplay: Option<Duration>,
    playing: bool,
    buttons: [Option<String>; 3],
}

impl<'a, K: ItemKey> Carousel<'a, K> {
    /// `label` names the carousel for assistive technology; `active` is the key of the shown card, snapped to the
    /// first card when no card carries it. Agent key `<id_salt>-carousel`, dots `<id_salt>-<key>-carouselnav`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, active: &'a mut K) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("carousel"),
            label: label.into(),
            active,
            appearance: CarouselAppearance::default(),
            circular: false,
            brand: false,
            autoplay: None,
            playing: false,
            buttons: [None, None, None],
        }
    }

    /// Default [`CarouselAppearance::Default`].
    pub fn appearance(mut self, appearance: CarouselAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// `circular`: next on the last card wraps to the first and previous on the first to the last; default off.
    pub fn circular(mut self, circular: bool) -> Self {
        self.circular = circular;
        self
    }

    /// Selected dot in the brand color; default off.
    pub fn brand(mut self, brand: bool) -> Self {
        self.brand = brand;
        self
    }

    /// Adds the autoplay button; while playing, the next card shows every `interval`.
    pub fn autoplay(mut self, interval: Duration) -> Self {
        self.autoplay = Some(interval);
        self
    }

    /// Autoplay starts playing instead of paused; default off.
    pub fn autoplay_playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    /// Accessible name of the previous button; default "Previous slide".
    pub fn previous_label(mut self, previous_label: impl Into<String>) -> Self {
        self.buttons[0] = Some(previous_label.into());
        self
    }

    /// Accessible name of the next button; default "Next slide".
    pub fn next_label(mut self, next_label: impl Into<String>) -> Self {
        self.buttons[1] = Some(next_label.into());
        self
    }

    /// Accessible name of the autoplay button; default "Autoplay".
    pub fn autoplay_label(mut self, autoplay_label: impl Into<String>) -> Self {
        self.buttons[2] = Some(autoplay_label.into());
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-carousel` agent key.
        agent_key(key)
    }

    /// Shows the cards `add` places and the navigation below them.
    pub fn show(
        self,
        ui: &mut MineralUi<'_, Fluent>,
        add: impl FnOnce(&mut CarouselUi<'_, '_, K>),
    ) -> CarouselResponse<K> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let memo_id = id.with("mineral::carousel-memo");
        let memo = Memo::load(ctx, memo_id);
        let mut keys = Vec::new();
        let mut labels = Vec::new();
        let (mut before, mut next) = (None, 0);
        let width = ui.available_width();
        // The tallest slide seen at this width; a new width measures again.
        let reserved = if (memo.width - width).abs() > 0.5 {
            0.0
        } else {
            memo.height
        };
        let mut height = reserved;
        let mut nav_width = memo.nav_width;
        let group = ui
            .push_id(self.salt.id_salt(), |ui| {
                ui.scope_builder(UiBuilder::new().layout(rt.i18n().direction().column()), |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let mut cards = CarouselUi {
                        ui: &mut rt.ui(ui),
                        salt: &self.salt,
                        active: &*self.active,
                        appearance: self.appearance,
                        keys: Vec::new(),
                        labels: Vec::new(),
                        height: reserved,
                    };
                    add(&mut cards);
                    (keys, labels, height) = (cards.keys, cards.labels, cards.height);
                    before = keys.iter().position(|k| k == &*self.active);
                    next = before.unwrap_or(0);
                    nav_width = self.navigation(&mut rt.ui(ui), id, (&keys, &labels), memo.nav_width, &mut next);
                })
            })
            .response;
        let seen = Memo {
            height,
            nav_width,
            width,
        };
        seen.store(ctx, memo_id);
        if seen.differs(&memo) && ctx.current_pass_index() == 0 {
            ctx.request_discard("mineral: carousel size");
        }
        let changed = before != Some(next) && !keys.is_empty();
        if let Some(key) = keys.get(next).filter(|_| changed) {
            self.active.clone_from(key);
            if before.is_none() {
                ctx.request_discard("mineral: carousel active card");
            }
        }
        let (key, label) = (self.key.id(&self.salt), self.label);
        semantic::emit(ctx, Some(group.id), group.rect, || {
            SemanticNode::new(key, Role::Group, label)
        });
        let mut response = group;
        if changed {
            response.mark_changed();
        }
        CarouselResponse {
            response,
            changed,
            selected: keys.get(next).filter(|_| changed).cloned(),
            len: labels.len(),
        }
    }

    fn step(&self, from: usize, len: usize, forward: bool) -> usize {
        let last = len.saturating_sub(1);
        match (forward, self.circular) {
            (true, true) => (from + 1) % len.max(1),
            (true, false) => (from + 1).min(last),
            (false, true) => from.checked_sub(1).unwrap_or(last),
            (false, false) => from.saturating_sub(1),
        }
    }

    // Row of autoplay, previous, dots and next, centered by the previous pass's width; returns that width.
    fn navigation(
        &self,
        ui: &mut MineralUi<'_, Fluent>,
        id: Id,
        (keys, labels): (&[K], &[String]),
        width: f32,
        next: &mut usize,
    ) -> f32 {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let len = labels.len();
        let dir = rt.i18n().direction();
        ui.add_space(d::NAV_MARGIN_TOP);
        let state_id = id.with("mineral::carousel-autoplay");
        let mut auto = Autoplay::load_or(ctx, state_id, || Autoplay {
            playing: self.playing,
            since: ctx.input(|i| i.time),
        });
        let reduced = rt.core().motion().reduced();
        let tab_stop = focus::tab_stop(ctx, id, Some(*next), &focus::Items::all(len));
        let mut ids = Vec::with_capacity(len);
        let mut picked = None;
        let lead = ((ui.available_width() - width) / 2.0).max(0.0);
        let row_width = ui.available_width();
        let used = ui
            .allocate_ui_with_layout(vec2(row_width, NAV_HEIGHT), dir.row(), |ui| {
                ui.spacing_mut().item_spacing.x = d::NAV_MARGIN;
                ui.add_space(lead);
                let start = ui.cursor().min.x;
                let label = |at: usize, id: crate::Msg| {
                    let over = self.buttons.get(at).and_then(Option::as_deref);
                    crate::tr_or(rt.i18n(), over, id)
                };
                let button = |ui: &mut Ui, salt: &'static str, label: &str, icon, disabled: bool| {
                    rt.ui(ui)
                        .mineral(
                            PushButton::new(self.salt.child(salt), label)
                                .icon(icon)
                                .label_visible(false)
                                .appearance(ButtonAppearance::Subtle)
                                .shape(ButtonShape::Circular)
                                .disabled(disabled),
                        )
                        .clicked()
                };
                if let Some(interval) = self.autoplay {
                    let icon = if auto.playing {
                        rt.icons().pause
                    } else {
                        rt.icons().play
                    };
                    if button(ui, "autoplay", &label(2, msg::CAROUSEL_AUTOPLAY), icon, reduced) {
                        auto.playing = !auto.playing;
                        auto.since = ctx.input(|i| i.time);
                    }
                    if auto.playing && !reduced && len > 1 {
                        let now = ctx.input(|i| i.time);
                        let due = auto.since + interval.as_secs_f64();
                        if now >= due {
                            picked = Some(self.step(*next, len, true));
                            auto.since = now;
                        } else if ui.is_rect_visible(ui.max_rect()) {
                            ctx.request_repaint_after(Duration::try_from_secs_f64(due - now).unwrap_or(interval));
                        }
                    }
                }
                let (prev, following) = match dir {
                    egui_mineral::i18n::Direction::Ltr => (rt.icons().chevron_left, rt.icons().chevron_right),
                    egui_mineral::i18n::Direction::Rtl => (rt.icons().chevron_right, rt.icons().chevron_left),
                };
                let at_start = !self.circular && *next == 0;
                if button(ui, "previous", &label(0, msg::CAROUSEL_PREVIOUS), prev, at_start) {
                    picked = Some(self.step(*next, len, false));
                }
                for (i, (key, label)) in keys.iter().zip(labels).enumerate() {
                    let out = rt.ui(ui).mineral(Dot {
                        salt: self.salt.child(key),
                        label: label.clone(),
                        selected: i == *next,
                        brand: self.brand,
                        tab_stop: focus::TabStop::at(i, tab_stop),
                    });
                    if out.changed() {
                        picked = Some(i);
                    }
                    ids.push(out.id);
                }
                let at_end = !self.circular && *next + 1 >= len;
                if button(ui, "next", &label(1, msg::CAROUSEL_NEXT), following, at_end) {
                    picked = Some(self.step(*next, len, true));
                }
                (ui.cursor().min.x - start).abs()
            })
            .inner;
        if let Some(i) = RovingFocus::new(Axis::Horizontal)
            .wrap(true)
            .update_group(ui.core(), id, &ids, |_| true)
        {
            picked = Some(i);
        }
        if let Some(i) = picked {
            *next = i.min(len.saturating_sub(1));
            auto.since = ctx.input(|i| i.time);
        }
        auto.store(ctx, state_id);
        used
    }
}

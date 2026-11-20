use crate::msg;
use crate::tokens::ShadowTint;
use egui::{Align, Frame, Margin, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder, pos2, vec2};
use egui_mineral::{
    layout::Axis,
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{Availability, Interaction, MineralUi},
};

use crate::spec::ColorToken::{
    ColorNeutralBackground1, ColorNeutralBackground1Hover, ColorNeutralBackground1Pressed,
    ColorNeutralBackground1Selected, ColorNeutralBackground2, ColorNeutralBackground2Hover,
    ColorNeutralBackground2Pressed, ColorNeutralBackground2Selected, ColorNeutralBackgroundDisabled,
    ColorNeutralForeground1, ColorNeutralStroke1, ColorNeutralStroke1Hover, ColorNeutralStroke1Pressed,
    ColorNeutralStroke1Selected, ColorNeutralStrokeDisabled, ColorSubtleBackground, ColorSubtleBackgroundHover,
    ColorSubtleBackgroundPressed, ColorSubtleBackgroundSelected, ColorTransparentBackground,
    ColorTransparentBackgroundHover, ColorTransparentBackgroundPressed, ColorTransparentBackgroundSelected,
    ColorTransparentStroke,
};
use crate::{
    Fluent, FluentExt, Icon, Rt, Size,
    data::Text,
    inputs::Checkbox,
    spec::ColorToken,
    spec::{defaults::card as d, tokens::geometry},
    theme::{Elevation, Radius},
    typography::TypeRamp,
    widget::{focus, paint, state::State},
};

/// Card `appearance`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CardAppearance {
    /// `filled`.
    #[default]
    Filled,
    /// `filled-alternative`.
    FilledAlternative,
    /// `outline`.
    Outline,
    /// `subtle`.
    Subtle,
}

impl CardAppearance {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Filled, Self::FilledAlternative, Self::Outline, Self::Subtle];
}

/// Content of a Fluent `CardHeader`.
#[derive(Clone, Debug)]
#[must_use]
pub struct CardHeader {
    title: String,
    description: Option<String>,
    image: Option<Icon>,
}

impl CardHeader {
    /// Header text.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            image: None,
        }
    }

    /// `description` under the title.
    pub fn secondary_text(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// `image` before the text.
    pub fn image(mut self, image: impl Into<Icon>) -> Self {
        self.image = Some(image.into());
        self
    }
}

/// Result of showing a [`Card`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CardResponse<R> {
    /// Whole card; hover only unless interactive or selectable.
    pub response: Response,
    /// Value of the content closure.
    pub inner: R,
    /// Clicked, or Space or Enter pressed on it, this pass.
    pub clicked: bool,
    /// Selected state flipped this pass, by the card or its checkbox.
    pub changed: bool,
}

/// Fluent `Card`: a surface grouping a preview, header, body and footer.
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct Card<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: Option<String>,
    appearance: CardAppearance,
    orientation: Axis,
    size: Size,
    interactive: bool,
    selected: Option<&'a mut bool>,
    checkbox: bool,
    disabled: bool,
}

impl<'a> Card<'a> {
    /// Filled, vertical, medium; agent key `<id_salt>-card`.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("card"),
            label: None,
            appearance: CardAppearance::Filled,
            orientation: Axis::Vertical,
            size: Size::Medium,
            interactive: false,
            selected: None,
            checkbox: false,
            disabled: false,
        }
    }

    /// Accessible name; defaults to the localized "Card".
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Default [`CardAppearance::Filled`].
    pub fn appearance(mut self, appearance: CardAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// `orientation`: [`Axis::Vertical`] (default) stacks the parts, [`Axis::Horizontal`] puts them in a row.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.orientation = axis;
        self
    }

    /// Default [`Size::Medium`].
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Hover and press states; a click is reported in [`CardResponse::clicked`].
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// `selected` state toggled by clicks and Space or Enter.
    pub fn selectable(mut self, selected: &'a mut bool) -> Self {
        self.selected = Some(selected);
        self
    }

    /// Shows the floating selection checkbox of a selectable card.
    pub fn checkbox(mut self, checkbox: bool) -> Self {
        self.checkbox = checkbox;
        self
    }

    crate::widget::setters! {
        /// Not interactive.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-card` agent key.
        agent_key(key)
    }

    // react-card useCardStyles.styles.ts filled, filledAlternative, outline, subtle
    const fn rest(&self) -> (ColorToken, ColorToken, Option<Elevation>) {
        match self.appearance {
            CardAppearance::Filled => (ColorNeutralBackground1, ColorTransparentStroke, Some(Elevation::E4)),
            CardAppearance::FilledAlternative => (ColorNeutralBackground2, ColorTransparentStroke, Some(Elevation::E4)),
            CardAppearance::Outline => (ColorTransparentBackground, ColorNeutralStroke1, None),
            CardAppearance::Subtle => (ColorSubtleBackground, ColorTransparentStroke, None),
        }
    }

    // `*Interactive` `:hover` and `:active`
    const fn engaged(&self, hover: bool) -> (ColorToken, Option<Elevation>) {
        let (hover_bg, pressed_bg, lift) = match self.appearance {
            CardAppearance::Filled => (ColorNeutralBackground1Hover, ColorNeutralBackground1Pressed, true),
            CardAppearance::FilledAlternative => (ColorNeutralBackground2Hover, ColorNeutralBackground2Pressed, true),
            CardAppearance::Outline => (
                ColorTransparentBackgroundHover,
                ColorTransparentBackgroundPressed,
                false,
            ),
            CardAppearance::Subtle => (ColorSubtleBackgroundHover, ColorSubtleBackgroundPressed, false),
        };
        let elevation = if !lift {
            None
        } else if hover {
            Some(Elevation::E8)
        } else {
            Some(Elevation::E4)
        };
        (if hover { hover_bg } else { pressed_bg }, elevation)
    }

    // `*InteractiveSelected`
    const fn selected_fill(&self) -> ColorToken {
        match self.appearance {
            CardAppearance::Filled => ColorNeutralBackground1Selected,
            CardAppearance::FilledAlternative => ColorNeutralBackground2Selected,
            CardAppearance::Outline => ColorTransparentBackgroundSelected,
            CardAppearance::Subtle => ColorSubtleBackgroundSelected,
        }
    }

    const fn pad(&self) -> f32 {
        match self.size {
            Size::Small => d::SIZE_SMALL,
            Size::Medium => d::SIZE_MEDIUM,
            Size::Large => d::SIZE_LARGE,
        }
    }

    const fn radius(&self) -> Radius {
        match self.size {
            Size::Small => Radius::Small,
            Size::Medium => Radius::Medium,
            Size::Large => Radius::Large,
        }
    }

    fn colors(&self, s: State, selected: bool) -> (ColorToken, ColorToken, Option<Elevation>) {
        let (mut fill, mut border, mut elevation) = self.rest();
        if s.engaged() {
            (fill, elevation) = self.engaged(s == State::Hover);
            if self.appearance == CardAppearance::Outline {
                border = if s == State::Pressed {
                    ColorNeutralStroke1Pressed
                } else {
                    ColorNeutralStroke1Hover
                };
            }
        }
        if selected && !self.disabled {
            fill = self.selected_fill();
            border = ColorNeutralStroke1Selected;
        }
        if self.disabled {
            let outline = self.appearance == CardAppearance::Outline;
            fill = if outline {
                ColorTransparentBackground
            } else {
                ColorNeutralBackgroundDisabled
            };
            border = ColorNeutralStrokeDisabled;
            elevation = (!outline).then_some(Elevation::E2);
        }
        (fill, border, elevation)
    }

    fn select_checkbox(&mut self, rt: &Rt, ui: &mut MineralUi<'_, Fluent>, rect: Rect) -> bool {
        let dir = rt.i18n().direction();
        let Some(sel) = self.selected.as_deref_mut() else {
            return false;
        };
        let label = crate::tr(rt.i18n(), msg::CARD_SELECT);
        let inset = d::SELECT_INSET;
        let place = Rect::from_min_max(
            pos2(rect.left() + inset, rect.top() + inset),
            pos2(rect.right() - inset, rect.bottom() - inset),
        );
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(place)
                .layout(dir.reversed().row().with_cross_align(Align::Min)),
        );
        let before = *sel;
        rt.ui(&mut child).mineral(
            Checkbox::new(self.salt.child("select"), label, sel)
                .label_visible(false)
                .disabled(self.disabled),
        );
        *sel != before
    }

    fn paint(
        &self,
        ui: &MineralUi<'_, Fluent>,
        bg: egui::layers::ShapeIdx,
        rect: Rect,
        s: State,
        selected: bool,
        focused: bool,
    ) {
        let t = ui.tokens();
        let radius = self.radius().on(rect);
        let (fill, border, elevation) = self.colors(s, selected);
        let mut shapes: Vec<Shape> = Vec::new();
        if let Some(e) = elevation {
            shapes.extend(
                t.shadow(e, ShadowTint::Neutral)
                    .layers()
                    .into_iter()
                    .map(|l| Shape::from(l.as_shape(rect, radius))),
            );
        }
        shapes.push(Shape::rect_filled(rect, radius, t.color(fill)));
        shapes.push(Shape::rect_stroke(
            rect,
            radius,
            Stroke::new(geometry::STROKE_WIDTH_THIN, t.color(border)),
            StrokeKind::Inside,
        ));
        ui.painter().set(bg, Shape::Vec(shapes));
        if focused {
            focus::outline(ui.painter(), rect.shrink(d::FOCUS_OFFSET), radius, &t);
        }
    }

    /// Shows the card across the available width; `add` places its parts.
    pub fn show<R>(
        mut self,
        ui: &mut MineralUi<'_, Fluent>,
        add: impl FnOnce(&mut CardUi<'_, '_>) -> R,
    ) -> CardResponse<R> {
        let rt = ui.rt();
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let pad = self.pad();
        let width = ui.available_width();
        let activatable = !self.disabled && (self.interactive || self.selected.is_some());
        let sense = if activatable { Sense::click() } else { Sense::hover() };
        let height_id = id.with("height");
        let known: Option<f32> = ui.ctx().data(|d| d.get_temp(height_id));
        let bg = ui.painter().add(Shape::Noop);
        let horizontal = self.orientation == Axis::Horizontal;
        let layout = if horizontal {
            dir.row()
                .with_cross_align(if known.is_some() { Align::Center } else { Align::Min })
        } else {
            dir.column()
        };
        let text_color = t.color(if self.disabled {
            ColorToken::ColorNeutralForegroundDisabled
        } else {
            ColorNeutralForeground1
        });
        let salt = self.salt.clone();
        let scope = ui.scope_builder(UiBuilder::new().id_salt(id).sense(sense).layout(layout), |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            ui.set_width(width);
            if let (true, Some(h)) = (horizontal, known) {
                ui.set_max_height(h);
            }
            ui.visuals_mut().override_text_color = Some(text_color);
            let mut card = CardUi {
                ui,
                rt,
                salt,
                pad,
                horizontal,
                width,
                height: known,
                count: 0,
                last_preview: false,
            };
            let inner = add(&mut card);
            card.finish();
            inner
        });
        let response = scope.response;
        let rect = response.rect;
        ui.ctx().data_mut(|d| d.insert_temp(height_id, rect.height()));
        let availability = Availability::disabled_if(self.disabled);
        let mut clicked = false;
        let mut changed = false;
        if activatable && response.clicked() {
            clicked = true;
            if let Some(sel) = self.selected.as_deref_mut() {
                *sel = !*sel;
                changed = true;
            }
        }
        let selected = self.selected.as_deref().copied().unwrap_or(false);
        let i = Interaction::of(&response, availability);
        let s = if activatable { State::of(i) } else { State::Rest };
        self.paint(ui, bg, rect, s, selected, i.focused && activatable);
        if self.checkbox {
            let flipped = self.select_checkbox(rt, ui, rect);
            changed |= flipped;
            clicked |= flipped;
        }
        let label = self.label.take().unwrap_or_else(|| crate::tr(rt.i18n(), msg::CARD));
        let key = self.key.id(&self.salt);
        let selectable = self.selected.is_some();
        semantic::emit(ui.ctx(), Some(response.id), rect, || {
            let mut n = SemanticNode::new(key, Role::Group, label);
            if activatable {
                n = n.clickable(availability);
            } else {
                n.state.disabled = !availability.is_enabled();
            }
            if selectable {
                n.state.selected = Some(selected);
            }
            n
        });
        CardResponse {
            response,
            inner: scope.inner,
            clicked,
            changed,
        }
    }
}

/// Places the parts of a [`Card`].
pub struct CardUi<'u, 'r> {
    ui: &'u mut Ui,
    rt: &'r Rt,
    salt: KeyedSalt,
    pad: f32,
    horizontal: bool,
    width: f32,
    height: Option<f32>,
    count: usize,
    last_preview: bool,
}

egui_mineral::kit::opaque_debug!(CardUi<'_, '_>);

impl CardUi<'_, '_> {
    fn part<R>(&mut self, preview: bool, add: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R) -> R {
        if self.count > 0 || !preview {
            self.ui.add_space(self.pad);
        }
        self.count += 1;
        self.last_preview = preview;
        let pad = self.pad as i8;
        let margin = match (preview, self.horizontal) {
            (true, _) => Margin::ZERO,
            (false, false) => Margin::symmetric(pad, 0),
            (false, true) => Margin::symmetric(0, pad),
        };
        let rt = self.rt;
        let (width, height, horizontal) = (self.width, self.height, self.horizontal);
        Frame::NONE
            .inner_margin(margin)
            .show(self.ui, |ui| {
                if preview && !horizontal {
                    ui.set_min_width(width);
                } else if preview && let Some(h) = height {
                    ui.set_min_height(h);
                }
                add(&mut rt.ui(ui))
            })
            .inner
    }

    fn finish(&mut self) {
        if self.count == 0 || !self.last_preview {
            self.ui.add_space(self.pad);
        }
    }

    /// `CardPreview`: edge to edge across the card.
    pub fn preview<R>(&mut self, add: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R) -> R {
        self.part(true, add)
    }

    /// Part padded by the card size across the card.
    pub fn body<R>(&mut self, add: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R) -> R {
        self.part(false, add)
    }

    /// `CardFooter`: a row of actions with the 12pt gap.
    pub fn footer<R>(&mut self, add: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R) -> R {
        let rt = self.rt;
        self.part(false, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = d::FOOTER_GAP;
                add(&mut rt.ui(ui))
            })
            .inner
        })
    }

    /// `CardHeader`: image, title and description, with an `action` slot at the end.
    pub fn header<R>(&mut self, header: &CardHeader, action: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R) -> R {
        let salt = self.salt.child("header");
        let rt = self.rt;
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        self.part(false, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = d::HEADER_GAP;
                if let Some(icon) = header.image {
                    let (r, _) = ui.allocate_exact_size(vec2(d::IMAGE_SIZE, d::IMAGE_SIZE), Sense::hover());
                    paint::icon(
                        ui.painter(),
                        r.center(),
                        icon,
                        false,
                        d::IMAGE_SIZE,
                        t.color(ColorNeutralForeground1),
                    );
                }
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let ui = &mut rt.ui(ui);
                    ui.mineral(Text::new(salt.child("title"), header.title.clone()).ramp(TypeRamp::Body1Strong));
                    if let Some(desc) = &header.description {
                        ui.mineral(Text::new(salt.child("description"), desc.clone()).ramp(TypeRamp::Caption1));
                    }
                });
                ui.with_layout(dir.row().with_main_align(Align::Max), |ui| action(&mut rt.ui(ui)))
                    .inner
            })
            .inner
        })
    }
}

use egui::{Color32, CornerRadius, ImageSource, Sense, Stroke, StrokeKind, Ui, vec2};
use egui_mineral::{
    focus::{self, RovingFocus},
    i18n::I18n,
    kit,
    layout::{Axis, Constraints, Measure, MinTarget},
    select::SelectionResponse,
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode, StableId},
    widget::{Availability, Interaction, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::ColorToken::{
    ColorBrandStroke1, ColorCompoundBrandStrokeHover, ColorCompoundBrandStrokePressed, ColorNeutralForegroundInverted,
    ColorStrokeFocus1, ColorStrokeFocus2, ColorTransparentStroke,
};
use crate::{
    ColorToken, Fluent, FluentExt,
    spec::{
        defaults::swatch_picker as sp,
        tokens::geometry::{
            BORDER_RADIUS_CIRCULAR, BORDER_RADIUS_MEDIUM, BORDER_RADIUS_NONE, STROKE_WIDTH_THICK, STROKE_WIDTH_THICKER,
            STROKE_WIDTH_THICKEST, STROKE_WIDTH_THIN,
        },
    },
    widget::paint,
};

/// `size` of a swatch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SwatchSize {
    /// `extra-small`: 20.
    ExtraSmall,
    /// `small`: 24.
    Small,
    /// `medium`: 28.
    #[default]
    Medium,
    /// `large`: 32.
    Large,
}

impl SwatchSize {
    /// Every variant, smallest first.
    pub const ALL: &'static [Self] = &[Self::ExtraSmall, Self::Small, Self::Medium, Self::Large];

    const fn side(self) -> f32 {
        match self {
            Self::ExtraSmall => sp::SIZE_EXTRA_SMALL,
            Self::Small => sp::SIZE_SMALL,
            Self::Medium => sp::SIZE_MEDIUM,
            Self::Large => sp::SIZE_LARGE,
        }
    }

    const fn icon(self) -> f32 {
        match self {
            Self::ExtraSmall | Self::Small => sp::ICON_SMALL,
            Self::Medium => sp::ICON_MEDIUM,
            Self::Large => sp::ICON_LARGE,
        }
    }

    const fn compact(self) -> bool {
        matches!(self, Self::ExtraSmall | Self::Small)
    }
}

/// `shape` of a swatch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SwatchShape {
    /// `rounded`: `borderRadiusMedium`.
    Rounded,
    /// `circular`: pill.
    Circular,
    /// `square`: no rounding.
    #[default]
    Square,
}

impl SwatchShape {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Rounded, Self::Circular, Self::Square];

    fn radius(self) -> CornerRadius {
        CornerRadius::from(match self {
            Self::Rounded => BORDER_RADIUS_MEDIUM,
            Self::Circular => BORDER_RADIUS_CIRCULAR,
            Self::Square => BORDER_RADIUS_NONE,
        })
    }
}

/// `spacing` between swatches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SwatchSpacing {
    /// `small`: 2.
    Small,
    /// `medium`: 4.
    #[default]
    Medium,
}

/// `layout` of a [`SwatchPicker`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SwatchLayout {
    /// `row`: one wrapping row.
    #[default]
    Row,
    /// `grid`: rows of `columns` swatches.
    Grid {
        /// Swatches per row; 0 counts as 1.
        columns: usize,
    },
}

#[derive(Clone, Debug)]
enum Fill {
    Color(Color32),
    Image(ImageSource<'static>),
}

/// Fluent `ColorSwatch` or `ImageSwatch`: a choice of a [`SwatchPicker`].
#[derive(Clone, Debug)]
#[must_use = "add to a SwatchPicker"]
pub struct Swatch {
    value: String,
    label: String,
    fill: Fill,
    disabled: bool,
    salt: Option<KeyedSalt>,
}

impl Swatch {
    /// `ColorSwatch`: `value` identifies it, `label` is the accessible name.
    pub fn color(value: impl Into<String>, label: impl Into<String>, color: Color32) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            fill: Fill::Color(color),
            disabled: false,
            salt: None,
        }
    }

    /// `ImageSwatch`: the image fills the swatch.
    pub fn image(value: impl Into<String>, label: impl Into<String>, source: impl Into<ImageSource<'static>>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            fill: Fill::Image(source.into()),
            disabled: false,
            salt: None,
        }
    }

    crate::widget::setters! {
        item_salt
    }

    /// Shows the disabled icon and ignores clicks.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Fluent `SwatchPicker`: single selection among swatches, one Tab stop, arrow-key navigation.
#[derive(Debug)]
#[must_use = "show with SwatchPicker::show"]
pub struct SwatchPicker<'a> {
    salt: KeyedSalt,
    label: String,
    value: &'a mut Option<String>,
    swatches: Vec<Swatch>,
    layout: SwatchLayout,
    size: SwatchSize,
    shape: SwatchShape,
    spacing: SwatchSpacing,
    disabled: bool,
}

impl<'a> SwatchPicker<'a> {
    /// `label` names the group for assistive technology; node key `<id_salt>-swatch-picker`, swatches
    /// `<id_salt>-<index>-swatch`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, value: &'a mut Option<String>) -> Self {
        Self {
            salt: id_salt.into(),
            label: label.into(),
            value,
            swatches: Vec::new(),
            layout: SwatchLayout::Row,
            size: SwatchSize::Medium,
            shape: SwatchShape::Square,
            spacing: SwatchSpacing::Medium,
            disabled: false,
        }
    }

    /// Adds swatches in order.
    pub fn swatches(mut self, swatches: impl IntoIterator<Item = Swatch>) -> Self {
        self.swatches.extend(swatches);
        self
    }

    /// Default [`SwatchLayout::Row`].
    pub fn layout(mut self, layout: SwatchLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Default [`SwatchSize::Medium`].
    pub fn size(mut self, size: SwatchSize) -> Self {
        self.size = size;
        self
    }

    /// Default [`SwatchShape::Square`].
    pub fn shape(mut self, shape: SwatchShape) -> Self {
        self.shape = shape;
        self
    }

    /// Default [`SwatchSpacing::Medium`].
    pub fn spacing(mut self, spacing: SwatchSpacing) -> Self {
        self.spacing = spacing;
        self
    }

    crate::widget::setters! {
        /// Disables every swatch.
        disabled
    }

    /// Lays out the swatches; a pick writes `value` and reports `changed`.
    pub fn show(self, ui: &mut MineralUi<'_, Fluent>) -> SelectionResponse {
        let rt = ui.rt();
        let Self {
            salt,
            label,
            value,
            swatches,
            layout,
            size,
            shape,
            spacing,
            disabled,
        } = self;
        let gap = match spacing {
            SwatchSpacing::Small => sp::SPACING_SMALL,
            SwatchSpacing::Medium => sp::SPACING_MEDIUM,
        };
        let columns = match layout {
            SwatchLayout::Row => swatches.len().max(1),
            SwatchLayout::Grid { columns } => columns.max(1),
        };
        let mut ids = Vec::with_capacity(swatches.len());
        let mut picked = None;
        let group_id = ui.make_persistent_id(salt.id_salt());
        let enabled = |i: usize| !disabled && swatches.get(i).is_some_and(|s| !s.disabled);
        let selected = swatches.iter().position(|s| value.as_deref() == Some(s.value.as_str()));
        let tab_stop = focus::tab_stop(ui.ctx(), group_id, selected, swatches.len(), enabled);
        let body = |ui: &mut Ui| {
            ui.spacing_mut().item_spacing = vec2(gap, gap);
            for (r, row) in swatches.chunks(columns).enumerate() {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(gap, gap);
                    for (c, s) in row.iter().enumerate() {
                        let i = r * columns + c;
                        let out = rt.ui(ui).mineral(Cell {
                            salt: s.salt.clone().unwrap_or_else(|| salt.child(i)),
                            swatch: s,
                            size,
                            shape,
                            selected: selected == Some(i),
                            tab_stop: i == tab_stop,
                            disabled: !enabled(i),
                        });
                        if out.changed() {
                            picked = Some(i);
                        }
                        ids.push(out.id);
                    }
                });
            }
        };
        let group = ui.push_id(salt.id_salt(), |ui| ui.vertical(body).response).inner;
        let roving = match layout {
            SwatchLayout::Row => RovingFocus::new(Axis::Horizontal),
            SwatchLayout::Grid { .. } => RovingFocus::grid(columns),
        };
        if let Some(i) = roving.update_group(ui.core(), group_id, &ids, enabled) {
            picked = Some(i);
        }
        let mut changed = false;
        if let Some(s) = picked.and_then(|i| swatches.get(i)) {
            changed = value.as_deref() != Some(s.value.as_str());
            *value = Some(s.value.clone());
        }
        semantic::emit(ui.ctx(), Some(group.id), group.rect, || {
            let mut n = SemanticNode::new(StableId::of(&salt, "swatch-picker"), Role::RadioGroup, &label);
            n.state.disabled = disabled;
            n
        });
        let response = group;
        SelectionResponse::new(response, picked.filter(|_| changed), changed)
    }
}

struct Cell<'a> {
    salt: KeyedSalt,
    swatch: &'a Swatch,
    size: SwatchSize,
    shape: SwatchShape,
    selected: bool,
    tab_stop: bool,
    disabled: bool,
}

type Rings = [(f32, ColorToken); 2];

// react-swatch-picker useColorSwatchStyles.styles.ts inset `box-shadow`s, topmost first.
fn rings(size: SwatchSize, selected: bool, i: Interaction) -> Option<Rings> {
    let compact = size.compact();
    let hover = i.hovered && !i.disabled;
    let pressed = i.pressed && !i.disabled;
    Some(if selected {
        if i.focused && !hover {
            [
                (STROKE_WIDTH_THICKER, ColorStrokeFocus2),
                (sp::SELECTED_RING, ColorStrokeFocus1),
            ]
        } else if compact {
            if pressed {
                [
                    (STROKE_WIDTH_THICKER, ColorCompoundBrandStrokePressed),
                    (STROKE_WIDTH_THICKEST, ColorStrokeFocus1),
                ]
            } else if hover {
                [
                    (STROKE_WIDTH_THICK, ColorCompoundBrandStrokeHover),
                    (STROKE_WIDTH_THICKER, ColorStrokeFocus1),
                ]
            } else {
                [
                    (STROKE_WIDTH_THICK, ColorBrandStroke1),
                    (STROKE_WIDTH_THICKER, ColorStrokeFocus1),
                ]
            }
        } else if pressed {
            [
                (STROKE_WIDTH_THICKEST, ColorCompoundBrandStrokePressed),
                (sp::SELECTED_RING_PRESSED, ColorStrokeFocus1),
            ]
        } else if hover {
            [
                (STROKE_WIDTH_THICKEST, ColorCompoundBrandStrokeHover),
                (sp::SELECTED_RING_HOVER, ColorStrokeFocus1),
            ]
        } else {
            [
                (STROKE_WIDTH_THICKER, ColorBrandStroke1),
                (sp::SELECTED_RING, ColorStrokeFocus1),
            ]
        }
    } else if pressed {
        if compact {
            [
                (STROKE_WIDTH_THICK, ColorCompoundBrandStrokePressed),
                (STROKE_WIDTH_THICKER, ColorStrokeFocus1),
            ]
        } else {
            [
                (STROKE_WIDTH_THICKER, ColorCompoundBrandStrokePressed),
                (STROKE_WIDTH_THICKEST, ColorStrokeFocus1),
            ]
        }
    } else if hover {
        if size == SwatchSize::ExtraSmall {
            [
                (STROKE_WIDTH_THIN, ColorBrandStroke1),
                (STROKE_WIDTH_THICK, ColorStrokeFocus1),
            ]
        } else {
            [
                (STROKE_WIDTH_THICK, ColorBrandStroke1),
                (STROKE_WIDTH_THICKER, ColorStrokeFocus1),
            ]
        }
    } else if i.focused {
        [
            (STROKE_WIDTH_THICK, ColorStrokeFocus2),
            (STROKE_WIDTH_THICKER, ColorStrokeFocus1),
        ]
    } else {
        return None;
    })
}

impl MineralWidget for Cell<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, enabled);

    fn sense(&self) -> Sense {
        focus::roving_sense(
            Sense::click(),
            if self.tab_stop {
                focus::TabStop::Stop
            } else {
                focus::TabStop::Skip
            },
        )
    }

    fn min_target(&self) -> MinTarget {
        MinTarget::None
    }

    fn layout(&self, _: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        c.constrain(vec2(self.size.side(), self.size.side())).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let r = &frame.response;
        let availability = Availability::disabled_if(self.disabled);
        kit::input::hover_cursor(ui.core(), r, availability);
        kit::focus::follow_press(r);
        let changed = r.clicked() && !self.disabled && !self.selected;
        let selected = self.selected || changed;
        let i = Interaction::of(r, availability);
        let rect = frame.zones.visual;
        let radius = self.shape.radius();
        match &self.swatch.fill {
            Fill::Color(c) => {
                ui.painter().rect_filled(rect, radius, *c);
            }
            Fill::Image(src) => {
                egui::Image::new(src.clone()).corner_radius(radius).paint_at(ui, rect);
            }
        }
        let rings = rings(self.size, selected, i);
        if rings.is_none() {
            ui.painter().rect_stroke(
                rect,
                radius,
                Stroke::new(STROKE_WIDTH_THIN, t.color(ColorTransparentStroke)),
                StrokeKind::Inside,
            );
        }
        for (w, c) in rings.into_iter().flatten().rev() {
            ui.painter()
                .rect_stroke(rect, radius, Stroke::new(w, t.color(c)), StrokeKind::Inside);
        }
        if self.disabled {
            paint::icon(
                ui.painter(),
                rect.center(),
                ui.icons().prohibited,
                false,
                self.size.icon(),
                t.color(ColorNeutralForegroundInverted),
            );
        }
        WidgetResponse::new(r.clone(), ()).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(
            AgentKey::Kind("swatch").id(&self.salt),
            Role::RadioButton,
            &self.swatch.label,
        )
        .clickable(Availability::disabled_if(self.disabled));
        n.state.checked = Some(self.selected.into());
        Some(n)
    }
}

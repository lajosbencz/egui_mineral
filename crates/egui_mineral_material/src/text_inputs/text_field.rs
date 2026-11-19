use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::Rt;
use crate::adaptive::Density;
use egui_mineral::i18n::I18n;
use egui_mineral::i18n::Sides;
use egui_mineral::kit::state::TempState;
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use std::{num::NonZeroUsize, sync::Arc};

use egui::{
    Color32, EventFilter, FontId, Frame, Galley, Id, Margin, Painter, Pos2, Rangef, Rect, Response, ScrollArea, Shape,
    Stroke, StrokeKind, TextBuffer, TextEdit, TextFormat, Ui, UiBuilder, Vec2,
    epaint::MarginF32,
    pos2,
    text::{LayoutJob, TextWrapping},
    vec2,
};
use egui_mineral::layout::{Constraints, Measure};
use egui_mineral::widget::Availability;
use egui_mineral::{
    edit::{EditBuffer, EditEvent, FieldFocus},
    i18n::Direction,
    semantic::{self, Action, NumericRange, Role, SemanticNode},
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::widget::common;
use crate::widget::parts;
use crate::{
    Icon, motion,
    spec::{
        ColorRole, TypeScale,
        defaults::text_field as tf,
        tokens::{filled_text_field, outlined_text_field, small_icon_button},
    },
    tokens::Tokens,
};

/// Compose `TextField` and `OutlinedTextField` styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum TextFieldVariant {
    #[default]
    /// `TextField`: filled container, bottom indicator line.
    Filled,
    /// `OutlinedTextField`: outline with the label in its notch.
    Outlined,
}

pub(crate) const INPUT_FONT: TypeScale = filled_text_field::INPUT_FONT;
const LABEL_FONT: TypeScale = filled_text_field::LABEL_FONT;
// Compose bodySmall
const SMALL_FONT: TypeScale = filled_text_field::SUPPORTING_FONT;

// Compose MinHeight, shrunk by density down to the label and one input line with padding.
pub(crate) fn field_h(density: Density, variant: TextFieldVariant, labeled: bool) -> f32 {
    let content = match (variant, labeled) {
        (TextFieldVariant::Filled, true) => {
            tf::LABEL_VERTICAL_PADDING
                + SMALL_FONT.line_height().max(tf::MIN_FOCUSED_LABEL_LINE_HEIGHT)
                + line_h()
                + 4.0
        }
        _ => line_h() + 8.0,
    };
    density.dense_to(tf::MIN_HEIGHT, content)
}

// Compose MinTextLineHeight
pub(crate) fn line_h() -> f32 {
    INPUT_FONT.line_height().max(tf::MIN_TEXT_LINE_HEIGHT)
}

fn small_line_h() -> f32 {
    SMALL_FONT.line_height().max(tf::MIN_SUPPORTING_LINE_HEIGHT)
}

pub(crate) fn support_h() -> f32 {
    tf::SUPPORTING_TOP_PADDING + small_line_h()
}

// Compose topPaddingForLabelCutout, BodySmall, minimizedLabelHalfHeight
pub(crate) fn label_cutout(variant: TextFieldVariant, labeled: bool) -> f32 {
    match variant {
        TextFieldVariant::Outlined if labeled => SMALL_FONT.line_height() / 2.0,
        TextFieldVariant::Filled | TextFieldVariant::Outlined => 0.0,
    }
}

pub(crate) fn measure(container: Vec2, variant: TextFieldVariant, labeled: bool, support: bool) -> Measure {
    Measure::new(container).margin(MarginF32 {
        top: label_cutout(variant, labeled),
        bottom: if support { support_h() } else { 0.0 },
        ..MarginF32::ZERO
    })
}

// Compose textPosition
pub(crate) fn text_top(density: Density, variant: TextFieldVariant, labeled: bool) -> f32 {
    let h = field_h(density, variant, labeled);
    match variant {
        TextFieldVariant::Filled if labeled => {
            // Density shrink moves filled content up by half of it.
            let lift = (tf::MIN_HEIGHT - h) / 2.0;
            tf::LABEL_VERTICAL_PADDING + SMALL_FONT.line_height().max(tf::MIN_FOCUSED_LABEL_LINE_HEIGHT) - lift
        }
        TextFieldVariant::Filled | TextFieldVariant::Outlined => (h - line_h()) / 2.0,
    }
}

// Compose minimumInteractiveComponentSize, shrunk by density down to the icon with padding.
pub(crate) fn icon_slot(rt: &Rt, density: Density) -> f32 {
    let slot = crate::adaptive::MIN_INTERACTIVE.points(rt.core());
    density.dense_to(slot, small_icon_button::ICON_SIZE + 8.0)
}

pub(crate) fn trailing_icon_size(variant: TextFieldVariant) -> f32 {
    match variant {
        TextFieldVariant::Filled => filled_text_field::TRAILING_ICON_SIZE,
        TextFieldVariant::Outlined => outlined_text_field::TRAILING_ICON_SIZE,
    }
}

// Compose textFieldHorizontalIconPadding
pub(crate) fn content_inset(rt: &Rt, density: Density, icon: bool) -> f32 {
    if icon {
        let slot = icon_slot(rt, density);
        let icon_padding = ((slot - small_icon_button::ICON_SIZE) / 2.0).max(0.0);
        slot + (tf::PADDING - icon_padding).max(0.0)
    } else {
        tf::PADDING
    }
}

// Compose TextFieldColors
#[derive(Clone, Copy, Debug)]
pub(crate) struct FieldColors {
    pub container: Color32,
    /// Filled active indicator or outline.
    pub indicator: Color32,
    pub label: Color32,
    pub input: Color32,
    pub placeholder: Color32,
    pub affix: Color32,
    pub leading: Color32,
    pub trailing: Color32,
    pub supporting: Color32,
    pub caret: Color32,
}

impl FieldColors {
    fn lerp(self, to: Self, k: f32) -> Self {
        let l = |a: Color32, b: Color32| a.lerp_to_gamma(b, k);
        Self {
            container: l(self.container, to.container),
            indicator: l(self.indicator, to.indicator),
            label: l(self.label, to.label),
            input: l(self.input, to.input),
            placeholder: l(self.placeholder, to.placeholder),
            affix: l(self.affix, to.affix),
            leading: l(self.leading, to.leading),
            trailing: l(self.trailing, to.trailing),
            supporting: l(self.supporting, to.supporting),
            caret: l(self.caret, to.caret),
        }
    }

    fn apply_editor(&self, ui: &mut Ui, t: &Tokens) {
        let v = ui.visuals_mut();
        v.text_cursor.stroke.color = self.caret;
        v.selection.bg_fill = FieldColors::selection(t);
    }
}

impl FieldColors {
    pub(crate) fn selection(t: &Tokens) -> Color32 {
        t.role_alpha(tf::SELECTION_COLOR, tf::SELECTION_OPACITY)
    }
}

// Editors that own arrow keys (list or stepper navigation) keep them from egui's focus navigation.
#[cfg(feature = "extras")]
pub(crate) const fn editor_keys(escape: bool) -> EventFilter {
    EventFilter {
        tab: false,
        horizontal_arrows: true,
        vertical_arrows: true,
        escape,
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FieldState {
    pub focused: bool,
    pub hovered: bool,
    pub error: bool,
    pub disabled: bool,
}

/// Token colors of the text field token module `$m` in state `$s`; `$ind` are the indicator (filled
/// active indicator or outline) roles in `pick` order.
macro_rules! field_colors {
    ($m:ident, $s:expr, $t:expr, $container:expr, $ind_dis:ident, $ind_dis_a:ident, [$($ind:ident),*]) => {{
        use $m as m;
        let s: FieldState = $s;
        let t: &Tokens = $t;
        // `[error focused, error hovered, error, focused, hovered, rest]`.
        let pick = |roles: [ColorRole; 6]| {
            t.role(roles[match (s.error, s.focused, s.hovered) {
                (true, true, _) => 0,
                (true, false, true) => 1,
                (true, false, false) => 2,
                (false, true, _) => 3,
                (false, false, true) => 4,
                (false, false, false) => 5,
            }])
        };
        if s.disabled {
            FieldColors {
                container: $container,
                indicator: t.role_alpha(m::$ind_dis, m::$ind_dis_a),
                label: t.role_alpha(m::DISABLED_LABEL_COLOR, m::DISABLED_LABEL_OPACITY),
                input: t.role_alpha(m::DISABLED_INPUT_COLOR, m::DISABLED_INPUT_OPACITY),
                placeholder: t.role_alpha(m::DISABLED_INPUT_COLOR, m::DISABLED_INPUT_OPACITY),
                affix: t.role_alpha(m::INPUT_PREFIX_COLOR, m::DISABLED_INPUT_OPACITY),
                leading: t.role_alpha(m::DISABLED_LEADING_ICON_COLOR, m::DISABLED_LEADING_ICON_OPACITY),
                trailing: t.role_alpha(m::DISABLED_TRAILING_ICON_COLOR, m::DISABLED_TRAILING_ICON_OPACITY),
                supporting: t.role_alpha(m::DISABLED_SUPPORTING_COLOR, m::DISABLED_SUPPORTING_OPACITY),
                caret: Color32::TRANSPARENT,
            }
        } else {
            FieldColors {
                container: $container,
                indicator: pick([$(m::$ind),*]),
                label: pick([
                    m::ERROR_FOCUS_LABEL_COLOR,
                    m::ERROR_HOVER_LABEL_COLOR,
                    m::ERROR_LABEL_COLOR,
                    m::FOCUS_LABEL_COLOR,
                    m::HOVER_LABEL_COLOR,
                    m::LABEL_COLOR,
                ]),
                input: pick([
                    m::ERROR_FOCUS_INPUT_COLOR,
                    m::ERROR_HOVER_INPUT_COLOR,
                    m::ERROR_INPUT_COLOR,
                    m::FOCUS_INPUT_COLOR,
                    m::HOVER_INPUT_COLOR,
                    m::INPUT_COLOR,
                ]),
                placeholder: t.role(m::INPUT_PLACEHOLDER_COLOR),
                affix: t.role(m::INPUT_PREFIX_COLOR),
                leading: pick([
                    m::ERROR_FOCUS_LEADING_ICON_COLOR,
                    m::ERROR_HOVER_LEADING_ICON_COLOR,
                    m::ERROR_LEADING_ICON_COLOR,
                    m::FOCUS_LEADING_ICON_COLOR,
                    m::HOVER_LEADING_ICON_COLOR,
                    m::LEADING_ICON_COLOR,
                ]),
                trailing: pick([
                    m::ERROR_FOCUS_TRAILING_ICON_COLOR,
                    m::ERROR_HOVER_TRAILING_ICON_COLOR,
                    m::ERROR_TRAILING_ICON_COLOR,
                    m::FOCUS_TRAILING_ICON_COLOR,
                    m::HOVER_TRAILING_ICON_COLOR,
                    m::TRAILING_ICON_COLOR,
                ]),
                supporting: pick([
                    m::ERROR_FOCUS_SUPPORTING_COLOR,
                    m::ERROR_HOVER_SUPPORTING_COLOR,
                    m::ERROR_SUPPORTING_COLOR,
                    m::FOCUS_SUPPORTING_COLOR,
                    m::HOVER_SUPPORTING_COLOR,
                    m::SUPPORTING_COLOR,
                ]),
                caret: t.role(if s.error { m::ERROR_FOCUS_CARET_COLOR } else { m::CARET_COLOR }),
            }
        }
    }};
}

// Compose TextFieldDefaults.colors, OutlinedTextFieldDefaults.colors
pub(crate) fn colors(variant: TextFieldVariant, s: FieldState, t: &Tokens) -> FieldColors {
    match variant {
        TextFieldVariant::Filled => {
            let container = if s.disabled {
                tf::FILLED_DISABLED_CONTAINER_COLOR
            } else {
                filled_text_field::CONTAINER_COLOR
            };
            field_colors!(
                filled_text_field,
                s,
                t,
                t.role(container),
                DISABLED_ACTIVE_INDICATOR_COLOR,
                DISABLED_ACTIVE_INDICATOR_OPACITY,
                [
                    ERROR_FOCUS_ACTIVE_INDICATOR_COLOR,
                    ERROR_HOVER_ACTIVE_INDICATOR_COLOR,
                    ERROR_ACTIVE_INDICATOR_COLOR,
                    FOCUS_ACTIVE_INDICATOR_COLOR,
                    HOVER_ACTIVE_INDICATOR_COLOR,
                    ACTIVE_INDICATOR_COLOR
                ]
            )
        }
        TextFieldVariant::Outlined => field_colors!(
            outlined_text_field,
            s,
            t,
            Color32::TRANSPARENT,
            DISABLED_OUTLINE_COLOR,
            DISABLED_OUTLINE_OPACITY,
            [
                ERROR_FOCUS_OUTLINE_COLOR,
                ERROR_HOVER_OUTLINE_COLOR,
                ERROR_OUTLINE_COLOR,
                FOCUS_OUTLINE_COLOR,
                HOVER_OUTLINE_COLOR,
                OUTLINE_COLOR
            ]
        ),
    }
}

fn indicator_widths(variant: TextFieldVariant) -> (f32, f32) {
    match variant {
        TextFieldVariant::Filled => (
            filled_text_field::ACTIVE_INDICATOR_HEIGHT,
            filled_text_field::FOCUS_ACTIVE_INDICATOR_HEIGHT,
        ),
        TextFieldVariant::Outlined => (
            outlined_text_field::OUTLINE_WIDTH,
            outlined_text_field::FOCUS_OUTLINE_WIDTH,
        ),
    }
}

pub(crate) fn line_galley(p: &Painter, text: &str, scale: TypeScale, color: Color32, max_width: f32) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(text.to_owned(), scale.format(p.ctx(), color));
    job.wrap = TextWrapping::truncate_at_width(max_width.max(0.0));
    p.layout_job(job)
}

/// Part of an overflowing one-line text replaced by an ellipsis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Elide {
    /// Cuts the end: `/home/user/pro...`.
    #[default]
    End,
    /// Cuts the middle: `/home/...ts/a.png`.
    Middle,
    /// Cuts the start, keeping e.g. a file name: `...shots/a.png`.
    Start,
}

const ELLIPSIS: char = '\u{2026}';

// One line of `text` at most `max_width` wide, `elide` choosing what overflow drops.
pub(crate) fn elided_galley(
    p: &Painter,
    text: &str,
    scale: TypeScale,
    color: Color32,
    max_width: f32,
    elide: Elide,
) -> Arc<Galley> {
    if elide == Elide::End {
        return line_galley(p, text, scale, color, max_width);
    }
    let format = scale.format(p.ctx(), color);
    let layout = |s: String| p.layout_job(LayoutJob::single_section(s, format.clone()));
    let full = layout(text.to_owned());
    if full.size().x <= max_width {
        return full;
    }
    let chars: Vec<char> = text.chars().collect();
    let candidate = |kept: usize| -> String {
        let (head, tail) = match elide {
            Elide::Start => (0, kept),
            Elide::Middle | Elide::End => (kept.div_ceil(2), kept / 2),
        };
        chars[..head]
            .iter()
            .chain(std::iter::once(&ELLIPSIS))
            .chain(&chars[chars.len() - tail..])
            .collect()
    };
    // Largest kept count that fits; widths grow with it.
    let (mut lo, mut hi) = (0, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if layout(candidate(mid)).size().x <= max_width {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    layout(candidate(lo))
}

// Compose bodyLarge, bodySmall, lerp(TextStyle)
fn label_format(ctx: &egui::Context, k: f32, color: Color32) -> TextFormat {
    let (from, to) = (LABEL_FONT.spec(), SMALL_FONT.spec());
    let lerp = |a: f32, b: f32| egui::lerp(a..=b, k);
    let line = lerp(
        from.line_height.max(tf::MIN_TEXT_LINE_HEIGHT),
        to.line_height.max(tf::MIN_FOCUSED_LABEL_LINE_HEIGHT),
    );
    TextFormat {
        font_id: FontId::new(lerp(from.size, to.size), LABEL_FONT.font(ctx).family),
        extra_letter_spacing: lerp(from.tracking, to.tracking),
        line_height: Some(line),
        ..TextFormat::simple(LABEL_FONT.font(ctx), color)
    }
}

pub(crate) struct Decor<'a> {
    pub id: Id,
    pub variant: TextFieldVariant,
    pub label: &'a str,
    pub focused: bool,
    /// Compose `labelProgress` target: focused or not empty.
    pub floated: bool,
    pub error: bool,
    pub disabled: bool,
    pub leading: Option<Icon>,
    pub trailing: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct DecorOut {
    /// Content span on screen.
    pub span: egui::Rangef,
    /// Top of the first input line box.
    pub text_top: f32,
    /// Label progress (0 expanded, 1 minimized).
    #[cfg(feature = "extras")]
    pub k: f32,
    pub colors: FieldColors,
}

impl Decor<'_> {
    fn colors(&self, ui: &MineralUi<'_, Material>, field: Rect, t: &Tokens) -> FieldColors {
        let state = FieldState {
            focused: false,
            hovered: !self.disabled && ui.rect_contains_pointer(field),
            error: self.error,
            disabled: self.disabled,
        };
        let focus = egui_mineral::motion::spring(
            ui.core(),
            self.id.with("focus"),
            f32::from(self.focused),
            motion::FAST_EFFECTS,
        );
        let focused = FieldState { focused: true, ..state };
        colors(self.variant, state, t).lerp(colors(self.variant, focused, t), focus)
    }

    pub(crate) fn paint(&self, ui: &MineralUi<'_, Material>, field: Rect, t: &Tokens) -> DecorOut {
        let rt = ui.rt();
        let dir = rt.i18n().direction();
        let labeled = !self.label.is_empty();
        let k = egui_mineral::motion::spring(
            rt.core(),
            self.id.with("label"),
            f32::from(self.floated),
            motion::FAST_SPATIAL,
        );
        let colors = self.colors(ui, field, t);
        let (lead, trail) = (
            content_inset(rt, ui.density(), self.leading.is_some()),
            content_inset(rt, ui.density(), self.trailing),
        );
        let Sides { left, right } = dir.sides(lead, trail);
        let (start, end) = (field.left() + left, field.right() - right);
        let (rest_w, focus_w) = indicator_widths(self.variant);
        let width = egui_mineral::motion::spring(
            rt.core(),
            self.id.with("indicator"),
            if self.focused && !self.disabled {
                focus_w
            } else {
                rest_w
            },
            motion::FAST_SPATIAL,
        );

        let p = ui.painter();
        let outlined = self.variant == TextFieldVariant::Outlined;
        let shape = match self.variant {
            TextFieldVariant::Filled => filled_text_field::CONTAINER_SHAPE,
            TextFieldVariant::Outlined => outlined_text_field::CONTAINER_SHAPE,
        }
        .corner_radius(&t.shape, dir);
        p.rect_filled(field, shape, colors.container);

        let lay = LabelLayout {
            dir,
            k,
            lead,
            start,
            end,
        };
        let gap = if labeled {
            self.paint_label(ui.density(), p, field, &lay, &colors)
        } else {
            None
        };
        if outlined {
            let r = f32::from(shape.nw);
            outline_with_gap(p, field, r, gap, Stroke::new(width, colors.indicator));
        } else {
            let line = Rect::from_min_max(pos2(field.left(), field.bottom() - width), field.max);
            p.rect_filled(line, 0, colors.indicator);
        }
        if let Some(icon) = self.leading {
            let slot = icon_slot(rt, ui.density()) / 2.0;
            let x = dir.start_x(field, Rangef::point(slot));
            let size = match self.variant {
                TextFieldVariant::Filled => filled_text_field::LEADING_ICON_SIZE,
                TextFieldVariant::Outlined => outlined_text_field::LEADING_ICON_SIZE,
            };
            common::icon(p, pos2(x, field.center().y), icon, size, colors.leading);
        }
        let text_top = field.top() + text_top(ui.density(), self.variant, labeled);
        DecorOut {
            span: egui::Rangef::new(start, end),
            text_top,
            #[cfg(feature = "extras")]
            k,
            colors,
        }
    }

    fn paint_label(
        &self,
        density: Density,
        p: &Painter,
        field: Rect,
        lay: &LabelLayout,
        colors: &FieldColors,
    ) -> Option<(f32, f32)> {
        let LabelLayout {
            dir,
            k,
            lead,
            start,
            end,
        } = *lay;
        let ctx = p.ctx();
        let outlined = self.variant == TextFieldVariant::Outlined;
        // Single-line container height; content of taller fields stays at its top.
        let h = field_h(density, self.variant, true).min(field.height());
        // Density shrink; filled content moves up by half of it.
        let lift = (tf::MIN_HEIGHT - field_h(density, self.variant, true)) / 2.0;
        // Outlined labels minimize to the padding, over a leading icon too.
        let min_x = if outlined { tf::PADDING } else { lead };
        let label_x = egui::lerp(lead..=min_x, k);
        let mut gap = None;
        let min_span = if outlined {
            field.width() - 2.0 * tf::PADDING
        } else {
            end - start
        };
        let span = egui::lerp((end - start)..=min_span, k);
        let mut job = LayoutJob::single_section(self.label.to_owned(), label_format(ctx, k, colors.label));
        job.wrap = TextWrapping::truncate_at_width(span.max(0.0));
        let galley = p.layout_job(job);
        let lh = galley.size().y;
        let rest_y = field.top() + (h - lh) / 2.0;
        let min_y = if outlined {
            field.top() - lh / 2.0
        } else {
            field.top() + tf::LABEL_VERTICAL_PADDING - lift
        };
        let x = dir.start_x(field, Rangef::new(label_x, label_x + galley.size().x));
        let w = galley.size().x * k.max(0.0);
        if outlined && w > 0.0 {
            // `outlineCutout`: centered on the minimized label.
            let x0 = dir.start_x(field, Rangef::new(min_x, min_x + w));
            gap = Some((x0 - tf::CUTOUT_PADDING, x0 + w + tf::CUTOUT_PADDING));
        }
        p.galley(pos2(x, egui::lerp(rest_y..=min_y, k)), galley, colors.label);
        gap
    }
}

#[derive(Clone, Copy)]
struct LabelLayout {
    dir: Direction,
    k: f32,
    lead: f32,
    start: f32,
    end: f32,
}

// Compose IconButton
pub(crate) fn trailing_button(
    ui: &mut MineralUi<'_, Material>,
    field: Rect,
    id: Id,
    icon: Icon,
    color: Color32,
    disabled: bool,
) -> Response {
    let rt = ui.rt();
    let slot = icon_slot(ui.rt(), ui.density()) / 2.0;
    let x = rt.i18n().direction().reversed().start_x(field, Rangef::point(slot));
    slot_button(ui, pos2(x, field.center().y), id, icon, color, disabled)
}

// Compose IconButton
pub(crate) fn slot_button(
    ui: &mut MineralUi<'_, Material>,
    center: Pos2,
    id: Id,
    icon: Icon,
    color: Color32,
    disabled: bool,
) -> Response {
    let part = parts::IconPart::small(icon, color, Availability::disabled_if(disabled));
    let rect = Rect::from_center_size(center, part.layer);
    parts::icon_part(ui, rect, id, egui::Sense::click(), &part)
}

pub(crate) fn paint_support(
    ui: &MineralUi<'_, Material>,
    field: Rect,
    text: Option<&str>,
    counter: Option<String>,
    color: Color32,
    elide: Elide,
) {
    let rt = ui.rt();
    let dir = rt.i18n().direction();
    let y = field.bottom() + tf::SUPPORTING_TOP_PADDING;
    let p = ui.painter();
    let span = field.width() - 2.0 * tf::PADDING;
    let count = counter.map(|n| line_galley(p, &n, SMALL_FONT, color, span));
    let count_w = count.as_ref().map_or(0.0, |g| g.size().x + tf::PADDING);
    if let Some(s) = text {
        let g = elided_galley(p, s, SMALL_FONT, color, span - count_w, elide);
        let x = dir.start_x(field, Rangef::new(tf::PADDING, tf::PADDING + g.size().x));
        p.galley(pos2(x, y), g, color);
    }
    if let Some(g) = count {
        let x = dir
            .reversed()
            .start_x(field, Rangef::new(tf::PADDING, tf::PADDING + g.size().x));
        p.galley(pos2(x, y), g, color);
    }
}

pub(crate) fn emit_part(ctx: &egui::Context, r: &Response, depth: usize, node: impl FnOnce() -> SemanticNode) {
    semantic::emit(ctx, Some(r.id), r.rect, || {
        let mut n = node();
        n.depth = depth;
        n
    });
}

pub(crate) fn counter(rt: &Rt, count: usize, max: usize) -> String {
    let f = rt.i18n().format();
    rt.i18n().tr(
        "mineral-text-counter",
        &[("count", f.count(count).into()), ("max", f.count(max).into())],
    )
}

fn input_layouter(
    format: TextFormat,
    password: bool,
    multiline: bool,
) -> impl FnMut(&Ui, &dyn TextBuffer, f32) -> Arc<Galley> {
    move |ui, buf, wrap_width| {
        let text = if password {
            std::iter::repeat_n(
                egui::epaint::text::PASSWORD_REPLACEMENT_CHAR,
                buf.as_str().chars().count(),
            )
            .collect()
        } else {
            buf.as_str().to_owned()
        };
        let mut job = LayoutJob::single_section(text, format.clone());
        job.keep_trailing_whitespace = true;
        if multiline {
            job.wrap.max_width = wrap_width;
        } else {
            job.break_on_newline = false;
        }
        ui.fonts_mut(|f| f.layout_job(job))
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct FieldChrome {
    pub variant: TextFieldVariant,
    pub supporting: Option<String>,
    pub error: Option<String>,
    pub leading: Option<Icon>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub placeholder: Option<String>,
    pub max_chars: Option<usize>,
    pub text_style: Option<TypeScale>,
    pub label_hidden: bool,
}

/// `variant`, `supporting`, `error`, `maybe_error` and (unless `no_icon`) `icon` builders over a
/// `chrome: FieldChrome` field.
macro_rules! field_chrome_setters {
    () => {
        $crate::text_inputs::text_field::field_chrome_setters!(no_icon);

        /// Leading icon.
        pub fn icon(mut self, icon: impl Into<$crate::Icon>) -> Self {
            self.chrome.leading = Some(icon.into());
            self
        }
    };
    (text) => {
        $crate::text_inputs::text_field::field_chrome_setters!(text chrome);
    };
    (text $($path:ident).+) => {
        $crate::text_inputs::text_field::field_chrome_setters!(affix $($path).+);

        /// Hint shown while empty.
        pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
            self.$($path).+.placeholder = Some(placeholder.into());
            self
        }

        /// Character limit, with a counter below the field.
        pub fn max_chars(mut self, max_chars: usize) -> Self {
            self.$($path).+.max_chars = Some(max_chars);
            self
        }
    };
    (affix) => {
        $crate::text_inputs::text_field::field_chrome_setters!(affix chrome);
    };
    (affix $($path:ident).+) => {
        /// Type style of the value and affixes; default [`TypeScale::BodyLarge`]($crate::spec::TypeScale::BodyLarge).
        /// Lines past the first are spaced by its line height; the first keeps the 24dp line box.
        pub fn text_style(mut self, text_style: $crate::spec::TypeScale) -> Self {
            self.$($path).+.text_style = Some(text_style);
            self
        }

        /// Text before the input, e.g. a currency sign.
        pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
            self.$($path).+.prefix = Some(prefix.into());
            self
        }

        /// Text after the input, e.g. a unit.
        pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
            self.$($path).+.suffix = Some(suffix.into());
            self
        }

        /// `false` hides the label but keeps it as the accessible name; default `true`.
        pub fn label_visible(mut self, label_visible: bool) -> Self {
            self.$($path).+.label_hidden = !label_visible;
            self
        }
    };
    (variant) => {
        /// Default [`TextFieldVariant::Filled`]($crate::text_inputs::TextFieldVariant::Filled).
        pub fn variant(mut self, variant: $crate::text_inputs::TextFieldVariant) -> Self {
            self.chrome.variant = variant;
            self
        }
    };
    (no_icon) => {
        $crate::text_inputs::text_field::field_chrome_setters!(variant);

        /// Helper text below the field; replaced by an error.
        pub fn supporting(mut self, supporting: impl Into<String>) -> Self {
            self.chrome.supporting = Some(supporting.into());
            self
        }

        $crate::widget::error_setters! {
            /// Error colors; `message` replaces the supporting text.
            |s| s.chrome.error
        }
    };
}
pub(crate) use field_chrome_setters;

#[cfg(feature = "extras")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Combo {
    Collapsed,
    Expanded,
}

/// MD3 text field over `egui::TextEdit`.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct TextField<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    value: &'a mut String,
    trailing: Option<(Icon, String)>,
    password: bool,
    reveal_toggle: bool,
    revealed: bool,
    rows: NonZeroUsize,
    grow_rows: usize,
    disabled: bool,
    commit_on_submit: bool,
    event_filter: Option<EventFilter>,
    elide: Option<Elide>,
    pub(crate) spin: Option<NumericRange>,
    #[cfg(feature = "extras")]
    pub(crate) combo: Option<Combo>,
    pub(crate) active_descendant: Option<egui::Id>,
    chrome: FieldChrome,
}

/// Events of [`TextField`], read from `WidgetResponse::output`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct TextFieldOutput {
    /// Trailing icon clicked; see [`TextField::trailing_icon`].
    pub trailing_clicked: bool,
    /// Enter pressed in a single-line field.
    pub submitted: bool,
    /// Focus lost this frame.
    pub blurred: bool,
    /// End or flush of a buffered edit; always `None` without [`TextField::commit_on_submit`].
    pub edit: Option<EditEvent>,
}

pub(crate) fn field_id(ui: &Ui, id_salt: &KeyedSalt) -> Id {
    ui.make_persistent_id(id_salt.id_salt())
}

impl<'a> TextField<'a> {
    /// `label` is the floating label and accessible name; `value` is the bound text. Agent key `<id_salt>-field`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, value: &'a mut String) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        let label = label.into();
        Self {
            chrome: FieldChrome::default(),
            key: AgentKey::Kind("field"),
            salt: id_salt,
            label,
            value,
            trailing: None,
            password: false,
            reveal_toggle: true,
            revealed: false,
            rows: NonZeroUsize::MIN,
            grow_rows: 0,
            disabled: false,
            commit_on_submit: false,
            event_filter: None,
            elide: None,
            spin: None,
            #[cfg(feature = "extras")]
            combo: None,
            active_descendant: None,
        }
    }

    crate::text_inputs::text_field::field_chrome_setters!();
    crate::text_inputs::text_field::field_chrome_setters!(text);

    /// Overflowing text keeps this part visible: a single-line value while unfocused, and the
    /// supporting text. [`Elide::Start`] shows a path's file name. Default: the value scrolls
    /// to its start and the supporting text elides its end.
    pub fn elide(mut self, elide: Elide) -> Self {
        self.elide = Some(elide);
        self
    }

    /// Trailing icon button named `label`; reports [`TextFieldOutput::trailing_clicked`].
    pub fn trailing_icon(mut self, icon: impl Into<Icon>, label: impl Into<String>) -> Self {
        self.trailing = Some((icon.into(), label.into()));
        self
    }

    /// Masks the input.
    pub fn masked(mut self, masked: bool) -> Self {
        self.password = masked;
        self
    }

    /// Password fields only: eye button that unmasks the input; default `true`.
    pub fn reveal_toggle(mut self, reveal_toggle: bool) -> Self {
        self.reveal_toggle = reveal_toggle;
        self
    }

    fn max_lines(&self) -> usize {
        self.rows.get().saturating_add(self.grow_rows)
    }

    /// Visible lines; default 1, above 1 the field is multiline.
    pub fn rows(mut self, rows: NonZeroUsize) -> Self {
        self.rows = rows;
        self
    }

    /// Lines the field grows by past [`Self::rows`] before it scrolls; default 0.
    pub fn grow_rows(mut self, grow_rows: usize) -> Self {
        self.grow_rows = grow_rows;
        self
    }

    crate::widget::setters! {
        /// Greys out the field; the text is read-only.
        disabled
    }

    /// Edits a draft that reaches the bound value on Enter (single line), focus loss or
    /// [`edit::commit_pending`](egui_mineral::edit::commit_pending); Escape restores the value
    /// from before the edit. Off, typing writes the bound value live.
    pub fn commit_on_submit(mut self, commit_on_submit: bool) -> Self {
        self.commit_on_submit = commit_on_submit;
        self
    }

    /// Keys the editor keeps while focused; the rest move or drop focus.
    pub fn event_filter(mut self, filter: EventFilter) -> Self {
        self.event_filter = Some(filter);
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-field` agent key.
        agent_key(key)
    }

    #[cfg(feature = "extras")]
    pub(crate) fn with_agent_key(mut self, key: AgentKey) -> Self {
        self.key = key;
        self
    }

    // The input format hint when the chrome has no placeholder.
    pub(crate) fn placeholder_or(mut self, hint: String) -> Self {
        self.chrome.placeholder.get_or_insert(hint);
        self
    }

    pub(crate) fn with_chrome(mut self, chrome: FieldChrome) -> Self {
        self.chrome = chrome;
        self
    }

    fn input_style(&self) -> TypeScale {
        self.chrome.text_style.unwrap_or(INPUT_FONT)
    }

    fn labeled(&self) -> bool {
        !self.chrome.label_hidden && !self.label.is_empty()
    }

    fn multiline(&self) -> bool {
        self.max_lines() > 1
    }

    fn has_toggle(&self) -> bool {
        self.password && self.reveal_toggle && !self.multiline()
    }

    fn has_trailing(&self) -> bool {
        self.has_toggle() || self.trailing.is_some()
    }

    fn shown<'s>(&'s self, buffer: Option<&'s EditBuffer<String>>) -> &'s str {
        match buffer {
            Some(b) if self.commit_on_submit => b.draft(),
            _ => self.value,
        }
    }

    // Editor width in a `width` wide field: the span `paint_chrome` leaves beside the affixes.
    fn text_width(&self, ui: &MineralUi<'_, Material>, width: f32) -> f32 {
        let rt = ui.rt();
        let span = width
            - content_inset(rt, ui.density(), self.chrome.leading.is_some())
            - content_inset(rt, ui.density(), self.has_trailing());
        span - affix_room(ui, &self.chrome, span)
    }

    fn lines(&self, ui: &MineralUi<'_, Material>, width: f32, buffer: &EditBuffer<String>) -> usize {
        if self.grow_rows == 0 {
            return self.rows.get();
        }
        let mut job = LayoutJob::single_section(
            self.shown(Some(buffer)).to_owned(),
            self.input_style().format(ui.ctx(), Color32::PLACEHOLDER),
        );
        job.wrap.max_width = self.text_width(ui, width).max(1.0);
        let galley = ui.painter().layout_job(job);
        galley.rows.len().clamp(self.rows.get(), self.max_lines())
    }

    // Lines of a `height` tall field, the inverse of `field_height`.
    fn lines_in(&self, density: Density, height: f32) -> usize {
        if self.grow_rows == 0 {
            return self.rows.get();
        }
        let extra = ((height - field_h(density, self.chrome.variant, self.labeled()))
            / self.input_style().line_height())
        .round()
        .max(0.0);
        #[expect(clippy::cast_sign_loss, reason = "max(0.0)")]
        let extra = extra as usize;
        extra.saturating_add(1).clamp(self.rows.get(), self.max_lines())
    }

    fn field_height(&self, density: Density, lines: usize) -> f32 {
        field_h(density, self.chrome.variant, self.labeled()) + (lines - 1) as f32 * self.input_style().line_height()
    }

    fn has_support_row(&self) -> bool {
        self.chrome.supporting.is_some() || self.chrome.error.is_some() || self.chrome.max_chars.is_some()
    }
}

struct EditorCx<'a> {
    id: Id,
    rect: Rect,
    lines: usize,
    focused: bool,
    colors: &'a FieldColors,
    t: &'a Tokens,
}

// Frameless `TextEdit` in a child Ui filling `rect`, with field caret and selection colors.
pub(crate) struct BareEdit<'a> {
    id: Id,
    rect: Rect,
    format: TextFormat,
    colors: &'a FieldColors,
    t: &'a Tokens,
    v_align: egui::Align,
    mask: bool,
    rows: Option<usize>,
}

impl<'a> BareEdit<'a> {
    pub(crate) fn new(id: Id, rect: Rect, format: TextFormat, colors: &'a FieldColors, t: &'a Tokens) -> Self {
        Self {
            id,
            rect,
            format,
            colors,
            t,
            v_align: egui::Align::Min,
            mask: false,
            rows: None,
        }
    }

    #[cfg(feature = "extras")]
    pub(crate) fn v_align(mut self, v_align: egui::Align) -> Self {
        self.v_align = v_align;
        self
    }

    pub(crate) fn mask(mut self, mask: bool) -> Self {
        self.mask = mask;
        self
    }

    // `Some(rows)`: wrapping multiline edit.
    pub(crate) fn rows(mut self, rows: Option<usize>) -> Self {
        self.rows = rows;
        self
    }

    // `add` sets the remaining options and adds the edit to the child Ui.
    pub(crate) fn show(
        self,
        ui: &mut MineralUi<'_, Material>,
        text: &mut dyn TextBuffer,
        add: impl FnOnce(&mut Ui, TextEdit<'_>) -> Response,
    ) -> Response {
        let dir = ui.i18n().direction();
        let mut layouter = input_layouter(self.format, self.mask, self.rows.is_some());
        let edit = match self.rows {
            Some(rows) => TextEdit::multiline(text).desired_rows(rows),
            None => TextEdit::singleline(text),
        }
        .id(self.id)
        .frame(Frame::NONE)
        .margin(Margin::ZERO)
        .layouter(&mut layouter)
        // egui otherwise takes the parent layout's alignment.
        .align(egui::Align2([dir.start_align(), self.v_align]))
        .password(self.mask)
        .desired_width(self.rect.width());
        // A child Ui leaves the parent cursor alone (`put`/`scope_builder` would move it back).
        let mut child = ui.new_child(UiBuilder::new().id_salt(self.id).max_rect(self.rect));
        self.colors.apply_editor(&mut child, self.t);
        add(&mut child, edit)
    }
}

// Single-line submit: Enter pressed as the edit lost focus.
pub(crate) fn submitted(edit: &Response) -> bool {
    edit.lost_focus() && edit.ctx.input(|i| i.key_pressed(egui::Key::Enter))
}

impl TextField<'_> {
    fn sync_buffer(
        &self,
        ctx: &egui::Context,
        buffer: &mut EditBuffer<String>,
        edit_id: Id,
        focused: bool,
        out: &mut TextFieldOutput,
    ) {
        if !self.commit_on_submit {
            return;
        }
        let focus = FieldFocus::of(focused);
        buffer.sync(self.value, focus);
        if !self.disabled && egui_mineral::kit::overlay::consume_escape_from(ctx, edit_id) {
            out.edit = buffer.cancel();
            ctx.memory_mut(|m| m.surrender_focus(edit_id));
        }
    }

    fn show_editor(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        buffer: &mut EditBuffer<String>,
        cx: &EditorCx<'_>,
    ) -> Response {
        let buffered = self.commit_on_submit;
        let mask = self.password && !self.revealed;
        let rows = self.multiline().then_some(cx.lines);
        let (interactive, max_chars, event_filter) = (!self.disabled, self.chrome.max_chars, self.event_filter);
        let style = self.input_style();
        let text: &mut String = if buffered { buffer.draft_mut() } else { &mut *self.value };
        // The unfocused editor stays for clicks and Tab; an elided copy is painted over its
        // transparent text.
        let elided = self
            .elide
            .filter(|_| !cx.focused && rows.is_none() && !mask)
            .map(|e| elided_galley(ui.painter(), text, style, cx.colors.input, cx.rect.width(), e))
            .filter(|g| g.text() != text.as_str());
        let color = if elided.is_some() {
            Color32::TRANSPARENT
        } else {
            cx.colors.input
        };
        if let Some(g) = &elided {
            let x = ui
                .i18n()
                .direction()
                .start_x(cx.rect, Rangef::new(0.0, 0.0 + g.size().x));
            ui.painter()
                .galley(pos2(x, cx.rect.top()), Arc::clone(g), Color32::PLACEHOLDER);
        }
        BareEdit::new(cx.id, cx.rect, style.format(ui.ctx(), color), cx.colors, cx.t)
            .mask(mask)
            .rows(rows)
            .show(ui, text, |ui, mut edit| {
                edit = edit.interactive(interactive);
                if let Some(n) = max_chars {
                    edit = edit.char_limit(n);
                }
                if let Some(f) = event_filter {
                    edit = edit.event_filter(f);
                }
                if buffered {
                    // Escape cancels the edit instead of dropping focus.
                    let f = edit.get_event_filter();
                    edit = edit.event_filter(EventFilter { escape: true, ..f });
                }
                if rows.is_some() {
                    ScrollArea::vertical()
                        .id_salt("scroll")
                        .max_height(cx.rect.height())
                        .auto_shrink([false; 2])
                        .stick_to_bottom(cx.focused)
                        .show(ui, |ui| ui.add(edit))
                        .inner
                } else {
                    ui.add(edit)
                }
            })
    }

    fn show_trailing(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        field: Rect,
        base: Id,
        colors: &FieldColors,
        out: &mut TextFieldOutput,
    ) {
        let rt = ui.rt();
        let ctx = rt.ctx();
        if self.has_toggle() {
            let revealed = self.revealed;
            let icon = if revealed {
                ui.icons().visibility_off
            } else {
                ui.icons().visibility
            };
            let r = trailing_button(ui, field, base.with("trailing"), icon, colors.trailing, self.disabled);
            let revealed = if r.clicked() {
                let flipped = !revealed;
                flipped.store(ctx, base.with("reveal"));
                self.revealed = flipped;
                if ctx.current_pass_index() == 0 {
                    ctx.request_discard("password reveal toggled after the editor laid out");
                }
                flipped
            } else {
                revealed
            };
            emit_part(ctx, &r, 1, || {
                let msg = if revealed {
                    "mineral-password-hide"
                } else {
                    "mineral-password-show"
                };
                let mut n = SemanticNode::new(
                    self.key.id(&self.salt).sub("reveal"),
                    Role::Button,
                    crate::tr(rt.i18n(), msg),
                )
                .clickable(Availability::disabled_if(self.disabled));
                n.state.checked = Some(revealed.into());
                n
            });
        } else if let Some((icon, label)) = &self.trailing {
            let r = trailing_button(ui, field, base.with("trailing"), *icon, colors.trailing, self.disabled);
            out.trailing_clicked = r.clicked();
            emit_part(ctx, &r, 1, || {
                SemanticNode::new(self.key.id(&self.salt).sub("trailing"), Role::Button, label)
                    .clickable(Availability::disabled_if(self.disabled))
            });
        }
    }

    fn settle_edit(
        &mut self,
        ctx: &egui::Context,
        buffer: &mut EditBuffer<String>,
        te: &mut Response,
        edit_id: Id,
        out: &mut TextFieldOutput,
    ) -> bool {
        if !self.commit_on_submit {
            return te.changed();
        }
        if te.changed() {
            buffer.start(self.value);
        }
        let flush = buffer.commit_requested(ctx);
        if out.edit.is_none() && (flush || !ctx.memory(|m| m.has_focus(edit_id))) {
            out.edit = buffer.commit();
        }
        // The draft changed, not the bound value.
        te.flags.remove(egui::response::Flags::CHANGED);
        let commit = out.edit == Some(EditEvent::Committed) && buffer.draft() != self.value;
        if commit {
            self.value.clone_from(buffer.draft());
            buffer.wrote(self.value);
        }
        commit
    }
}

pub(crate) struct AffixCx<'a> {
    pub chrome: &'a FieldChrome,
    pub labeled: bool,
    pub base: Id,
    pub focused: bool,
    pub empty: bool,
    pub lines: usize,
}

impl DecorOut {
    // The content span narrowed to `rect`.
    #[cfg(feature = "extras")]
    pub(crate) fn within(self, rect: Rect) -> Self {
        Self {
            span: Rangef::new(rect.left(), rect.right()),
            ..self
        }
    }
}

// Width the prefix and suffix take of a `span` wide content area.
pub(crate) fn affix_room(ui: &MineralUi<'_, Material>, chrome: &FieldChrome, span: f32) -> f32 {
    let style = chrome.text_style.unwrap_or(INPUT_FONT);
    let room = |s: &Option<String>| {
        s.as_deref().map_or(0.0, |s| {
            line_galley(ui.painter(), s, style, Color32::PLACEHOLDER, span).size().x + tf::AFFIX_PADDING
        })
    };
    room(&chrome.prefix) + room(&chrome.suffix)
}

// Affixes and placeholder in the content span; the editor rect they leave.
pub(crate) fn paint_affixes(ui: &MineralUi<'_, Material>, decor: &DecorOut, cx: &AffixCx<'_>) -> Rect {
    let AffixCx {
        chrome,
        labeled,
        base,
        focused,
        empty,
        lines,
    } = *cx;
    let rt = ui.rt();
    let style = chrome.text_style.unwrap_or(INPUT_FONT);
    let colors = decor.colors;
    // Compose `placeholderOpacity` and `affixOpacity`: hidden under an expanded label.
    let shown_unfocused = !labeled;
    let placeholder_alpha = egui_mineral::motion::spring(
        rt.core(),
        base.with("placeholder"),
        f32::from(focused || shown_unfocused),
        if focused {
            motion::SLOW_EFFECTS
        } else {
            motion::FAST_EFFECTS
        },
    );
    let affix_alpha = egui_mineral::motion::spring(
        rt.core(),
        base.with("affix"),
        f32::from(focused || shown_unfocused || !empty),
        motion::FAST_EFFECTS,
    );
    let dir = rt.i18n().direction();
    let p = ui.painter().clone();
    let affix = |s: &Option<String>| {
        s.as_deref()
            .map(|s| line_galley(&p, s, style, colors.affix, decor.span.span()))
    };
    let (prefix, suffix) = (affix(&chrome.prefix), affix(&chrome.suffix));
    let room = |g: &Option<Arc<Galley>>| g.as_ref().map_or(0.0, |g| g.size().x + tf::AFFIX_PADDING);
    let (lead_room, trail_room) = (room(&prefix), room(&suffix));
    let Sides { left, right } = dir.sides(lead_room, trail_room);
    let (x0, x1) = (decor.span.min + left, decor.span.max - right);
    let text_rect = Rect::from_min_max(
        pos2(x0, decor.text_top),
        pos2(
            x1.max(x0),
            decor.text_top + line_h() + (lines - 1) as f32 * style.line_height(),
        ),
    );
    let fade = |c: Color32, a: f32| c.gamma_multiply(a.clamp(0.0, 1.0));
    if affix_alpha > 0.0 {
        if let Some(g) = prefix {
            let x = dir.span_x(decor.span, Rangef::new(0.0, 0.0 + g.size().x));
            p.galley(pos2(x, decor.text_top), g, fade(colors.affix, affix_alpha));
        }
        if let Some(g) = suffix {
            let x = dir.reversed().span_x(decor.span, Rangef::new(0.0, 0.0 + g.size().x));
            p.galley(pos2(x, decor.text_top), g, fade(colors.affix, affix_alpha));
        }
    }
    if let (Some(ph), true) = (&chrome.placeholder, empty && placeholder_alpha > 0.0) {
        let g = line_galley(&p, ph, style, colors.placeholder, text_rect.width());
        let x = dir.start_x(text_rect, Rangef::new(0.0, 0.0 + g.size().x));
        p.galley(pos2(x, text_rect.top()), g, fade(colors.placeholder, placeholder_alpha));
    }
    text_rect
}

impl MineralWidget for TextField<'_> {
    type Kit = Material;
    type State = EditBuffer<String>;
    type Output = TextFieldOutput;

    egui_mineral::kit::widget_meta!(salt, enabled, no_target, frame_sense);

    /// `MIN_WIDTH` wide unless constrained.
    fn layout(&self, ui: &MineralUi<'_, Material>, constraints: Constraints, buffer: &Self::State) -> Measure {
        let w = constraints.constrain(vec2(tf::MIN_WIDTH, 0.0)).x;
        let container = constraints.constrain(vec2(w, self.field_height(ui.density(), self.lines(ui, w, buffer))));
        measure(container, self.chrome.variant, self.labeled(), self.has_support_row())
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        frame: &WidgetFrame<'_>,
        buffer: &mut EditBuffer<String>,
    ) -> WidgetResponse<TextFieldOutput> {
        let rt = ui.rt();
        let t = ui.tokens();
        let ctx = ui.ctx().clone();
        let base = ui.make_persistent_id(self.salt.id_salt());
        let edit_id = base.with("edit");
        let field = frame.zones.visual;
        let lines = self.lines_in(ui.density(), field.height());
        let mut out = TextFieldOutput::default();
        let focused = ctx.memory(|m| m.has_focus(edit_id));
        self.sync_buffer(&ctx, buffer, edit_id, focused, &mut out);
        let empty = self.shown(Some(buffer)).is_empty();
        let decor = Decor {
            id: base,
            variant: self.chrome.variant,
            label: if self.labeled() { &self.label } else { "" },
            focused,
            floated: focused || !empty,
            error: self.chrome.error.is_some(),
            disabled: self.disabled,
            leading: self.chrome.leading,
            trailing: self.has_trailing(),
        }
        .paint(ui, field, &t);
        let colors = decor.colors;
        self.revealed = bool::load(&ctx, base.with("reveal"));

        let text_rect = paint_affixes(
            ui,
            &decor,
            &AffixCx {
                chrome: &self.chrome,
                labeled: self.labeled(),
                base,
                focused,
                empty,
                lines,
            },
        );
        let mut te = self.show_editor(
            ui,
            buffer,
            &EditorCx {
                id: edit_id,
                rect: text_rect,
                lines,
                focused,
                colors: &colors,
                t: &t,
            },
        );
        // After the editor: Tab order is registration order.
        self.show_trailing(ui, field, base, &colors, &mut out);

        if !focused && frame.response.clicked() {
            ctx.memory_mut(|m| m.request_focus(edit_id));
        }
        out.blurred = te.lost_focus();
        out.submitted = !self.multiline() && submitted(&te);
        let changed = self.settle_edit(&ctx, buffer, &mut te, edit_id, &mut out);

        if self.has_support_row() {
            let support = self.chrome.error.as_deref().or(self.chrome.supporting.as_deref());
            let count = self
                .chrome
                .max_chars
                .map(|n| counter(rt, self.shown(Some(buffer)).chars().count(), n));
            paint_support(
                ui,
                field,
                support,
                count,
                colors.supporting,
                self.elide.unwrap_or_default(),
            );
        }

        WidgetResponse::new(te, out).with_changed(changed)
    }

    fn semantics(
        &self,
        _: &I18n,
        _: &WidgetResponse<TextFieldOutput>,
        buffer: &EditBuffer<String>,
    ) -> Option<SemanticNode> {
        let revealed = self.revealed;
        let role = match self.spin {
            Some(_) => Role::SpinButton,
            #[cfg(feature = "extras")]
            None if self.combo.is_some() => Role::ComboBox,
            _ if self.password => Role::PasswordInput,
            _ if self.multiline() => Role::MultilineTextInput,
            _ => Role::TextInput,
        };
        let shown = self.shown(Some(buffer));
        let value = if self.password && !revealed {
            "*".repeat(shown.chars().count())
        } else {
            shown.to_owned()
        };
        let mut node = SemanticNode::new(self.key.id(&self.salt), role, &self.label)
            .value(value)
            .editable(Availability::disabled_if(self.disabled));
        node.state.invalid = self.chrome.error.is_some();
        node.state.range = self.spin;
        #[cfg(feature = "extras")]
        {
            node.state.expanded = self.combo.map(|c| c == Combo::Expanded);
        }
        node.active_descendant = self.active_descendant;
        if !self.disabled {
            node.actions.push(Action::ReplaceSelectedText);
            if self.spin.is_some() {
                node.actions.extend([Action::Increment, Action::Decrement]);
            }
            #[cfg(feature = "extras")]
            if self.combo.is_some() {
                node.actions.push(Action::Expand);
            }
        }
        Some(node)
    }
}

pub(crate) fn outline_with_gap(p: &Painter, rect: Rect, r: f32, gap: Option<(f32, f32)>, stroke: Stroke) {
    let Some((g0, g1)) = gap else {
        p.rect_stroke(rect, r, stroke, StrokeKind::Inside);
        return;
    };
    let rect = rect.shrink(stroke.width / 2.0);
    let arc = |c: Pos2, from: f32| -> Vec<Pos2> {
        (0..=6)
            .map(|s| {
                let a = from + std::f32::consts::FRAC_PI_2 * s as f32 / 6.0;
                c + vec2(a.cos(), a.sin()) * r
            })
            .collect()
    };
    let mut pts = vec![pos2(g1.min(rect.right() - r), rect.top())];
    pts.extend(arc(
        pos2(rect.right() - r, rect.top() + r),
        -std::f32::consts::FRAC_PI_2,
    ));
    pts.extend(arc(pos2(rect.right() - r, rect.bottom() - r), 0.0));
    pts.extend(arc(
        pos2(rect.left() + r, rect.bottom() - r),
        std::f32::consts::FRAC_PI_2,
    ));
    pts.extend(arc(pos2(rect.left() + r, rect.top() + r), std::f32::consts::PI));
    pts.push(pos2(g0.max(rect.left() + r), rect.top()));
    p.add(Shape::line(pts, stroke));
}

#[cfg(test)]
mod tests {
    use super::{Elide, elided_galley};
    use crate::spec::TypeScale;

    #[test]
    fn elision_keeps_the_chosen_part_within_the_width() {
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let p = ui.painter();
            let fit = |text, e| elided_galley(p, text, TypeScale::BodyLarge, egui::Color32::WHITE, 160.0, e);
            let path = "/home/user/projects/screenshots/final-render.png";
            let (start, middle) = (fit(path, Elide::Start), fit(path, Elide::Middle));
            assert!(start.size().x <= 160.0 && middle.size().x <= 160.0);
            assert!(
                start.text().starts_with('\u{2026}') && start.text().ends_with("render.png"),
                "{}",
                start.text()
            );
            assert!(
                middle.text().starts_with("/home") && middle.text().ends_with("r.png"),
                "{}",
                middle.text()
            );
            assert_eq!(fit("a.png", Elide::Start).text(), "a.png");
        });
        out.textures_delta.clear();
    }
}

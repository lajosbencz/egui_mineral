use std::sync::Arc;

use egui::{
    Align, Align2, Color32, CornerRadius, Frame, Galley, Id, Margin, Pos2, Rangef, Rect, Response, ScrollArea, Stroke,
    StrokeKind, TextBuffer, TextEdit, TextFormat, Ui, UiBuilder, text::LayoutJob, vec2,
};
use egui_mineral::{
    i18n::{Direction, Sides},
    widget::MineralUi,
};

use crate::spec::ColorToken::{
    ColorCompoundBrandStroke, ColorCompoundBrandStrokePressed, ColorNeutralBackground1, ColorNeutralBackground3,
    ColorNeutralForeground1, ColorNeutralForeground3, ColorNeutralForeground4, ColorNeutralForegroundDisabled,
    ColorNeutralStroke1, ColorNeutralStroke1Hover, ColorNeutralStroke1Pressed, ColorNeutralStrokeAccessible,
    ColorNeutralStrokeAccessibleHover, ColorNeutralStrokeAccessiblePressed, ColorNeutralStrokeDisabled,
    ColorPaletteRedBorder2, ColorTransparentStroke, ColorTransparentStrokeInteractive,
};
use crate::{
    Fluent, FluentExt, Icon, Size,
    spec::{
        defaults::input as inp,
        tokens::{geometry as g, motion, typography as ty},
    },
    theme::Tokens,
    widget::{paint, tween},
};

/// Input `appearance` of [`Input`](super::Input), [`Textarea`](super::Textarea), [`SearchBox`](super::SearchBox)
/// and [`SpinButton`](super::SpinButton).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InputAppearance {
    /// `outline`: stroked box on `colorNeutralBackground1`.
    #[default]
    Outline,
    /// `underline`: bottom stroke only.
    Underline,
    /// `filled-darker`: `colorNeutralBackground3` fill.
    FilledDarker,
    /// `filled-lighter`: `colorNeutralBackground1` fill.
    FilledLighter,
}

impl InputAppearance {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Outline, Self::Underline, Self::FilledDarker, Self::FilledLighter];

    const fn filled(self) -> bool {
        matches!(self, Self::FilledDarker | Self::FilledLighter)
    }
}

/// Events of the text fields, read from [`WidgetResponse::output`](egui_mineral::widget::WidgetResponse::output).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct FieldOutput {
    /// Enter pressed as a single-line field lost focus.
    pub submitted: bool,
    /// Focus left the field.
    pub blurred: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Chrome {
    pub appearance: InputAppearance,
    pub size: Size,
    pub invalid: bool,
    pub disabled: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Live {
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Metrics {
    pub height: f32,
    pub font: f32,
    pub line: f32,
    pub icon: f32,
    pub root: f32,
    pub input: f32,
    pub combined: f32,
    pub gap: f32,
}

impl Chrome {
    // react-input useInputStyles.styles.ts fieldHeights, horizontalPadding, useRootStyles, useContentStyles
    pub(crate) fn metrics(self) -> Metrics {
        match self.size {
            Size::Small => Metrics {
                height: inp::HEIGHT_SMALL,
                font: ty::FONT_SIZE_BASE_200,
                line: ty::LINE_HEIGHT_BASE_200,
                icon: inp::CONTENT_ICON_SMALL,
                root: g::SPACING_HORIZONTAL_S_NUDGE,
                input: g::SPACING_HORIZONTAL_XXS,
                combined: g::SPACING_HORIZONTAL_S,
                gap: g::SPACING_HORIZONTAL_XXS,
            },
            Size::Medium => Metrics {
                height: inp::HEIGHT_MEDIUM,
                font: ty::FONT_SIZE_BASE_300,
                line: ty::LINE_HEIGHT_BASE_300,
                icon: inp::CONTENT_ICON,
                root: g::SPACING_HORIZONTAL_M_NUDGE,
                input: g::SPACING_HORIZONTAL_XXS,
                combined: g::SPACING_HORIZONTAL_M,
                gap: g::SPACING_HORIZONTAL_XXS,
            },
            Size::Large => Metrics {
                height: inp::HEIGHT_LARGE,
                font: ty::FONT_SIZE_BASE_400,
                line: ty::LINE_HEIGHT_BASE_400,
                icon: inp::CONTENT_ICON_LARGE,
                root: g::SPACING_HORIZONTAL_M,
                input: g::SPACING_HORIZONTAL_S_NUDGE,
                combined: g::SPACING_HORIZONTAL_M + g::SPACING_HORIZONTAL_S_NUDGE,
                gap: g::SPACING_HORIZONTAL_S_NUDGE,
            },
        }
    }

    pub(crate) fn text_format(self, ctx: &egui::Context, t: &Tokens) -> TextFormat {
        let m = self.metrics();
        let color = if self.disabled {
            ColorNeutralForegroundDisabled
        } else {
            ColorNeutralForeground1
        };
        crate::typography::format(
            ctx,
            crate::typography::TypeSpec {
                size: m.font,
                line_height: m.line,
                weight: ty::FONT_WEIGHT_REGULAR,
            },
            t.color(color),
        )
    }

    pub(crate) fn placeholder_color(self, t: &Tokens) -> Color32 {
        t.color(if self.disabled {
            ColorNeutralForegroundDisabled
        } else {
            ColorNeutralForeground4
        })
    }

    pub(crate) fn icon_color(self, t: &Tokens) -> Color32 {
        t.color(if self.disabled {
            ColorNeutralForegroundDisabled
        } else {
            ColorNeutralForeground3
        })
    }
}

// react-input useInputStyles.styles.ts root, outline/underline/filled and their `Interactive` states, invalid,
// disabled; the focus underline is `::after`.
pub(crate) fn paint_frame(ui: &MineralUi<'_, Fluent>, id: Id, rect: Rect, c: Chrome, live: Live, t: &Tokens) {
    let p = ui.painter();
    let engaged = live.pressed || live.focused;
    let pick = |rest, hover, active| {
        t.color(if engaged {
            active
        } else if live.hovered {
            hover
        } else {
            rest
        })
    };
    let radius = match c.appearance {
        InputAppearance::Underline => CornerRadius::ZERO,
        _ => CornerRadius::from(g::BORDER_RADIUS_MEDIUM),
    };
    let fill = match c.appearance {
        _ if c.disabled => Color32::TRANSPARENT,
        InputAppearance::Outline | InputAppearance::FilledLighter => t.color(ColorNeutralBackground1),
        InputAppearance::FilledDarker => t.color(ColorNeutralBackground3),
        InputAppearance::Underline => Color32::TRANSPARENT,
    };
    let bottom = if c.disabled {
        t.color(ColorNeutralStrokeDisabled)
    } else {
        pick(
            ColorNeutralStrokeAccessible,
            ColorNeutralStrokeAccessibleHover,
            ColorNeutralStrokeAccessiblePressed,
        )
    };
    let side = if c.disabled {
        t.color(ColorNeutralStrokeDisabled)
    } else if c.appearance.filled() {
        t.color(if live.hovered || live.focused {
            ColorTransparentStrokeInteractive
        } else {
            ColorTransparentStroke
        })
    } else {
        pick(
            ColorNeutralStroke1,
            ColorNeutralStroke1Hover,
            ColorNeutralStroke1Pressed,
        )
    };
    let (side, bottom) = if c.invalid && !c.disabled && !live.focused {
        let red = t.color(ColorPaletteRedBorder2);
        (red, red)
    } else {
        (side, bottom)
    };
    p.rect_filled(rect, radius, fill);
    let stroke = |color| Stroke::new(g::STROKE_WIDTH_THIN, color);
    if c.appearance == InputAppearance::Underline {
        p.hline(
            rect.x_range(),
            rect.bottom() - g::STROKE_WIDTH_THIN / 2.0,
            stroke(bottom),
        );
    } else {
        p.rect_stroke(rect, radius, stroke(side), StrokeKind::Inside);
        p.with_clip_rect(Rect::from_min_max(
            Pos2::new(rect.left(), rect.bottom() - g::STROKE_WIDTH_THIN),
            rect.max,
        ))
        .rect_stroke(rect, radius, stroke(bottom), StrokeKind::Inside);
    }
    if c.disabled {
        return;
    }
    let (d, curve) = if live.focused {
        (motion::DURATION_NORMAL, motion::CURVE_DECELERATE_MID)
    } else {
        (motion::DURATION_ULTRA_FAST, motion::CURVE_ACCELERATE_MID)
    };
    let k = tween::tween(ui.core(), id.with("underline"), f32::from(live.focused), d, curve);
    if k > 0.0 {
        let color = t.color(if live.pressed {
            ColorCompoundBrandStrokePressed
        } else {
            ColorCompoundBrandStroke
        });
        let w = rect.width() * k;
        let x = rect.center().x - w / 2.0;
        let band = Rect::from_min_size(
            Pos2::new(x, rect.bottom() - inp::FOCUS_UNDERLINE),
            vec2(w, inp::FOCUS_UNDERLINE),
        );
        p.rect_filled(band, 0.0, color);
    }
}

pub(crate) struct Pads {
    pub start: f32,
    pub end: f32,
}

// Horizontal room of the text between the contents: `padding` of the root, the input and the gap.
// Which content slots around the text are filled.
#[derive(Clone, Copy)]
pub(crate) struct Slots {
    pub before: bool,
    pub after: bool,
}

pub(crate) fn pads(m: &Metrics, slots: Slots) -> Pads {
    let side = |content: bool| {
        if content {
            m.root + m.icon + m.gap + m.input
        } else {
            m.combined
        }
    };
    Pads {
        start: side(slots.before),
        end: side(slots.after),
    }
}

pub(crate) struct Rects {
    pub text: Rect,
    pub before: Option<Rect>,
    pub after: Option<Rect>,
}

pub(crate) fn rects(frame: Rect, dir: Direction, m: &Metrics, pads: &Pads, slots: Slots) -> Rects {
    let inner = frame.shrink(g::STROKE_WIDTH_THIN);
    let slot = |at: f32, trailing: bool| {
        let d = if trailing { dir.reversed() } else { dir };
        let x = d.start_x(inner, Rangef::new(at, at + m.icon));
        Rect::from_center_size(Pos2::new(x + m.icon / 2.0, inner.center().y), vec2(m.icon, m.icon))
    };
    let Sides { left, right } = dir.sides(pads.start, pads.end);
    Rects {
        text: Rect::from_min_max(
            Pos2::new(inner.left() + left, inner.top()),
            Pos2::new((inner.right() - right).max(inner.left() + left), inner.bottom()),
        ),
        before: slots.before.then(|| slot(m.root, false)),
        after: slots.after.then(|| slot(m.root, true)),
    }
}

// Frameless `TextEdit` in a child Ui filling `rect`.
pub(crate) struct Edit<'a> {
    pub id: Id,
    pub rect: Rect,
    pub format: TextFormat,
    pub t: &'a Tokens,
    pub mask: bool,
    pub rows: Option<usize>,
}

impl Edit<'_> {
    pub(crate) fn show(
        self,
        ui: &mut MineralUi<'_, Fluent>,
        text: &mut dyn TextBuffer,
        add: impl FnOnce(TextEdit<'_>) -> TextEdit<'_>,
    ) -> Response {
        let dir = ui.i18n().direction();
        let multiline = self.rows.is_some();
        let (format, mask) = (self.format.clone(), self.mask);
        let mut layouter = move |ui: &Ui, buf: &dyn TextBuffer, wrap: f32| -> Arc<Galley> {
            let shown = if mask {
                std::iter::repeat_n(
                    egui::epaint::text::PASSWORD_REPLACEMENT_CHAR,
                    buf.as_str().chars().count(),
                )
                .collect()
            } else {
                buf.as_str().to_owned()
            };
            let mut job = LayoutJob::single_section(shown, format.clone());
            job.keep_trailing_whitespace = true;
            if multiline {
                job.wrap.max_width = wrap;
            } else {
                job.break_on_newline = false;
            }
            ui.fonts_mut(|f| f.layout_job(job))
        };
        let v_align = if multiline { Align::Min } else { Align::Center };
        let base = match self.rows {
            Some(rows) => TextEdit::multiline(text).desired_rows(rows),
            None => TextEdit::singleline(text),
        }
        .id(self.id)
        .frame(Frame::NONE)
        .margin(Margin::ZERO)
        .layouter(&mut layouter)
        .align(Align2([dir.start_align(), v_align]))
        .password(self.mask)
        .desired_width(self.rect.width());
        let edit = add(base);
        let mut child = ui.new_child(UiBuilder::new().id_salt(self.id).max_rect(self.rect));
        let v = child.visuals_mut();
        v.text_cursor.stroke.color = self.format.color;
        v.selection.bg_fill = self.t.color(ColorCompoundBrandStroke).gamma_multiply(0.4);
        if multiline {
            let out = ScrollArea::vertical()
                .id_salt("scroll")
                .max_height(self.rect.height())
                .auto_shrink([false; 2])
                .show(&mut child, |ui| ui.add(edit));
            egui_mineral::kit::scroll::chain(&child, &out);
            out.inner
        } else {
            child.add(edit)
        }
    }
}

pub(crate) fn submitted(edit: &Response) -> bool {
    edit.lost_focus() && edit.ctx.input(|i| i.key_pressed(egui::Key::Enter))
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Content {
    pub before: Option<Icon>,
    pub after: Option<Icon>,
}

pub(crate) struct Shown {
    pub response: Response,
}

pub(crate) struct Fill<'a> {
    pub base: Id,
    pub frame: Rect,
    pub chrome: Chrome,
    pub content: Content,
    pub placeholder: Option<&'a str>,
    pub mask: bool,
    pub rows: Option<usize>,
    pub reserve_end: f32,
    pub text_rect: Option<Rect>,
    pub metrics: Option<Metrics>,
}

// Frame, contents, placeholder and editor of a text field.
pub(crate) fn show(ui: &mut MineralUi<'_, Fluent>, text: &mut String, frame: &Response, f: &Fill<'_>) -> Shown {
    let t = ui.tokens();
    let ctx = ui.ctx().clone();
    let dir = ui.i18n().direction();
    let edit_id = f.base.with("edit");
    let focused = ctx.memory(|m| m.has_focus(edit_id));
    let m = f.metrics.unwrap_or_else(|| f.chrome.metrics());
    let slots = Slots {
        before: f.content.before.is_some(),
        after: f.content.after.is_some(),
    };
    let mut pads = pads(&m, slots);
    pads.end += f.reserve_end;
    let rects = rects(f.frame, dir, &m, &pads, slots);
    let live = Live {
        hovered: frame.contains_pointer(),
        pressed: frame.contains_pointer() && ctx.input(|i| i.pointer.primary_down()),
        focused,
    };
    paint_frame(ui, f.base, f.frame, f.chrome, live, &t);
    let p = ui.painter().clone();
    for (rect, icon) in [(rects.before, f.content.before), (rects.after, f.content.after)] {
        if let (Some(rect), Some(icon)) = (rect, icon) {
            paint::icon(&p, rect.center(), icon, false, m.icon, f.chrome.icon_color(&t));
        }
    }
    let format = f.chrome.text_format(&ctx, &t);
    let text_rect = f.text_rect.unwrap_or(rects.text);
    if let (Some(ph), true) = (f.placeholder, text.is_empty()) {
        let color = f.chrome.placeholder_color(&t);
        let g = p.layout_no_wrap(ph.to_owned(), format.font_id.clone(), color);
        let x = dir.start_x(text_rect, Rangef::new(0.0, g.size().x));
        let y = if f.rows.is_some() {
            text_rect.top()
        } else {
            text_rect.center().y - g.size().y / 2.0
        };
        p.galley(Pos2::new(x, y), g, color);
    }
    let response = Edit {
        id: edit_id,
        rect: text_rect,
        format,
        t: &t,
        mask: f.mask,
        rows: f.rows,
    }
    .show(ui, text, |e| e.interactive(!f.chrome.disabled));
    if !focused && frame.clicked() && !f.chrome.disabled {
        ctx.memory_mut(|m| m.request_focus(edit_id));
    }
    Shown { response }
}

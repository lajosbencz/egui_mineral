use egui::{Rangef, Rect, ScrollArea, Sense, Stroke, StrokeKind, UiBuilder, pos2, vec2};
use egui_mineral::{
    kit,
    layout::{Constraints, Measure, MinTarget},
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{Availability, Interaction, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::ColorToken::{
    ColorNeutralBackground1, ColorNeutralBackground1Hover, ColorNeutralBackground1Pressed,
    ColorNeutralBackground1Selected, ColorNeutralBackground2, ColorNeutralForeground1, ColorNeutralForeground1Hover,
    ColorNeutralForeground1Pressed, ColorNeutralForeground1Selected, ColorNeutralStroke1, ColorNeutralStroke1Hover,
    ColorNeutralStroke1Pressed, ColorNeutralStroke1Selected, ColorStrokeFocus2,
};
use crate::{
    Fluent, FluentExt, Rt,
    navigation::overflow,
    spec::{
        defaults::avatar as d,
        tokens::{geometry, typography as ty},
    },
    surfaces::{Popover, PopoverSize},
    theme::Tokens,
    typography::{self, TypeRamp},
    widget::paint,
};

use super::{Avatar, AvatarShape, AvatarSize};
use super::{Text, avatar::Place};

/// `AvatarGroup` `layout`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AvatarGroupLayout {
    /// `spread`: spaced apart.
    #[default]
    Spread,
    /// `stack`: overlapping, outlined.
    Stack,
    /// `pie`: up to three slices of one circle.
    Pie,
}

impl AvatarGroupLayout {
    /// Every variant.
    pub const ALL: [Self; 3] = [Self::Spread, Self::Stack, Self::Pie];
}

// utils/partitionAvatarGroupItems.ts: inline and overflow items; `pie` keeps every item in the overflow
fn partition<T: Clone>(items: Vec<T>, layout: AvatarGroupLayout, max_inline: usize) -> (Vec<T>, Vec<T>) {
    if layout == AvatarGroupLayout::Pie {
        return (items.iter().take(d::PIE_SLICES).cloned().collect(), items);
    }
    let inline = max_inline - usize::from(items.len() > max_inline);
    let split = items.len().saturating_sub(inline);
    let mut overflow = items;
    let inline = overflow.split_off(split);
    (inline, overflow)
}

// usePieStyles `slices`: box, visible part and scale of each slice, relative to the group's top left
fn slices(count: usize, side: f32, divider: f32, rtl: bool) -> Vec<(Rect, Rect, f32)> {
    let (q, h) = (side / 4.0, side / 2.0);
    let full = Rect::from_min_size(pos2(0.0, 0.0), vec2(side, side));
    let rect = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(pos2(x0, y0), pos2(x1, y1));
    let mut out = match count {
        0 => Vec::new(),
        1 => vec![(full, full, 1.0)],
        2 => vec![
            (
                rect(-q, 0.0, side - q, side),
                rect(0.0, 0.0, h - divider / 2.0, side),
                1.0,
            ),
            (
                rect(q, 0.0, side + q, side),
                rect(h + divider / 2.0, 0.0, side, side),
                1.0,
            ),
        ],
        _ => vec![
            (
                rect(-q, 0.0, side - q, side),
                rect(0.0, 0.0, h - divider / 2.0, side),
                1.0,
            ),
            (
                rect(h, 0.0, side, h),
                rect(h + divider / 2.0, 0.0, side, h - divider / 2.0),
                0.5,
            ),
            (
                rect(h, h, side, side),
                rect(h + divider / 2.0, h + divider / 2.0, side, side),
                0.5,
            ),
        ],
    };
    if rtl && count == 3 {
        for (b, c, _) in &mut out {
            let flip = |r: Rect| rect(side - r.max.x, r.min.y, side - r.min.x, r.max.y);
            (*b, *c) = (flip(*b), flip(*c));
        }
    }
    out
}

// Convex polygon of the circle clipped to a rect (Sutherland-Hodgman)
fn circle_in(rect: Rect, circle: Rect) -> Vec<egui::Pos2> {
    const SEGMENTS: usize = 64;
    let (c, r) = (circle.center(), circle.width() / 2.0);
    let mut poly: Vec<egui::Pos2> = (0..SEGMENTS)
        .map(|i| {
            let a = std::f32::consts::TAU * i as f32 / SEGMENTS as f32;
            c + vec2(a.cos(), a.sin()) * r
        })
        .collect();
    let edges: [fn(egui::Pos2, Rect) -> f32; 4] = [
        |p, r| p.x - r.min.x,
        |p, r| r.max.x - p.x,
        |p, r| p.y - r.min.y,
        |p, r| r.max.y - p.y,
    ];
    for dist in edges {
        let input = std::mem::take(&mut poly);
        for (i, &a) in input.iter().enumerate() {
            let b = input.get((i + 1) % input.len()).copied().unwrap_or(a);
            let (da, db) = (dist(a, rect), dist(b, rect));
            if da >= 0.0 {
                poly.push(a);
            }
            if (da >= 0.0) != (db >= 0.0) {
                poly.push(a + (b - a) * (da / (da - db)));
            }
        }
    }
    poly
}

enum Indicator {
    Count(String),
    Icon,
}

struct Trigger {
    salt: KeyedSalt,
    label: String,
    size: AvatarSize,
    pie: bool,
    indicator: Indicator,
    open: bool,
}

impl Trigger {
    fn side(&self) -> f32 {
        f32::from(self.size.px())
    }
}

impl MineralWidget for Trigger {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt);

    fn sense(&self) -> Sense {
        Sense::click()
    }

    fn min_target(&self) -> MinTarget {
        MinTarget::None
    }

    fn layout(&self, _: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        c.constrain(vec2(self.side(), self.side())).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let r = &frame.response;
        kit::input::hover_cursor(ui.core(), r, Availability::Enabled);
        kit::focus::follow_press(r);
        let i = Interaction::of(r, Availability::Enabled);
        let rect = frame.zones.visual;
        let size = self.size.px();
        let radius = egui::CornerRadius::from(rect.width() / 2.0);
        let look = if self.pie {
            None
        } else if self.open {
            Some((
                ColorNeutralBackground1Selected,
                ColorNeutralForeground1Selected,
                ColorNeutralStroke1Selected,
            ))
        } else if i.pressed {
            Some((
                ColorNeutralBackground1Pressed,
                ColorNeutralForeground1Pressed,
                ColorNeutralStroke1Pressed,
            ))
        } else if i.hovered {
            Some((
                ColorNeutralBackground1Hover,
                ColorNeutralForeground1Hover,
                ColorNeutralStroke1Hover,
            ))
        } else {
            Some((ColorNeutralBackground1, ColorNeutralForeground1, ColorNeutralStroke1))
        };
        let p = ui.painter();
        if let Some((bg, fg, stroke)) = look {
            p.rect_filled(rect, radius, t.color(bg));
            p.rect_stroke(
                rect,
                radius,
                Stroke::new(d::trigger_border(size), t.color(stroke)),
                StrokeKind::Inside,
            );
            match &self.indicator {
                Indicator::Count(text) => {
                    let color = t.color(fg);
                    let font = typography::font(ui.ctx(), d::text_size(size), ty::FONT_WEIGHT_SEMIBOLD);
                    let g = p.layout_no_wrap(text.clone(), font, color);
                    p.galley(rect.center() - g.size() / 2.0, g, color);
                }
                Indicator::Icon => {
                    paint::icon(
                        p,
                        rect.center(),
                        ui.icons().more_horizontal,
                        false,
                        d::icon_size(size),
                        t.color(fg),
                    );
                }
            }
        }
        if i.focused {
            p.rect_stroke(
                rect,
                radius,
                Stroke::new(geometry::STROKE_WIDTH_THICK, t.color(ColorStrokeFocus2)),
                StrokeKind::Inside,
            );
        }
        WidgetResponse::new(r.clone(), ())
    }

    fn semantics(&self, _: &egui_mineral::i18n::I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(AgentKey::Kind("overflow").id(&self.salt), Role::Button, &self.label)
            .clickable(Availability::Enabled);
        n.state.expanded = Some(self.open);
        Some(n)
    }
}

/// Fluent `AvatarGroup`: avatars spread, stacked or as a pie, with the rest in an overflow popover.
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct AvatarGroup<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    layout: AvatarGroupLayout,
    size: AvatarSize,
    max_inline: usize,
    items: Vec<Avatar<'a>>,
}

impl<'a> AvatarGroup<'a> {
    /// `label` names the group for assistive technology; spread, 32, five inline items.
    /// Agent key `<id_salt>-avatar-group`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("avatar-group"),
            label: label.into(),
            layout: AvatarGroupLayout::Spread,
            size: AvatarSize::S32,
            max_inline: d::MAX_INLINE_ITEMS,
            items: Vec::new(),
        }
    }

    /// Default [`AvatarGroupLayout::Spread`].
    pub fn layout(mut self, layout: AvatarGroupLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Size of every avatar; default [`AvatarSize::S32`].
    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    /// `maxInlineItems`: avatars shown before the overflow button, which takes one of the places; default 5.
    pub fn max_inline_items(mut self, max: usize) -> Self {
        self.max_inline = max.max(1);
        self
    }

    /// `AvatarGroupItem`: an avatar of the group; its name labels it in the overflow popover.
    pub fn item(mut self, avatar: Avatar<'a>) -> Self {
        self.items.push(avatar);
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-avatar-group` agent key.
        agent_key(key)
    }

    /// Shows the group and its overflow popover.
    pub fn show(self, ui: &mut MineralUi<'_, Fluent>) -> egui::Response {
        let rt = ui.rt();
        let Self {
            salt,
            key,
            label,
            layout,
            size,
            max_inline,
            items,
        } = self;
        let dir = rt.i18n().direction();
        let rtl = dir.sign() < 0.0;
        let px = size.px();
        let side = f32::from(px);
        let (inline, overflow_items) = partition(items, layout, max_inline);
        let has_overflow = !overflow_items.is_empty();
        let pie = layout == AvatarGroupLayout::Pie;
        let slots = inline.len() + usize::from(has_overflow && !pie);
        let step = match layout {
            AvatarGroupLayout::Spread => side + d::spread_gap(px),
            AvatarGroupLayout::Stack => side - d::stack_overlap(px),
            AvatarGroupLayout::Pie => 0.0,
        };
        let width = if pie || slots == 0 {
            side
        } else {
            step * (slots - 1) as f32 + side
        };
        let (rect, response) = ui.allocate_exact_size(vec2(width, side), Sense::hover());
        let tokens = ui.tokens();
        let outline = d::group_outline(px);
        let divider = outline;
        let pie_slices = slices(inline.len(), side, divider, rtl);
        for (i, mut avatar) in inline.into_iter().enumerate() {
            avatar.size = size;
            if pie {
                let Some((boxed, clip, scale)) = pie_slices.get(i) else {
                    continue;
                };
                let off = rect.min.to_vec2();
                let (boxed, clip) = (boxed.translate(off), clip.translate(off));
                avatar.shape = AvatarShape::Square;
                avatar.place = Some(Place {
                    backdrop: circle_in(clip, rect),
                    clip,
                    scale: *scale,
                });
                place(ui, boxed, |ui| {
                    rt.ui(ui).mineral(avatar);
                });
            } else {
                let at = step * i as f32;
                let x = dir.start_x(rect, Rangef::new(at, at + side));
                let boxed = Rect::from_min_size(pos2(x, rect.min.y), vec2(side, side));
                if layout == AvatarGroupLayout::Stack {
                    stack_outline(ui, boxed, &avatar, outline, &tokens);
                }
                place(ui, boxed, |ui| {
                    rt.ui(ui).mineral(avatar);
                });
            }
        }
        if has_overflow {
            let at = if pie { 0.0 } else { step * (slots - 1) as f32 };
            let x = dir.start_x(rect, Rangef::new(at, at + side));
            let boxed = Rect::from_min_size(pos2(x, rect.min.y), vec2(side, side));
            let spec = OverflowSpec { salt: &salt, size, pie };
            show_overflow(ui, rt, &spec, boxed, overflow_items);
        }
        semantic::emit(ui.ctx(), Some(response.id), rect, || {
            SemanticNode::new(key.id(&salt), Role::Group, &label)
        });
        response
    }
}

struct OverflowSpec<'s> {
    salt: &'s KeyedSalt,
    size: AvatarSize,
    pie: bool,
}

// The overflow button in `boxed` and its popover listing `items`.
fn show_overflow(
    ui: &mut MineralUi<'_, Fluent>,
    rt: &Rt,
    spec: &OverflowSpec<'_>,
    boxed: Rect,
    items: Vec<Avatar<'_>>,
) {
    let OverflowSpec { salt, size, pie } = *spec;
    let open_id = ui.make_persistent_id(salt.id_salt()).with("mineral::overflow-open");
    let mut open = overflow::is_open(ui.ctx(), open_id);
    let count = i64::try_from(items.len()).unwrap_or(i64::MAX);
    let format = rt.i18n().format();
    let text = if count > d::COUNT_CAP {
        format!("{}+", format.integer(d::COUNT_CAP))
    } else {
        format!("+{}", format.integer(count))
    };
    let trigger = Trigger {
        salt: salt.clone(),
        label: crate::tr(rt.i18n(), "mineral-avatar-group-more"),
        size,
        pie,
        indicator: if size.px() < d::COUNT_INDICATOR_MIN_SIZE {
            Indicator::Icon
        } else {
            Indicator::Count(text)
        },
        open,
    };
    let trigger = place(ui, boxed, |ui| rt.ui(ui).mineral(trigger));
    let label = crate::tr(rt.i18n(), "mineral-avatar-group-overflow");
    Popover::new(salt.clone(), label)
        .size(PopoverSize::Small)
        .trap_focus(true)
        .show(rt, trigger.response(), &mut open, |ui| {
            ui.set_width(d::POPOVER_WIDTH - 2.0 * crate::spec::defaults::popover::PADDING_SMALL);
            ScrollArea::vertical()
                .max_height(d::POPOVER_MAX_HEIGHT)
                .min_scrolled_height(d::POPOVER_MIN_HEIGHT)
                .show(ui, |ui| {
                    for (i, mut avatar) in items.into_iter().enumerate() {
                        avatar.size = size;
                        avatar.place = None;
                        let name = avatar.name().to_owned();
                        avatar.salt = salt.child(("overflow-item", i));
                        let row_salt = salt.child(("overflow-label", i));
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = geometry::SPACING_HORIZONTAL_S;
                            let mut ui = rt.ui(ui);
                            ui.mineral(avatar);
                            ui.mineral(Text::new(row_salt, name).ramp(TypeRamp::Body1));
                        });
                    }
                });
        });
    overflow::set_open(ui.ctx(), open_id, open);
}

fn place<R>(ui: &mut egui::Ui, rect: Rect, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    // Not allocated in the parent: pie slice boxes reach past the group.
    add(&mut ui.new_child(UiBuilder::new().max_rect(rect)))
}

// useStackStyles box-shadow ring of the item's background
fn stack_outline(ui: &egui::Ui, rect: Rect, avatar: &Avatar<'_>, width: f32, t: &Tokens) {
    let color = t.color(ColorNeutralBackground2);
    let outer = rect.expand(width);
    let radius = match avatar.shape {
        AvatarShape::Circular => egui::CornerRadius::from(outer.width() / 2.0),
        AvatarShape::Square => egui::CornerRadius::from(d::square_radius(avatar.size.px()) + width),
    };
    ui.painter().rect_filled(outer, radius, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(n: usize, layout: AvatarGroupLayout, max: usize) -> (Vec<usize>, Vec<usize>) {
        partition((0..n).collect(), layout, max)
    }

    #[test]
    fn partition_keeps_the_last_items_inline() {
        assert_eq!(part(3, AvatarGroupLayout::Spread, 5), (vec![0, 1, 2], vec![]));
        assert_eq!(part(5, AvatarGroupLayout::Spread, 5), (vec![0, 1, 2, 3, 4], vec![]));
        assert_eq!(part(7, AvatarGroupLayout::Stack, 5), (vec![3, 4, 5, 6], vec![0, 1, 2]));
        assert_eq!(part(7, AvatarGroupLayout::Pie, 5), (vec![0, 1, 2], (0..7).collect()));
        assert_eq!(part(0, AvatarGroupLayout::Pie, 5), (vec![], vec![]));
    }

    #[test]
    fn pie_slices_split_the_circle() {
        let s = slices(3, 32.0, 2.0, false);
        assert_eq!(s.len(), 3);
        assert_eq!(s[0].1, Rect::from_min_max(pos2(0.0, 0.0), pos2(15.0, 32.0)));
        assert_eq!(s[1].1, Rect::from_min_max(pos2(17.0, 0.0), pos2(32.0, 15.0)));
        assert_eq!(s[2].1, Rect::from_min_max(pos2(17.0, 17.0), pos2(32.0, 32.0)));
        assert_eq!(s[1].2, 0.5);
        let rtl = slices(3, 32.0, 2.0, true);
        assert_eq!(rtl[0].1.min.x, 17.0);
    }

    #[test]
    fn clipped_circle_stays_inside_the_clip() {
        let circle = Rect::from_min_size(pos2(0.0, 0.0), vec2(32.0, 32.0));
        let clip = Rect::from_min_max(pos2(17.0, 0.0), pos2(32.0, 15.0));
        let poly = circle_in(clip, circle);
        assert!(poly.len() > 3);
        assert!(poly.iter().all(|p| clip.expand(1e-3).contains(*p)));
    }
}

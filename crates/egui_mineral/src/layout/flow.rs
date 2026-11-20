use std::panic::Location;

use egui::{Align, Id, IdSalt, Layout, Rangef, Rect, Ui, UiBuilder, Vec2, Vec2b, pos2, vec2};
use egui_taffy::{Tui, TuiBuilderLogic, TuiContainerResponse, taffy};

use crate::{
    i18n::Direction,
    runtime::{Runtime, Stylekit},
    widget::{MineralUi, MineralWidget, WidgetResponse},
};

/// Wrapping row of items laid out by taffy flexbox from the [`I18n::direction`](crate::i18n::I18n::direction) start edge.
///
/// Items take their natural size, measured at the row width on the previous pass; a changed size
/// costs one discarded pass. Item state is keyed by call site, occurrence and item index.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flow {
    gap: Vec2,
    align: Align,
    id_salt: Option<IdSalt>,
}

impl Flow {
    /// `gap.x` between items, `gap.y` between lines; a negative or non-finite component counts as 0. `align` is the
    /// vertical alignment of items within a line: `Min` is top, `Max` bottom.
    pub fn new(gap: Vec2, align: Align) -> Self {
        Self {
            gap: vec2(
                crate::kit::sanitize::at_least(gap.x, 0.0.., "Flow gap.x"),
                crate::kit::sanitize::at_least(gap.y, 0.0.., "Flow gap.y"),
            ),
            align,
            id_salt: None,
        }
    }

    /// Replaces the call-site key, e.g. for a row that moves between call sites.
    pub fn id_salt(mut self, id_salt: impl Into<crate::semantic::KeyedSalt>) -> Self {
        self.id_salt = Some(id_salt.into().id_salt());
        self
    }

    /// Lays out the items `add` adds; state is keyed by call site unless [`Self::id_salt`] is set.
    #[track_caller]
    pub fn show<K, R>(self, ui: &mut MineralUi<'_, K>, add: impl FnOnce(&mut FlowItems<'_, K>) -> R) -> R {
        let rt = ui.rt();
        let key = match self.id_salt {
            Some(salt) => ui.make_persistent_id(salt),
            None => occurrence(ui, ui.make_persistent_id(("mineral::flow", Location::caller()))),
        };
        let dir = rt.core().i18n().direction();
        let width = ui.available_width();
        let origin = ui.available_rect_before_wrap().min;
        let style = taffy::Style {
            display: taffy::Display::Flex,
            direction: match dir {
                Direction::Ltr => taffy::Direction::Ltr,
                Direction::Rtl => taffy::Direction::Rtl,
            },
            flex_direction: taffy::FlexDirection::Row,
            flex_wrap: taffy::FlexWrap::Wrap,
            size: taffy::Size {
                width: taffy::Dimension::length(width),
                height: taffy::Dimension::auto(),
            },
            gap: taffy::Size {
                width: taffy::LengthPercentage::length(self.gap.x),
                height: taffy::LengthPercentage::length(self.gap.y),
            },
            align_items: match self.align {
                Align::Min => taffy::AlignItems::START,
                Align::Center => taffy::AlignItems::CENTER,
                Align::Max => taffy::AlignItems::END,
            },
            ..taffy::Style::default()
        };
        let space = taffy::Size {
            width: taffy::AvailableSpace::Definite(width),
            height: taffy::AvailableSpace::MaxContent,
        };
        // Zero height: egui_taffy re-lays out when the root size changes, which is then only on resize.
        let root = Rect::from_min_size(origin, vec2(width, 0.0));
        let out = Tui::create(ui, key, root, Some(space), style, |tui| {
            add(&mut FlowItems {
                tui,
                rt,
                scope: key,
                next: 0,
                width,
                dir,
            })
        });
        let height = out.container.layout().size.height;
        ui.advance_cursor_after_rect(Rect::from_min_size(origin, vec2(width, height)));
        out.inner
    }
}

fn occurrence(ui: &Ui, key: Id) -> Id {
    let pass = ui.ctx().cumulative_pass_nr();
    let seen = key.with("mineral::flow_seen");
    let n = ui.ctx().data_mut(|d| {
        let slot = d.get_temp_mut_or_default::<(u64, u32)>(seen);
        *slot = if slot.0 == pass { (pass, slot.1 + 1) } else { (pass, 0) };
        slot.1
    });
    key.with(n)
}

/// Item adder handed to the [`Flow::show`] closure.
pub struct FlowItems<'t, K> {
    tui: &'t mut Tui,
    rt: &'t Runtime<K>,
    scope: Id,
    next: usize,
    width: f32,
    dir: Direction,
}

crate::kit::opaque_debug!(FlowItems<'_, K>);

impl<K> FlowItems<'_, K> {
    /// One item; `add` sees the full row width and the item takes the size it used.
    pub fn add<R>(&mut self, add: impl FnOnce(&mut MineralUi<'_, K>) -> R) -> R {
        let scope = self.scope.with(self.next);
        self.next += 1;
        let (width, dir, rt) = (self.width, self.dir, self.rt);
        self.tui.ui_manual(|ui, _| {
            let node = ui.max_rect();
            // Anchored at the node's start edge so content outgrowing last pass's node is measured whole.
            let x = dir.offset_x(dir.start_edge(node), Rangef::new(0.0, 0.0 + width));
            let align = dir.start_align();
            let mut item = ui.new_child(
                UiBuilder::new()
                    .scope_id(scope)
                    .max_rect(Rect::from_min_size(pos2(x, node.top()), vec2(width, f32::INFINITY)))
                    .layout(Layout::top_down(align)),
            );
            let inner = add(&mut rt.ui(&mut item));
            let size = item.min_size();
            TuiContainerResponse {
                inner,
                min_size: size,
                intrinsic_size: None,
                max_size: size,
                infinite: Vec2b::FALSE,
            }
        })
    }

    /// Shows `widget` as one flow item.
    pub fn mineral<W: MineralWidget<Kit = K>>(&mut self, widget: W) -> WidgetResponse<W::Output>
    where
        K: Stylekit,
    {
        self.add(|ui| ui.mineral(widget))
    }

    /// Empty item `width` wide; the gap still applies on both sides. A negative or non-finite
    /// `width` counts as 0.
    pub fn space(&mut self, width: f32) {
        let width = crate::kit::sanitize::at_least(width, 0.0.., "Flow space width");
        self.add(|ui| ui.allocate_space(vec2(width, 0.0)));
    }
}

use egui::{Sense, vec2};
use egui_mineral::{
    kit,
    layout::Constraints,
    pagination::{PageItem, Spread},
    semantic::{self, KeyedSalt, Role, SemanticNode, StableId},
    widget::{Composite, MineralUi, SizeExt as _, WidgetResponse},
};

use crate::ColorToken::ColorNeutralForeground2;
use crate::{
    Fluent, FluentExt, Size,
    actions::{PushButton, ToggleButton},
    icons::{ICON_NEXT, ICON_PREVIOUS},
    spec::tokens::geometry as g,
    widget::{paint, vocab::ButtonAppearance},
};

// Not in Fluent UI React v9; Small subtle buttons, the selected page checked.
const SIZE: Size = Size::Small;
const ROW: f32 = 24.0;
const GAP: f32 = g::SPACING_HORIZONTAL_XS;
const ICON: f32 = 20.0;
// Narrower than the button min width: page numbers are one to three digits.
const PAGE_WIDTH: f32 = 28.0;

/// Page selector with siblings, boundaries and gaps; not in Fluent UI React v9.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Pagination<'a> {
    salt: KeyedSalt,
    label: Option<String>,
    page: &'a mut usize,
    total: usize,
    siblings: usize,
    boundaries: usize,
    first_last: bool,
    disabled: bool,
}

impl<'a> Pagination<'a> {
    /// `total` pages (0 shows one); `page` is the bound zero-based index, moved onto the last page
    /// when past it. Agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, total: usize, page: &'a mut usize) -> Self {
        Self {
            salt: id_salt.into(),
            label: None,
            page,
            total,
            siblings: 1,
            boundaries: 1,
            first_last: false,
            disabled: false,
        }
    }

    /// Replaces the `<id_salt>` key prefix.
    pub fn agent_key(mut self, salt: impl Into<KeyedSalt>) -> Self {
        self.salt = salt.into();
        self
    }

    /// Accessible name of the landmark; default `mineral-pagination`.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Pages shown on each side of the current one; default 1.
    pub fn siblings(mut self, siblings: usize) -> Self {
        self.siblings = siblings;
        self
    }

    /// Pages always shown at each end; default 1.
    pub fn boundaries(mut self, boundaries: usize) -> Self {
        self.boundaries = boundaries;
        self
    }

    /// First and last page buttons.
    pub fn first_last(mut self, first_last: bool) -> Self {
        self.first_last = first_last;
        self
    }

    crate::widget::setters! {
        /// Greys out every button.
        disabled
    }
}

impl Composite for Pagination<'_> {
    type Kit = Fluent;
    type Output = ();

    kit::widget_meta!(salt);

    fn compose(&mut self, ui: &mut MineralUi<'_, Fluent>, _: Constraints) -> WidgetResponse {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = ui.tokens();
        let fmt = rt.i18n().format();
        let dir = rt.i18n().direction();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let total = self.total.max(1);
        let cur = (*self.page).min(total - 1) + 1;
        let mut target = cur;
        semantic::emit_parent(ctx, id, || {
            let name = crate::tr_or(rt.i18n(), self.label.as_deref(), "mineral-pagination");
            let mut n = SemanticNode::new(StableId::of(&self.salt, "pagination"), Role::Navigation, name);
            n.state.disabled = self.disabled;
            n
        });
        let nav = |ui: &mut egui::Ui, salt: &'static str, icon, msg: &str, enabled: bool| {
            rt.ui(ui)
                .mineral(
                    PushButton::new(self.salt.child(salt), crate::tr(rt.i18n(), msg))
                        .icon(icon)
                        .label_visible(false)
                        .appearance(ButtonAppearance::Subtle)
                        .size(SIZE)
                        .disabled(self.disabled || !enabled),
                )
                .clicked()
        };
        let (back, fwd) = (dir.pick(ICON_PREVIOUS, ICON_NEXT), dir.pick(ICON_NEXT, ICON_PREVIOUS));
        let (chev_back, chev_fwd) = (
            dir.pick(rt.icons().chevron_left, rt.icons().chevron_right),
            dir.pick(rt.icons().chevron_right, rt.icons().chevron_left),
        );
        let builder = crate::widget::row(ui, dir, ROW);
        let row = ui.scope_builder(builder, |ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            if self.first_last && nav(ui, "first", back.into(), "mineral-pagination-first", cur > 1) {
                target = 1;
            }
            if nav(ui, "prev", chev_back, "mineral-pagination-prev", cur > 1) {
                target = cur - 1;
            }
            let spread = Spread {
                siblings: self.siblings,
                boundaries: self.boundaries,
            };
            for item in spread.items(cur, total) {
                match item {
                    PageItem::Page(n) => {
                        let label = fmt.count(n);
                        let mut on = n == cur;
                        let button = ToggleButton::new(self.salt.child(("page", n)), label, &mut on)
                            .appearance(ButtonAppearance::Subtle)
                            .size(SIZE)
                            .disabled(self.disabled)
                            .agent_key(StableId::of(&self.salt, (n, "page-button")))
                            .preferred_width(PAGE_WIDTH);
                        if rt.ui(ui).mineral(button).changed() {
                            target = n;
                        }
                    }
                    PageItem::Gap => {
                        let (r, _) = ui.allocate_exact_size(vec2(ROW, ROW), Sense::hover());
                        paint::icon(
                            ui.painter(),
                            r.center(),
                            rt.icons().more_horizontal,
                            false,
                            ICON,
                            t.color(ColorNeutralForeground2),
                        );
                    }
                }
            }
            if nav(ui, "next", chev_fwd, "mineral-pagination-next", cur < total) {
                target = cur + 1;
            }
            if self.first_last && nav(ui, "last", fwd.into(), "mineral-pagination-last", cur < total) {
                target = total;
            }
        });
        kit::semantic::remember_rect(ctx, id, row.response.rect);
        let changed = target - 1 != *self.page;
        *self.page = target - 1;
        WidgetResponse::new(row.response, ()).with_changed(changed)
    }
}

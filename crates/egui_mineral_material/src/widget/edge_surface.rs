use egui::{Color32, Frame, Id, Pos2, Response};
use egui_mineral::{
    kit,
    motion::{self, Visibility},
    overlay::{self, Dismiss},
    semantic::Role,
    widget::MineralUi,
};

use crate::spec::tokens::motion::{
    EASING_EMPHASIZED_ACCELERATE_CUBIC_BEZIER, EASING_EMPHASIZED_DECELERATE_CUBIC_BEZIER,
};
use crate::{Material, MaterialExt, Rt};

pub(crate) struct EdgeSurface<'n> {
    pub(crate) id: Id,
    pub(crate) name: &'n str,
    pub(crate) pos: Pos2,
    // `Some` makes it modal.
    pub(crate) scrim: Option<Color32>,
}

pub(crate) struct EdgeShown<R> {
    pub(crate) inner: R,
    pub(crate) response: Response,
    pub(crate) close: Option<Dismiss>,
}

impl EdgeSurface<'_> {
    pub(crate) fn show<R>(self, rt: &Rt, body: impl FnOnce(&mut MineralUi<'_, Material>) -> R) -> EdgeShown<R> {
        let ctx = rt.ctx();
        // Motion is the caller's offset and scrim; egui's area fade would add an alpha ramp.
        let area = kit::overlay::floating(rt.core(), self.id.with("area"), self.pos)
            .fade_in(false)
            .role(Role::Dialog)
            .accessible_name(self.name);
        if let Some(scrim) = self.scrim {
            let r = egui::Modal::new(self.id)
                .frame(Frame::NONE)
                .backdrop_color(scrim)
                .area(area)
                .show(ctx, |ui| body(&mut rt.ui(ui)));
            EdgeShown {
                close: overlay::modal(&r),
                inner: r.inner,
                response: r.response,
            }
        } else {
            let r = area.show(ctx, |ui| body(&mut rt.ui(ui)));
            EdgeShown {
                inner: r.inner,
                response: r.response,
                close: None,
            }
        }
    }
}

pub(crate) fn slide(rt: &Rt, id: Id, open: bool) -> Option<f32> {
    let (visibility, easing) = if open {
        (Visibility::Visible, EASING_EMPHASIZED_DECELERATE_CUBIC_BEZIER)
    } else {
        (Visibility::Hidden, EASING_EMPHASIZED_ACCELERATE_CUBIC_BEZIER)
    };
    let k = motion::presence(rt.core(), id, visibility, rt.tokens().duration.medium[1])?;
    Some(easing.ease(k))
}

pub(crate) fn dismiss(open: &mut bool, close: Option<Dismiss>) -> Option<Dismiss> {
    let dismissed = close.filter(|_| *open);
    if dismissed.is_some() {
        *open = false;
    }
    dismissed
}

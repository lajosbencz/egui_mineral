use egui::{CornerRadius, Painter, Rect, Stroke};
use egui_mineral::kit::paint::FocusStroke;

use crate::{spec::ColorToken, spec::tokens::geometry, theme::Tokens};

// react-tabster createFocusOutlineStyle: `strokeWidthThick` `colorStrokeFocus2` outside, `strokeWidthThin`
// `colorStrokeFocus1` inside.
pub(crate) fn outline(p: &Painter, rect: Rect, r: CornerRadius, t: &Tokens) {
    p.add(
        FocusStroke::inside(Stroke::new(
            geometry::STROKE_WIDTH_THIN,
            t.color(ColorToken::ColorStrokeFocus1),
        ))
        .shape(rect, r),
    );
    p.add(
        FocusStroke::outside(Stroke::new(
            geometry::STROKE_WIDTH_THICK,
            t.color(ColorToken::ColorStrokeFocus2),
        ))
        .shape(rect, r),
    );
}

// react-button useButtonStyles.styles.ts createCustomFocusIndicatorStyle: the `strokeWidthThin` border turns
// `colorStrokeFocus2` and gains a `strokeWidthThin` inset ring of it; `on_brand` adds the primary's
// `strokeWidthThick` `colorNeutralForegroundOnBrand` inset ring under it.
pub(crate) fn inset(p: &Painter, rect: Rect, r: CornerRadius, on_brand: bool, t: &Tokens) {
    if on_brand {
        p.add(
            FocusStroke::inside(Stroke::new(
                geometry::STROKE_WIDTH_THICK,
                t.color(ColorToken::ColorNeutralForegroundOnBrand),
            ))
            .offset(-geometry::STROKE_WIDTH_THIN)
            .shape(rect, r),
        );
    }
    p.add(
        FocusStroke::inside(Stroke::new(
            geometry::STROKE_WIDTH_THIN + geometry::STROKE_WIDTH_THIN,
            t.color(ColorToken::ColorStrokeFocus2),
        ))
        .shape(rect, r),
    );
}

// react-link useLinkStyles.styles.ts focusIndicator: `double` underline in `colorStrokeFocus2`.
pub(crate) fn double_underline(p: &Painter, left: f32, right: f32, baseline: f32, t: &Tokens) {
    let stroke = Stroke::new(geometry::STROKE_WIDTH_THIN, t.color(ColorToken::ColorStrokeFocus2));
    for dy in [0.0, geometry::STROKE_WIDTH_THICK] {
        p.hline(left..=right, baseline + dy, stroke);
    }
}

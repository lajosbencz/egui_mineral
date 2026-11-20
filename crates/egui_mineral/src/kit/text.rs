//! Text layout jobs shared by stylekits; kits lay them out with their painter.

use egui::{Context, TextFormat, text::LayoutJob};

/// Job of `text` in `format` with every ASCII digit in a cell as wide as the font's widest digit,
/// flush to the cell's end, so a number keeps its width while its digits change. Stands in for
/// OpenType `tnum`, which epaint cannot request.
pub fn tabular_figures(ctx: &Context, text: &str, format: &TextFormat) -> LayoutJob {
    let widths = ctx
        .fonts_mut(|f| ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'].map(|d| f.glyph_width(&format.font_id, d)));
    let cell = widths.iter().copied().fold(0.0, f32::max);
    let mut job = LayoutJob::default();
    let mut buf = [0; 4];
    for c in text.chars() {
        let pad = c
            .to_digit(10)
            .filter(|_| c.is_ascii_digit())
            .and_then(|d| widths.get(d as usize))
            .map_or(0.0, |w| cell - w);
        job.append(c.encode_utf8(&mut buf), pad, format.clone());
    }
    job
}

#[cfg(test)]
mod tests {
    use egui::{FontId, TextFormat, text::LayoutJob};

    #[test]
    fn digits_share_one_advance() {
        let ctx = crate::test_support::ctx();
        let format = TextFormat::simple(FontId::proportional(14.0), egui::Color32::WHITE);
        let mut widths = Vec::new();
        crate::test_support::run(&ctx, egui::RawInput::default(), |ui| {
            let width = |job| ui.fonts_mut(|f| f.layout_job(job)).size().x;
            widths = ["f 111", "f 000", "f 478"]
                .map(|s| width(super::tabular_figures(ui.ctx(), s, &format)))
                .to_vec();
            let plain = LayoutJob::single_section("f 000".to_owned(), format.clone());
            assert!(widths[0] >= width(plain) && widths[0] > 0.0);
        });
        assert!(widths.iter().all(|w| (w - widths[0]).abs() < 0.01), "{widths:?}");
    }
}

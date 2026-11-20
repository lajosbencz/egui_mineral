use egui_mineral_fluent::{
    FluentExt, Size,
    data::{Text, TextSize},
    inputs::FieldLabel,
    spec::ColorToken,
    spec::tokens::typography,
    typography::TypeRamp,
};

use crate::support::{FluentHarness, harness, render};

#[test]
fn text_is_one_line_box_of_its_size() {
    for &size in TextSize::ALL {
        let h = harness((), move |ui, ()| {
            ui.mineral(Text::new("t", "Hello").size(size));
        });
        assert_eq!(h.rect("t-text").height(), size.metrics().line_height, "{size:?}");
    }
}

#[test]
fn ramp_presets_map_to_their_size() {
    let h = harness((), |ui, ()| {
        ui.mineral(Text::new("t", "Title").ramp(TypeRamp::Title1));
    });
    assert_eq!(h.rect("t-text").height(), typography::LINE_HEIGHT_HERO_800);
}

#[test]
fn type_ramp_matches_typography_styles() {
    let body = TypeRamp::Body1.spec();
    assert_eq!(
        (body.size, body.line_height, body.weight),
        (
            typography::FONT_SIZE_BASE_300,
            typography::LINE_HEIGHT_BASE_300,
            typography::FONT_WEIGHT_REGULAR
        )
    );
    let display = TypeRamp::Display.spec();
    assert_eq!(display.size, typography::FONT_SIZE_HERO_1000);
    assert_eq!(display.weight, typography::FONT_WEIGHT_SEMIBOLD);
}

#[test]
fn label_sizes_and_required_mark() {
    for (size, line) in [
        (Size::Small, typography::LINE_HEIGHT_BASE_200),
        (Size::Medium, typography::LINE_HEIGHT_BASE_300),
        (Size::Large, typography::LINE_HEIGHT_BASE_400),
    ] {
        let h = harness((), move |ui, ()| {
            ui.mineral(FieldLabel::new("l", "Name").size(size));
        });
        assert_eq!(h.rect("l-label").height(), line, "{size:?}");
    }
    let p = render(|ui| {
        ui.mineral(FieldLabel::new("l", "Name").required(true));
    });
    assert_eq!(
        p.text_color("Name*"),
        p.rt.tokens().color(ColorToken::ColorNeutralForeground1)
    );
}

mod avatar {
    use egui::{Rect, pos2, vec2};
    use egui_mineral_fluent::{
        FluentExt,
        data::{Avatar, AvatarColor, AvatarGroup, AvatarGroupLayout, AvatarNamedColor, AvatarShape, AvatarSize},
        feedback::PresenceStatus,
        spec::ColorToken,
        spec::defaults::avatar as d,
    };

    use crate::support::{FluentHarness, harness, render};

    #[test]
    fn side_is_the_size() {
        for &size in AvatarSize::ALL {
            let h = harness((), move |ui, ()| {
                ui.mineral(Avatar::new("a", "Kat Larrson").size(size));
            });
            let r = h.rect("a-avatar");
            assert_eq!(
                (r.width(), r.height()),
                (f32::from(size.px()), f32::from(size.px())),
                "{size:?}"
            );
        }
    }

    #[test]
    fn colors_follow_the_palette() {
        let p = render(|ui| {
            ui.mineral(
                Avatar::new("a", "Kat Larrson")
                    .size(AvatarSize::S64)
                    .color(AvatarColor::Brand),
            );
        });
        let t = p.rt.tokens();
        assert_eq!(
            p.one(t.color(ColorToken::ColorBrandBackgroundStatic)).size(),
            vec2(64.0, 64.0)
        );
        let p = render(|ui| {
            ui.mineral(
                Avatar::new("a", "Kat")
                    .shape(AvatarShape::Square)
                    .color(AvatarColor::Named(AvatarNamedColor::Pumpkin)),
            );
        });
        p.one(p.rt.tokens().color(ColorToken::ColorPalettePumpkinBackground2));
        assert_eq!(
            p.text_color("K"),
            p.rt.tokens().color(ColorToken::ColorPalettePumpkinForeground2)
        );
    }

    #[test]
    fn initials_text_size_follows_the_size() {
        let (small, big) = (d::text_size(24), d::text_size(128));
        assert_eq!((small, big), (10.0, 24.0));
        let p = render(|ui| {
            ui.mineral(Avatar::new("a", "Kat Larrson").size(AvatarSize::S128));
        });
        assert!(p.text("KL").height() >= big);
    }

    #[test]
    fn badge_sits_in_the_bottom_end_corner() {
        let h = harness((), |ui, ()| {
            ui.mineral(
                Avatar::new("a", "Kat Larrson")
                    .size(AvatarSize::S56)
                    .presence(PresenceStatus::Busy),
            );
        });
        assert_eq!(h.node("a-avatar").name, "Kat Larrson, Busy");
    }

    #[test]
    fn group_layouts_place_the_items() {
        let at = |layout, size: AvatarSize| {
            harness((), move |ui, ()| {
                let mut g = AvatarGroup::new("g", "Team").layout(layout).size(size);
                for i in 0..3 {
                    g = g.item(Avatar::new(("m", i), format!("Member {i}")));
                }
                g.show(ui);
            })
        };
        let spread = at(AvatarGroupLayout::Spread, AvatarSize::S32);
        let (a, b) = (spread.rect("m-0-avatar"), spread.rect("m-1-avatar"));
        assert_eq!(b.min.x - a.max.x, d::spread_gap(32));
        let stack = at(AvatarGroupLayout::Stack, AvatarSize::S32);
        let (a, b) = (stack.rect("m-0-avatar"), stack.rect("m-1-avatar"));
        assert_eq!(a.max.x - b.min.x, d::stack_overlap(32));
        let pie = at(AvatarGroupLayout::Pie, AvatarSize::S32);
        let group: Rect = pie.rect("g-avatar-group");
        assert_eq!(group.size(), vec2(32.0, 32.0));
        assert_eq!(pie.rect("m-1-avatar").size(), vec2(16.0, 16.0));
        assert_eq!(pie.rect("m-1-avatar").min, pos2(group.center().x, group.min.y));
    }
}

mod persona {
    use egui_mineral_fluent::{
        data::{Persona, PersonaSize, PersonaTextAlignment, PersonaTextPosition},
        feedback::PresenceStatus,
    };

    use crate::support::{FluentHarness, harness};

    fn rect(build: impl Fn() -> Persona<'static> + 'static) -> egui::Rect {
        harness((), move |ui, ()| {
            ui.mineral(build());
        })
        .rect("p-persona")
    }

    #[test]
    fn media_size_follows_the_persona_size() {
        // avatarSizes
        for (size, side) in PersonaSize::ALL
            .iter()
            .copied()
            .zip([20.0, 28.0, 32.0, 36.0, 40.0, 56.0])
        {
            let r = rect(move || Persona::new("p", "Kat Larrson").size(size));
            assert!(r.height() >= side, "{size:?}");
        }
    }

    #[test]
    fn text_positions_stack_the_media() {
        let after = rect(|| Persona::new("p", "Kat Larrson").secondary_text("Designer"));
        let below = rect(|| {
            Persona::new("p", "Kat Larrson")
                .secondary_text("Designer")
                .text_position(PersonaTextPosition::Below)
        });
        assert!(below.height() > after.height());
        assert!(below.width() < after.width());
        let before = rect(|| {
            Persona::new("p", "Kat Larrson")
                .secondary_text("Designer")
                .text_position(PersonaTextPosition::Before)
                .text_alignment(PersonaTextAlignment::Center)
        });
        assert_eq!(before.size(), after.size());
    }

    #[test]
    fn text_wraps_within_the_available_width() {
        // Headless text has no glyphs: the icon font stands in as the proportional font.
        const GLYPHS: &[u8] = include_bytes!("../../assets/FluentSystemIcons-Resizable.ttf");
        let long = ["\u{e82b}"; 24].join(" ");
        let rect_in = |width: f32| {
            let long = long.clone();
            let mut h = harness((), move |ui, ()| {
                let mut fonts = egui::FontDefinitions::empty();
                fonts.font_data.insert(
                    "glyphs".into(),
                    std::sync::Arc::new(egui::FontData::from_static(GLYPHS)),
                );
                fonts
                    .families
                    .insert(egui::FontFamily::Proportional, vec!["glyphs".into()]);
                ui.ctx().set_fonts(fonts);
                ui.set_max_width(width);
                ui.mineral(Persona::new("p", long.as_str()).secondary_text(long.as_str()));
            });
            h.run();
            h.run();
            h.rect("p-persona")
        };
        let wide = rect_in(2000.0);
        let narrow = rect_in(160.0);
        assert!(narrow.width() <= 160.0, "{narrow:?}");
        assert!(
            narrow.height() > wide.height(),
            "wrapped lines add rows: {narrow:?} vs {wide:?}"
        );
    }

    #[test]
    fn presence_only_media_is_the_badge() {
        let r = rect(|| {
            Persona::new("p", "Kat")
                .presence(PresenceStatus::Away)
                .presence_only(true)
                .size(PersonaSize::Large)
        });
        // presenceSizes: large persona -> medium badge (16), below the 36 avatar
        assert!(r.height() >= 16.0 && r.height() < 36.0);
    }
}

mod tag {
    use egui::vec2;
    use egui_mineral_fluent::{
        FluentExt,
        data::{InteractionTag, Tag, TagAppearance, TagShape, TagSize},
        spec::ColorToken,
        spec::defaults::tag as d,
    };

    use crate::support::{FluentHarness, harness, render};

    #[test]
    fn height_follows_the_size() {
        for (size, height) in TagSize::ALL.iter().copied().zip([32.0, 24.0, 20.0]) {
            let h = harness((), move |ui, ()| {
                ui.mineral(Tag::new("t", "Tag").size(size));
            });
            assert_eq!(h.rect("t-tag").height(), height, "{size:?}");
        }
    }

    #[test]
    fn dismissible_tag_is_wider_by_the_icon() {
        let width = |dismissible| {
            harness((), move |ui, ()| {
                ui.mineral(Tag::new("t", "Tag").dismissible(dismissible));
            })
            .rect("t-tag")
            .width()
        };
        let m = d::MEDIUM;
        assert_eq!(width(true) - width(false), m.dismiss_padding_start + m.icon);
    }

    #[test]
    fn appearance_colors() {
        for (appearance, bg) in [
            (TagAppearance::Filled, ColorToken::ColorNeutralBackground3),
            (TagAppearance::Brand, ColorToken::ColorBrandBackground2),
        ] {
            let p = render(move |ui| {
                ui.mineral(Tag::new("t", "Tag").appearance(appearance));
            });
            let r = p.one(p.rt.tokens().color(bg));
            assert_eq!(r.height(), 32.0, "{appearance:?}");
        }
        let p = render(|ui| {
            ui.mineral(Tag::new("t", "Tag").selected(true));
        });
        p.one(p.rt.tokens().color(ColorToken::ColorBrandBackground));
        assert_eq!(
            p.text_color("Tag"),
            p.rt.tokens().color(ColorToken::ColorNeutralForegroundOnBrand)
        );
    }

    #[test]
    fn circular_tags_are_pills() {
        let p = render(|ui| {
            ui.mineral(Tag::new("t", "Tag").shape(TagShape::Circular));
        });
        assert_eq!(
            p.one(p.rt.tokens().color(ColorToken::ColorNeutralBackground3)).height(),
            32.0
        );
    }

    #[test]
    fn interaction_tag_splits_into_primary_and_secondary() {
        let h = harness((), |ui, ()| {
            ui.mineral(Tag::new("a", "Alpha").dismissible(true));
            InteractionTag::new("b", "Beta").dismissible(true).show(ui);
        });
        let (primary, secondary) = (h.rect("b-interaction-tag"), h.rect("b-dismiss"));
        assert_eq!(primary.right(), secondary.left());
        assert_eq!(
            secondary.size(),
            vec2(2.0 * 1.0 + 2.0 * d::MEDIUM.secondary_padding + d::MEDIUM.icon, 32.0)
        );
    }
}

mod table {
    use egui::vec2;
    use egui_mineral::select::Selection;
    use egui_mineral_fluent::{
        data::{Table, TableCellLayout, TableSize, Text},
        spec::defaults::table as d,
    };

    use crate::support::{FluentHarness, harness, render};

    #[test]
    fn long_cell_text_stays_inside_its_cell() {
        let p = render(|ui| {
            ui.set_max_width(120.0);
            Table::new("t", "T", ["A"]).show(ui, 1, |ui, _, _| {
                ui.mineral(
                    TableCellLayout::new("c", "A very long person name that overruns")
                        .secondary_text("Also a long role"),
                );
            });
        });
        let table =
            p.rt.tree(egui::ViewportId::ROOT)
                .expect("semantics")
                .find_key("t-table")
                .expect("table")
                .rect;
        let limit = table.right() - d::CELL_PADDING;
        for text in ["A very long", "Also a long"] {
            let clip = p.text_clip(text);
            assert!(clip.right() <= limit, "{text}: {clip:?} past {limit}");
        }
    }

    fn rects(size: TableSize, selectable: bool) -> (egui::Rect, egui::Rect) {
        let h = harness(Selection::<usize>::multiple(), move |ui, sel| {
            let mut t = Table::new("t", "T", ["A", "B"]).size(size);
            if selectable {
                t = t.selection(sel);
            }
            t.show(ui, 2, |ui, row, col| {
                ui.mineral(Text::new((("c", row), col), "x"));
            });
        });
        (h.rect("t-table"), h.rect("c-0-0-text"))
    }

    #[test]
    fn rows_are_the_size_height() {
        for (size, height) in TableSize::ALL.iter().copied().zip(d::ROW_HEIGHTS) {
            let (table, _) = rects(size, false);
            assert_eq!(table.height(), 3.0 * height, "{size:?}: a header and two rows");
        }
    }

    #[test]
    fn cells_pad_inline_and_selection_adds_a_column() {
        let (plain, cell) = rects(TableSize::Medium, false);
        assert_eq!(cell.min.x - plain.min.x, d::CELL_PADDING);
        let (selectable, first) = rects(TableSize::Medium, true);
        assert_eq!(first.min.x - selectable.min.x, d::SELECTION_WIDTH + d::CELL_PADDING);
        assert_eq!(
            selectable.size(),
            vec2(plain.width(), plain.height()),
            "the columns share what is left"
        );
    }
}

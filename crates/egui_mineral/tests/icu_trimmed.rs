//! Formatting with `ICU4X_DATA_DIR` baked for `en` and `ar` only (ci.yml generates it).

use egui_mineral::{
    i18n::{DateFields, DateLength, Direction, I18n, IcuFormat, LocaleFormat},
    jiff::civil::date,
};

egui_mineral::i18n::messages!(
    #[expect(dead_code, reason = "one message suffices")]
    mod msg = "tests/locales/en-US.ftl"
);

const IGNORED: &str = "needs ICU4X_DATA_DIR baked for en and ar; run by ci.yml";

fn long_date(f: &impl LocaleFormat) -> String {
    f.date(date(2026, 11, 18), DateFields::YearMonthDay, DateLength::Long)
}

#[test]
#[ignore = "needs ICU4X_DATA_DIR baked for en and ar; run by ci.yml"]
fn baked_locales_format_natively_and_others_as_root() {
    assert!(option_env!("ICU4X_DATA_DIR").is_some(), "{IGNORED}");
    let root = IcuFormat::new(&egui_mineral::i18n::locale!("und")).expect("root");
    let en = IcuFormat::new(&egui_mineral::i18n::locale!("en-US")).expect("en");
    assert_eq!(long_date(&en), "November 18, 2026");

    let ar = IcuFormat::new(&egui_mineral::i18n::locale!("ar-EG")).expect("ar");
    assert_eq!(ar.direction(), Direction::Rtl);
    assert_ne!(long_date(&ar), long_date(&root), "ar is baked");

    let de = IcuFormat::new(&egui_mineral::i18n::locale!("de-DE")).expect("unbaked locales load");
    assert_eq!(long_date(&de), long_date(&root), "unbaked de formats as root");
    let he = IcuFormat::new(&egui_mineral::i18n::locale!("he-IL")).expect("unbaked locales load");
    assert_eq!(he.direction(), Direction::Rtl, "direction data is not per locale");
}

#[test]
#[ignore = "needs ICU4X_DATA_DIR baked for en and ar; run by ci.yml"]
fn switching_to_an_unbaked_locale_keeps_its_tag() {
    assert!(option_env!("ICU4X_DATA_DIR").is_some(), "{IGNORED}");
    let ctx = egui::Context::default();
    let i18n = I18n::new(&ctx, &[msg::CATALOG]).expect("i18n");
    i18n.set_locale(&egui_mineral::i18n::locale!("de-DE"))
        .expect("unbaked locale");
    assert_eq!(*i18n.format().locale(), egui_mineral::i18n::locale!("de-DE"));
    assert_eq!(i18n.tr(msg::HELLO), "Hello", "messages negotiate separately");
}

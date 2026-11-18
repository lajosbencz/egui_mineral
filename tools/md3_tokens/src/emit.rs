//! Rust output: one module per emitted token object, `roles.rs` from the consumed objects, `mod.rs`.

use std::collections::{BTreeMap, BTreeSet};

macro_rules! w {
    ($out:expr, $($arg:tt)*) => {
        {
            use std::fmt::Write as _;
            $out.write_fmt(format_args!($($arg)*)).expect("writing to a String cannot fail")
        }
    };
}

use crate::{
    CONSUMED, Error, REPO, SOURCE_DIR, Source,
    kotlin::{Decl, Object, Value},
};

const MAX_WIDTH: usize = 120;
const ARRAY_WIDTH: usize = MAX_WIDTH * 6 / 10;
const SIDES: [&str; 4] = ["Top", "Bottom", "Start", "End"];
// Compose FontWeight
const WEIGHTS: [(&str, u16); 9] = [
    ("Thin", 100),
    ("ExtraLight", 200),
    ("Light", 300),
    ("Normal", 400),
    ("Medium", 500),
    ("SemiBold", 600),
    ("Bold", 700),
    ("ExtraBold", 800),
    ("Black", 900),
];
const RUST_KEYWORDS: [&str; 39] = [
    "as", "async", "await", "box", "break", "const", "continue", "crate", "do", "dyn", "else", "enum", "extern",
    "false", "final", "fn", "for", "gen", "if", "impl", "in", "let", "loop", "macro", "match", "mod", "move", "mut",
    "override", "priv", "pub", "ref", "return", "self", "static", "struct", "super", "trait", "true",
];

struct Roles {
    color: Vec<String>,
    typography: Vec<String>,
    elevation: Vec<f32>,
    families: Vec<String>,
    weights: BTreeMap<String, u16>,
    scales: Scales,
    shapes: Shapes,
}

pub(crate) fn files(sha: &str, sources: &[Source]) -> Result<BTreeMap<String, String>, Error> {
    let by_name = |name: &str| {
        sources
            .iter()
            .find(|s| s.object.name == name)
            .ok_or_else(|| Error::new(format!("token object {name} is not vendored")))
    };
    let roles = roles(&by_name)?;
    let mut files = BTreeMap::new();
    let consumed: Vec<&Source> = CONSUMED.iter().map(|n| by_name(n)).collect::<Result<_, _>>()?;
    files.insert("roles.rs".to_owned(), roles_rs(sha, &consumed, &roles));
    let mut modules = Vec::new();
    for src in sources.iter().filter(|s| !CONSUMED.contains(&s.object.name.as_str())) {
        let base = src
            .object
            .name
            .strip_suffix("Tokens")
            .ok_or_else(|| Error::new(format!("{}: object name lacks the Tokens suffix", src.file)))?;
        let module = snake(base);
        if RUST_KEYWORDS.contains(&module.as_str()) || module == "roles" || modules.contains(&module) {
            return Err(Error::new(format!(
                "{}: module name {module} is reserved or taken",
                src.file
            )));
        }
        let text = module_rs(sha, src, &roles).map_err(|(line, e)| Error::new(format!("{}:{line}: {e}", src.file)))?;
        files.insert(format!("{module}.rs"), text);
        modules.push(module);
    }
    modules.sort();
    files.insert("mod.rs".to_owned(), mod_rs(sha, &modules));
    Ok(files)
}

fn names_of<'a>(obj: &'a Object, what: &str, pred: impl Fn(&Value) -> bool) -> Result<Vec<String>, Error> {
    obj.decls
        .iter()
        .map(|d: &'a Decl| {
            if pred(&d.value) {
                Ok(d.name.clone())
            } else {
                Err(Error::new(format!(
                    "{}:{}: {} is not a {what}",
                    obj.name, d.line, d.name
                )))
            }
        })
        .collect()
}

fn roles<'a>(by_name: &impl Fn(&str) -> Result<&'a Source, Error>) -> Result<Roles, Error> {
    let key = |v: &Value| matches!(v, Value::Key(_));
    let color = names_of(&by_name("ColorSchemeKeyTokens")?.object, "key", key)?;
    let typography = names_of(&by_name("TypographyKeyTokens")?.object, "key", key)?;
    let shape_keys = names_of(&by_name("ShapeKeyTokens")?.object, "key", key)?;
    let mut elevation = Vec::new();
    for (i, d) in by_name("ElevationTokens")?.object.decls.iter().enumerate() {
        match d.value {
            Value::Dp(dp) if d.name == format!("Level{i}") => elevation.push(dp),
            _ => {
                return Err(Error::new(format!(
                    "ElevationTokens:{}: expected a Level{i} dp, got {}",
                    d.line, d.name
                )));
            }
        }
    }

    let typeface = &by_name("TypefaceTokens")?.object;
    let mut families = Vec::new();
    let mut weights = BTreeMap::new();
    for d in &typeface.decls {
        match &d.value {
            Value::FontFamily(_) => families.push(d.name.clone()),
            Value::FontWeight(w) => {
                let (_, n) = WEIGHTS
                    .iter()
                    .find(|(name, _)| name == w)
                    .ok_or_else(|| Error::new(format!("TypefaceTokens:{}: unknown FontWeight.{w}", d.line)))?;
                weights.insert(d.name.clone(), *n);
            }
            v => return Err(Error::new(format!("TypefaceTokens:{}: unsupported {v:?}", d.line))),
        }
    }

    let (scales, shapes) = shape_roles(&shape_keys, &by_name("ShapeTokens")?.object)?;

    Ok(Roles {
        color,
        typography,
        elevation,
        families,
        weights,
        scales,
        shapes,
    })
}

type Scales = Vec<(String, Option<f32>)>;
type Shapes = Vec<(String, String, &'static str)>;

fn shape_roles(shape_keys: &[String], shape_tokens: &Object) -> Result<(Scales, Shapes), Error> {
    let mut scales = Vec::new();
    for d in &shape_tokens.decls {
        if let Some(scale) = d.name.strip_prefix("CornerValue") {
            let Value::CornerSize(dp) = d.value else {
                return Err(Error::new(format!(
                    "ShapeTokens:{}: {} is not a CornerSize",
                    d.line, d.name
                )));
            };
            scales.push((scale.to_owned(), Some(dp)));
        }
    }
    scales.sort_by(|a, b| a.1.partial_cmp(&b.1).expect("dp literals are finite"));
    if shape_keys.iter().any(|k| k == "CornerFull") {
        scales.push(("Full".to_owned(), None));
    }

    let mut shapes = Vec::new();
    let mut seen_shapes = BTreeSet::new();
    for key in shape_keys {
        let rest = key
            .strip_prefix("Corner")
            .ok_or_else(|| Error::new(format!("ShapeKeyTokens: {key} lacks the Corner prefix")))?;
        let (scale, side) = SIDES
            .iter()
            .find_map(|s| rest.strip_suffix(s).map(|scale| (scale, *s)))
            .unwrap_or((rest, "All"));
        let (_, dp) = scales.iter().find(|(s, _)| s == scale).ok_or_else(|| {
            Error::new(format!(
                "ShapeKeyTokens: {key} has no CornerValue{scale} in ShapeTokens"
            ))
        })?;
        let decl = shape_tokens
            .decls
            .iter()
            .find(|d| d.name == *key)
            .ok_or_else(|| Error::new(format!("ShapeTokens: no {key} shape")))?;
        let expected = match (side, dp) {
            ("All", None) => Value::Circle,
            ("All", Some(d)) if *d <= 0.0 => Value::Rectangle,
            ("All", Some(d)) => Value::RoundedAll(*d),
            (_, None) => return Err(Error::new(format!("ShapeKeyTokens: {key}: Full has no sided form"))),
            (side, Some(d)) => {
                let d = *d;
                Value::RoundedCorners(match side {
                    "Top" => [d, d, 0.0, 0.0],
                    "Bottom" => [0.0, 0.0, d, d],
                    "Start" => [d, 0.0, 0.0, d],
                    _ => [0.0, d, d, 0.0],
                })
            }
        };
        if decl.value != expected {
            return Err(Error::new(format!(
                "ShapeTokens:{}: {key} is {:?}, its name implies {expected:?}",
                decl.line, decl.value
            )));
        }
        seen_shapes.insert(key.as_str());
        shapes.push((key.clone(), scale.to_owned(), side));
    }
    for d in &shape_tokens.decls {
        if !d.name.starts_with("CornerValue") && !seen_shapes.contains(d.name.as_str()) {
            return Err(Error::new(format!(
                "ShapeTokens:{}: {} has no ShapeKeyTokens role",
                d.line, d.name
            )));
        }
    }
    Ok((scales, shapes))
}

fn snake(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && c.is_ascii_uppercase() {
            let prev = chars[i - 1];
            let next_lower = chars.get(i + 1).is_some_and(char::is_ascii_lowercase);
            if prev.is_ascii_lowercase() || prev.is_ascii_digit() || (prev.is_ascii_uppercase() && next_lower) {
                out.push('_');
            }
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

fn header(sha: &str, sources: &[&Source]) -> String {
    let mut out = format!("// @generated by tools/md3_tokens from {REPO}@{sha}, do not edit.\n");
    for src in sources {
        w!(out, "// {SOURCE_DIR}/{}\n", src.file);
    }
    let mut copyrights: Vec<&str> = sources.iter().map(|s| s.copyright.as_str()).collect();
    copyrights.sort_unstable();
    copyrights.dedup();
    for c in copyrights {
        w!(out, "// Copyright {c}\n");
    }
    out.push_str("// Licensed under the Apache License, Version 2.0; see NOTICE and LICENSE-APACHE.\n");
    out
}

fn const_item(out: &mut String, doc: &str, name: &str, ty: &str, expr: &str) {
    w!(out, "/// {doc}\n");
    let line = format!("pub const {name}: {ty} = {expr};");
    if line.len() <= MAX_WIDTH {
        out.push_str(&line);
        out.push('\n');
    } else {
        w!(out, "pub const {name}: {ty} =\n    {expr};\n");
    }
}

fn integral(v: f64, max: f64, what: &str) -> Result<String, String> {
    if v.fract() == 0.0 && (0.0..=max).contains(&v) {
        Ok(format!("{v:.0}"))
    } else {
        Err(format!("{what} {v} is not an integer in 0..={max}"))
    }
}

fn rust_value(decl: &Decl, roles: &Roles) -> Result<(&'static str, String, Option<&'static str>), String> {
    let known = |list: &[String], name: &str, object: &str| {
        if list.iter().any(|n| n == name) {
            Ok(())
        } else {
            Err(format!("{object}.{name} is not defined"))
        }
    };
    Ok(match &decl.value {
        Value::Dp(v) | Value::Sp(v) | Value::Float(v) => ("f32", format!("{v:?}"), None),
        Value::Double(v) if decl.name.starts_with("Duration") => (
            "Duration",
            format!("Duration::from_millis({})", integral(*v, 1e9, "duration ms")?),
            Some("std::time::Duration"),
        ),
        Value::RoundedAll(v) => (
            "CornerRadius",
            format!(
                "CornerRadius::same({})",
                integral(f64::from(*v), 255.0, "corner radius dp")?
            ),
            Some("egui::CornerRadius"),
        ),
        Value::Easing([a, b, c, d]) => (
            "CubicBezier",
            format!("CubicBezier::new({a:?}, {b:?}, {c:?}, {d:?})"),
            Some("egui_mineral::motion::CubicBezier"),
        ),
        Value::Ref { object, name } => match object.as_str() {
            "ColorSchemeKeyTokens" => {
                known(&roles.color, name, object)?;
                (
                    "ColorRole",
                    format!("ColorRole::{name}"),
                    Some("crate::spec::ColorRole"),
                )
            }
            "ShapeKeyTokens" => {
                if !roles.shapes.iter().any(|(n, _, _)| n == name) {
                    return Err(format!("{object}.{name} is not defined"));
                }
                (
                    "ShapeRole",
                    format!("ShapeRole::{name}"),
                    Some("crate::spec::ShapeRole"),
                )
            }
            "TypographyKeyTokens" => {
                known(&roles.typography, name, object)?;
                (
                    "TypeScale",
                    format!("TypeScale::{name}"),
                    Some("crate::spec::TypeScale"),
                )
            }
            "ElevationTokens" => {
                let level = name
                    .strip_prefix("Level")
                    .and_then(|l| l.parse::<usize>().ok())
                    .filter(|l| *l < roles.elevation.len())
                    .ok_or_else(|| format!("{object}.{name} is not defined"))?;
                (
                    "ElevationLevel",
                    format!("ElevationLevel::Level{level}"),
                    Some("crate::spec::ElevationLevel"),
                )
            }
            "ShapeTokens" => {
                let scale = name
                    .strip_prefix("CornerValue")
                    .filter(|s| roles.scales.iter().any(|(n, _)| n == s))
                    .ok_or_else(|| format!("{object}.{name} is not a corner value"))?;
                (
                    "ShapeScale",
                    format!("ShapeScale::{scale}"),
                    Some("crate::spec::ShapeScale"),
                )
            }
            "TypefaceTokens" => {
                if let Some(w) = roles.weights.get(name) {
                    ("u16", w.to_string(), None)
                } else {
                    known(&roles.families, name, object)?;
                    ("Typeface", format!("Typeface::{name}"), Some("crate::spec::Typeface"))
                }
            }
            _ => return Err(format!("unsupported reference {object}.{name}")),
        },
        v => return Err(format!("{} = {v:?} is unsupported in an emitted object", decl.name)),
    })
}

fn module_rs(sha: &str, src: &Source, roles: &Roles) -> Result<String, (usize, String)> {
    let mut uses = BTreeSet::new();
    let mut items = String::new();
    let mut names = BTreeSet::new();
    for decl in &src.object.decls {
        let (ty, expr, import) = rust_value(decl, roles).map_err(|e| (decl.line, e))?;
        let name = snake(&decl.name).to_ascii_uppercase();
        if !names.insert(name.clone()) {
            return Err((decl.line, format!("duplicate constant {name}")));
        }
        uses.extend(import);
        const_item(
            &mut items,
            &format!("`{}.{}`.", src.object.name, decl.name),
            &name,
            ty,
            &expr,
        );
    }
    let mut out = header(sha, &[src]);
    w!(out, "//! `{}`", src.object.name);
    if let Some(v) = &src.object.version {
        w!(out, " (`VERSION: {v}`)");
    }
    out.push_str(".\n\n");
    for u in &uses {
        w!(out, "use {u};\n");
    }
    if !uses.is_empty() {
        out.push('\n');
    }
    out.push_str(&items);
    Ok(out)
}

// Enums tracking MD3 lists that can grow.
const NON_EXHAUSTIVE: [&str; 4] = ["ColorRole", "ShapeRole", "ShapeScale", "TypeScale"];

fn enum_item(out: &mut String, doc: &str, name: &str, variants: &[(&str, String)]) {
    let attr = if NON_EXHAUSTIVE.contains(&name) {
        "#[non_exhaustive]\n"
    } else {
        ""
    };
    w!(
        out,
        "\n/// {doc}\n#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\n{attr}pub enum {name} {{\n"
    );
    for (v, token) in variants {
        w!(out, "    /// `{token}`.\n    {v},\n");
    }
    out.push_str("}\n");
}

fn all_const(out: &mut String, doc: &str, variants: &[&str]) {
    let items: Vec<String> = variants.iter().map(|v| format!("Self::{v}")).collect();
    let inline = format!("[{}]", items.join(", "));
    w!(out, "    /// {doc}\n");
    w!(out, "    pub const ALL: [Self; {}] = ", variants.len());
    if inline.len() <= ARRAY_WIDTH {
        w!(out, "{inline};\n");
    } else {
        out.push_str("[\n");
        for item in items {
            w!(out, "        {item},\n");
        }
        out.push_str("    ];\n");
    }
}

fn match_fn(out: &mut String, doc: Option<&str>, signature: &str, arms: &[(&str, String)]) {
    let distinct: BTreeSet<&String> = arms.iter().map(|(_, v)| v).collect();
    out.push('\n');
    if let Some(doc) = doc {
        w!(out, "    /// {doc}\n");
    }
    if distinct.len() < arms.len() {
        out.push_str("    #[expect(clippy::match_same_arms, reason = \"roles sharing a value stay distinct\")]\n");
    }
    w!(out, "    {signature} {{\n        match self {{\n");
    for (v, expr) in arms {
        w!(out, "            Self::{v} => {expr},\n");
    }
    out.push_str("        }\n    }\n");
}

fn tokens<'a>(variants: &[&'a str], object: &str) -> Vec<(&'a str, String)> {
    variants.iter().map(|v| (*v, format!("{object}.{v}"))).collect()
}

fn declaration_order(object: &str) -> String {
    format!("Every variant, in `{object}` declaration order.")
}

fn shape_enums(out: &mut String, roles: &Roles) {
    let scales: Vec<&str> = roles.scales.iter().map(|(s, _)| s.as_str()).collect();
    enum_item(
        out,
        "Corner radius step of the shape scale, one per [`Shape`] field.",
        "ShapeScale",
        &scales
            .iter()
            .map(|s| {
                let token = if *s == "Full" {
                    "ShapeKeyTokens.CornerFull".to_owned()
                } else {
                    format!("ShapeTokens.CornerValue{s}")
                };
                (*s, token)
            })
            .collect::<Vec<_>>(),
    );
    out.push_str("\nimpl ShapeScale {\n");
    all_const(out, "Every variant, by ascending radius.", &scales);
    let arms: Vec<_> = scales.iter().map(|v| (*v, format!("shape.{}", snake(v)))).collect();
    match_fn(
        out,
        Some("Theme radius in dp, from `shape`."),
        "pub const fn radius(self, shape: &Shape) -> u8",
        &arms,
    );
    let arms: Vec<_> = roles
        .scales
        .iter()
        .map(|(s, dp)| {
            (
                s.as_str(),
                dp.map_or_else(|| "None".to_owned(), |d| format!("Some({d:?})")),
            )
        })
        .collect();
    match_fn(
        out,
        Some("Spec radius in dp; `None` is fully rounded."),
        "pub const fn dp(self) -> Option<f32>",
        &arms,
    );
    out.push_str("}\n");

    let shapes: Vec<&str> = roles.shapes.iter().map(|(n, _, _)| n.as_str()).collect();
    enum_item(
        out,
        "Shape role (`ShapeKeyTokens`).",
        "ShapeRole",
        &tokens(&shapes, "ShapeKeyTokens"),
    );
    out.push_str("\nimpl ShapeRole {\n");
    all_const(out, &declaration_order("ShapeKeyTokens"), &shapes);
    let arms: Vec<_> = roles
        .shapes
        .iter()
        .map(|(n, scale, _)| (n.as_str(), format!("ShapeScale::{scale}")))
        .collect();
    match_fn(
        out,
        Some("Radius step of the rounded corners."),
        "pub const fn scale(self) -> ShapeScale",
        &arms,
    );
    let arms: Vec<_> = roles
        .shapes
        .iter()
        .map(|(n, _, side)| (n.as_str(), format!("Corners::{side}")))
        .collect();
    match_fn(
        out,
        Some("Rounded corners; the others stay square."),
        "pub const fn corners(self) -> Corners",
        &arms,
    );
    out.push_str("}\n");
}

fn roles_rs(sha: &str, consumed: &[&Source], roles: &Roles) -> String {
    let mut out = header(sha, consumed);
    out.push_str(
        "//! Role enums from the key token objects.\n\n\
         use crate::color::ColorScheme;\nuse crate::spec::Corners;\nuse crate::tokens::Shape;\nuse egui::Color32;\n",
    );

    let color: Vec<&str> = roles.color.iter().map(String::as_str).collect();
    enum_item(
        &mut out,
        "Color scheme role (`ColorSchemeKeyTokens`).",
        "ColorRole",
        &tokens(&color, "ColorSchemeKeyTokens"),
    );
    out.push_str("\nimpl ColorRole {\n");
    all_const(&mut out, &declaration_order("ColorSchemeKeyTokens"), &color);
    let arms: Vec<_> = color.iter().map(|v| (*v, format!("scheme.{}", snake(v)))).collect();
    match_fn(
        &mut out,
        Some("Color of this role in `scheme`."),
        "pub const fn resolve(self, scheme: &ColorScheme) -> Color32",
        &arms,
    );
    out.push_str("}\n");

    shape_enums(&mut out, roles);

    let typography: Vec<&str> = roles.typography.iter().map(String::as_str).collect();
    enum_item(
        &mut out,
        "Type scale role (`TypographyKeyTokens`).",
        "TypeScale",
        &tokens(&typography, "TypographyKeyTokens"),
    );
    out.push_str("\nimpl TypeScale {\n");
    all_const(&mut out, &declaration_order("TypographyKeyTokens"), &typography);
    out.push_str("}\n");

    let levels: Vec<String> = (0..roles.elevation.len()).map(|i| format!("Level{i}")).collect();
    let levels: Vec<&str> = levels.iter().map(String::as_str).collect();
    enum_item(
        &mut out,
        "Elevation level (`ElevationTokens`).",
        "ElevationLevel",
        &tokens(&levels, "ElevationTokens"),
    );
    out.push_str("\nimpl ElevationLevel {\n");
    all_const(&mut out, "Every variant, lowest first.", &levels);
    let arms: Vec<_> = levels
        .iter()
        .zip(&roles.elevation)
        .map(|(l, dp)| (*l, format!("{dp:?}")))
        .collect();
    match_fn(
        &mut out,
        Some("Elevation in dp."),
        "pub const fn dp(self) -> f32",
        &arms,
    );
    out.push_str("}\n");

    let families: Vec<&str> = roles.families.iter().map(String::as_str).collect();
    enum_item(
        &mut out,
        "Font family role (`TypefaceTokens`).",
        "Typeface",
        &tokens(&families, "TypefaceTokens"),
    );
    out.push_str("\nimpl Typeface {\n");
    all_const(&mut out, &declaration_order("TypefaceTokens"), &families);
    out.push_str("}\n");
    out
}

fn mod_rs(sha: &str, modules: &[String]) -> String {
    let mut out = format!(
        "// @generated by tools/md3_tokens, do not edit.\n\
         //! Compose Material3 token objects, one module each.\n\n\
         pub(crate) mod roles;\n\n\
         /// `{REPO}` commit of the token sources.\npub const SOURCE_COMMIT: &str = \"{sha}\";\n\n"
    );
    for m in modules {
        w!(out, "pub mod {m};\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::snake;

    #[test]
    fn snake_case_boundaries() {
        assert_eq!(snake("FilledTextField"), "filled_text_field");
        assert_eq!(snake("XSmallIconButton"), "x_small_icon_button");
        assert_eq!(snake("DurationShort1"), "duration_short1");
        assert_eq!(snake("Level0"), "level0");
    }
}

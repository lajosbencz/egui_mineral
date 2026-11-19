//! Token data extracted from the parsed Fluent UI sources.

use std::{collections::BTreeMap, fs, path::Path};

use crate::{
    Error,
    ts::{self, Const, Expr, Parser, Part, Tok, Token},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rgba {
    pub(crate) r: u8,
    pub(crate) g: u8,
    pub(crate) b: u8,
    pub(crate) a: u8,
}

pub(crate) const VARIANT_KEYS: [&str; 12] = [
    "shade50", "shade40", "shade30", "shade20", "shade10", "primary", "tint10", "tint20", "tint30", "tint40", "tint50",
    "tint60",
];

pub(crate) const BRAND_STEPS: usize = 16;

pub(crate) enum Swatch {
    Rgba(Rgba),
    Const(String),
    Brand(u16),
}

pub(crate) struct AliasToken {
    pub(crate) name: String,
    pub(crate) light: Swatch,
    pub(crate) dark: Swatch,
    pub(crate) high_contrast: Swatch,
}

#[derive(Default)]
pub(crate) struct Palette {
    pub(crate) scalars: Vec<(String, Rgba)>,
    pub(crate) numeric: Vec<(String, Vec<(String, Rgba)>)>,
    pub(crate) variants: Vec<(String, [Rgba; 12])>,
}

pub(crate) struct Layer {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) blur: f32,
}

pub(crate) struct Model {
    pub(crate) palette: Palette,
    pub(crate) brand_web: [Rgba; BRAND_STEPS],
    pub(crate) radii: Vec<(String, f32)>,
    pub(crate) strokes: Vec<(String, f32)>,
    pub(crate) spacings: Vec<(String, f32)>,
    pub(crate) font_sizes: Vec<(String, f32)>,
    pub(crate) line_heights: Vec<(String, f32)>,
    pub(crate) font_weights: Vec<(String, u16)>,
    pub(crate) font_families: Vec<(String, String)>,
    pub(crate) durations: Vec<(String, u32)>,
    pub(crate) curves: Vec<(String, [f32; 4])>,
    pub(crate) shadows: Vec<(String, [Layer; 2])>,
    pub(crate) tokens: Vec<AliasToken>,
}

struct File {
    tokens: Vec<Token>,
    consts: Vec<Const>,
}

fn load(root: &Path, rel: &str) -> Result<File, Error> {
    read(root, rel, true)
}

// Palette alias and shadow files are loop/function scaffolding, scanned by token pattern instead of parsed.
fn lex_only(root: &Path, rel: &str) -> Result<File, Error> {
    read(root, rel, false)
}

fn read(root: &Path, rel: &str, parse: bool) -> Result<File, Error> {
    let path = root.join(rel);
    let src = fs::read_to_string(&path).map_err(|e| Error::new(format!("{}: {e}", path.display())))?;
    let at = |e: ts::ParseError| Error::new(format!("{}:{}: {}", path.display(), e.line, e.message));
    let tokens = ts::lex(&src).map_err(at)?;
    let consts = if parse {
        ts::module(&tokens).map_err(at)?
    } else {
        Vec::new()
    };
    Ok(File { tokens, consts })
}

impl File {
    fn get(&self, name: &str) -> Result<&Expr, Error> {
        self.consts
            .iter()
            .find(|c| c.name == name)
            .map(|c| &c.value)
            .ok_or_else(|| Error::new(format!("missing const {name}")))
    }

    fn object(&self, name: &str) -> Result<&[(String, Expr)], Error> {
        match self.get(name)? {
            Expr::Object(entries) => Ok(entries),
            other => Err(Error::new(format!("{name}: expected object, found {other:?}"))),
        }
    }
}

fn string(e: &Expr) -> Result<&str, Error> {
    match e {
        Expr::Str(s) => Ok(s),
        other => Err(Error::new(format!("expected string, found {other:?}"))),
    }
}

fn number<T: std::str::FromStr>(text: &str) -> Result<T, Error> {
    text.parse().map_err(|_| Error::new(format!("bad number {text:?}")))
}

fn px(text: &str) -> Result<f32, Error> {
    if text == "0" {
        return Ok(0.0);
    }
    number(
        text.strip_suffix("px")
            .ok_or_else(|| Error::new(format!("not a px length: {text:?}")))?,
    )
}

fn camel_group<T>(
    entries: &[(String, Expr)],
    prefix: &str,
    value: impl Fn(&Expr) -> Result<T, Error>,
) -> Result<Vec<(String, T)>, Error> {
    entries
        .iter()
        .map(|(k, v)| {
            if !k.starts_with(prefix) {
                return Err(Error::new(format!("{k}: expected prefix {prefix}")));
            }
            Ok((k.clone(), value(v)?))
        })
        .collect()
}

pub(crate) fn color(text: &str) -> Result<Rgba, Error> {
    let bad = || Error::new(format!("unsupported color {text:?}"));
    if text == "transparent" {
        return Ok(Rgba { r: 0, g: 0, b: 0, a: 0 });
    }
    if let Some(hex) = text.strip_prefix('#') {
        let byte = |i: usize| hex.get(i..i + 2).and_then(|h| u8::from_str_radix(h, 16).ok());
        return match (hex.len(), byte(0), byte(2), byte(4), byte(6)) {
            (6, Some(r), Some(g), Some(b), _) => Ok(Rgba { r, g, b, a: 255 }),
            (8, Some(r), Some(g), Some(b), Some(a)) => Ok(Rgba { r, g, b, a }),
            _ => Err(bad()),
        };
    }
    let inner = text
        .strip_prefix("rgba(")
        .and_then(|t| t.strip_suffix(')'))
        .ok_or_else(bad)?;
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    let [r, g, b, a] = parts.as_slice() else {
        return Err(bad());
    };
    let alpha: f32 = number(a)?;
    Ok(Rgba {
        r: number(r)?,
        g: number(g)?,
        b: number(b)?,
        a: alpha_byte(alpha),
    })
}

fn palette(file: &File) -> Result<Palette, Error> {
    let mut out = Palette::default();
    for c in &file.consts {
        match &c.value {
            Expr::Str(s) => out.scalars.push((c.name.clone(), color(s)?)),
            Expr::Object(entries) => {
                let mut table = Vec::new();
                for (k, v) in entries {
                    table.push((k.clone(), color(string(v)?)?));
                }
                if table.iter().all(|(k, _)| k.bytes().all(|b| b.is_ascii_digit())) {
                    out.numeric.push((c.name.clone(), table));
                } else {
                    let mut ramp = [Rgba { r: 0, g: 0, b: 0, a: 0 }; 12];
                    if table.len() != VARIANT_KEYS.len() {
                        return Err(Error::new(format!("{}: not a ColorVariants table", c.name)));
                    }
                    for (slot, key) in ramp.iter_mut().zip(VARIANT_KEYS) {
                        *slot = table
                            .iter()
                            .find(|(k, _)| k == key)
                            .map(|(_, v)| *v)
                            .ok_or_else(|| Error::new(format!("{}: missing {key}", c.name)))?;
                    }
                    out.variants.push((c.name.clone(), ramp));
                }
            }
            other => return Err(Error::new(format!("colors.ts {}: unsupported value {other:?}", c.name))),
        }
    }
    Ok(out)
}

impl Palette {
    fn scalar(&self, name: &str) -> Option<Rgba> {
        self.scalars.iter().find(|(n, _)| n == name).map(|(_, v)| *v)
    }

    fn numeric(&self, table: &str, key: &str) -> Option<Rgba> {
        let (_, t) = self.numeric.iter().find(|(n, _)| n == table)?;
        t.iter().find(|(k, _)| k == key).map(|(_, v)| *v)
    }

    fn variant(&self, table: &str, key: &str) -> Option<Rgba> {
        let (_, t) = self.variants.iter().find(|(n, _)| n == table)?;
        let i = VARIANT_KEYS.iter().position(|k| *k == key)?;
        t.get(i).copied()
    }
}

fn brand(file: &File) -> Result<[Rgba; BRAND_STEPS], Error> {
    let entries = file.object("brandWeb")?;
    let mut out = [Rgba { r: 0, g: 0, b: 0, a: 0 }; BRAND_STEPS];
    if entries.len() != BRAND_STEPS {
        return Err(Error::new("brandWeb: expected 16 steps"));
    }
    for (i, slot) in out.iter_mut().enumerate() {
        let key = ((i + 1) * 10).to_string();
        let (_, v) = entries
            .iter()
            .find(|(k, _)| *k == key)
            .ok_or_else(|| Error::new(format!("brandWeb: missing step {key}")))?;
        *slot = color(string(v)?)?;
    }
    Ok(out)
}

struct Shared {
    status: Vec<String>,
    persona: Vec<String>,
    mapping: Vec<(String, String)>,
    groups: Vec<(String, Vec<(String, String)>)>,
}

fn names(file: &File, name: &str) -> Result<Vec<String>, Error> {
    match file.get(name)? {
        Expr::Array(items) => items.iter().map(|e| string(e).map(str::to_owned)).collect(),
        other => Err(Error::new(format!("{name}: expected array, found {other:?}"))),
    }
}

fn shared(names_file: &File, mapping_file: &File, groups_file: &File) -> Result<Shared, Error> {
    let mut groups = Vec::new();
    for c in &groups_file.consts {
        let Expr::Object(entries) = &c.value else {
            return Err(Error::new(format!("colorPalette.ts {}: expected object", c.name)));
        };
        let mut members = Vec::new();
        for (k, v) in entries {
            let Expr::Ident(table) = v else {
                return Err(Error::new(format!(
                    "colorPalette.ts {}.{k}: expected reference",
                    c.name
                )));
            };
            members.push((k.clone(), table.clone()));
        }
        groups.push((c.name.clone(), members));
    }
    let mut mapping = Vec::new();
    for (k, v) in mapping_file.object("statusColorMapping")? {
        mapping.push((k.clone(), string(v)?.to_owned()));
    }
    Ok(Shared {
        status: names(names_file, "statusSharedColorNames")?,
        persona: names(names_file, "personaSharedColorNames")?,
        mapping,
        groups,
    })
}

fn capitalize(name: &str) -> String {
    let mut chars = name.chars();
    chars
        .next()
        .map_or_else(String::new, |c| c.to_uppercase().chain(chars).collect())
}

struct Eval<'a> {
    palette: &'a Palette,
    shared: &'a Shared,
}

impl Eval<'_> {
    fn key(&self, e: &Expr, var: Option<&str>) -> Result<String, Error> {
        match e {
            Expr::Ident(v) if v == "sharedColor" => var
                .map(str::to_owned)
                .ok_or_else(|| Error::new("sharedColor outside a loop template")),
            Expr::Member(inner, field) if matches!(&**inner, Expr::Ident(n) if n == "statusColorMapping") => self
                .shared
                .mapping
                .iter()
                .find(|(k, _)| k == field)
                .map(|(_, v)| v.clone())
                .ok_or_else(|| Error::new(format!("unknown status mapping {field}"))),
            other => Err(Error::new(format!("unsupported group key {other:?}"))),
        }
    }

    fn table(&self, e: &Expr, var: Option<&str>) -> Result<String, Error> {
        let (group, key) = match e {
            Expr::Index(g, k) => (g, self.key(k, var)?),
            Expr::Member(g, k) => (g, k.clone()),
            other => return Err(Error::new(format!("unsupported table reference {other:?}"))),
        };
        let Expr::Ident(group) = &**group else {
            return Err(Error::new("group reference must be an identifier"));
        };
        self.shared
            .groups
            .iter()
            .find(|(n, _)| n == group)
            .and_then(|(_, members)| members.iter().find(|(k, _)| *k == key))
            .map(|(_, table)| table.clone())
            .ok_or_else(|| Error::new(format!("unknown shared color {group}.{key}")))
    }

    fn rgba(&self, e: &Expr, var: Option<&str>) -> Result<Rgba, Error> {
        match e {
            Expr::Ident(n) => self
                .palette
                .scalar(n)
                .ok_or_else(|| Error::new(format!("unknown color {n}"))),
            Expr::Member(inner, field) => {
                let table = self.table(inner, var)?;
                self.palette
                    .variant(&table, field)
                    .ok_or_else(|| Error::new(format!("unknown variant {table}.{field}")))
            }
            other => Err(Error::new(format!("unsupported palette expression {other:?}"))),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Section {
    Status,
    Persona,
    Mapped,
}

impl Section {
    fn of(name: &str) -> Option<Self> {
        match name {
            "statusColorPaletteTokens" => Some(Self::Status),
            "personaColorPaletteTokens" => Some(Self::Persona),
            "colorStatusTokens" => Some(Self::Mapped),
            _ => None,
        }
    }
}

// Expands the `reduce` loop templates and one-off patches of the `*ColorPalette.ts` files.
fn palette_tokens(file: &File, eval: &Eval) -> Result<Vec<(String, Swatch)>, Error> {
    let toks = &file.tokens;
    let mut current = None;
    let mut templates: Vec<(Section, String, String, Expr)> = Vec::new();
    let mut patches: Vec<(Section, String, Expr)> = Vec::new();
    let at = |line: usize, e: ts::ParseError| Error::new(format!("line {line}: {}", e.message));
    let mut i = 0;
    while let Some(t) = toks.get(i) {
        let next = |n: usize| toks.get(i + n).map(|t| &t.tok);
        match (&t.tok, next(1), next(2), next(3)) {
            (Tok::Ident(kw), Some(Tok::Ident(name)), _, _) if kw == "const" => {
                if let Some(s) = Section::of(name) {
                    current = Some(s);
                }
            }
            (Tok::Ident(obj), Some(Tok::Punct('.')), Some(Tok::Ident(field)), Some(Tok::Punct('='))) => {
                let section = Section::of(obj)
                    .ok_or_else(|| Error::new(format!("line {}: assignment to unknown object {obj}", t.line)))?;
                let mut p = Parser::new(toks, i + 4);
                let expr = p.expr().map_err(|e| at(t.line, e))?;
                patches.push((section, field.clone(), expr));
                i = p.pos;
                continue;
            }
            (Tok::Punct('['), Some(Tok::Str(parts)), Some(Tok::Punct(']')), Some(Tok::Punct(':'))) => {
                let [Part::Text(prefix), Part::Var(var), Part::Text(suffix)] = parts.as_slice() else {
                    return Err(Error::new(format!("line {}: unsupported template key", t.line)));
                };
                if var != "color" {
                    return Err(Error::new(format!(
                        "line {}: unexpected template variable {var}",
                        t.line
                    )));
                }
                let section =
                    current.ok_or_else(|| Error::new(format!("line {}: template outside a section", t.line)))?;
                let mut p = Parser::new(toks, i + 4);
                let expr = p.expr().map_err(|e| at(t.line, e))?;
                templates.push((section, prefix.clone(), suffix.clone(), expr));
                i = p.pos;
                continue;
            }
            _ => {}
        }
        i += 1;
    }

    let mut out = Vec::new();
    for section in [Section::Status, Section::Persona, Section::Mapped] {
        let iterations: Vec<(String, String)> = match section {
            Section::Status => eval.shared.status.iter().map(|n| (n.clone(), n.clone())).collect(),
            Section::Persona => eval.shared.persona.iter().map(|n| (n.clone(), n.clone())).collect(),
            Section::Mapped => eval.shared.mapping.clone(),
        };
        let mut group: Vec<(String, Swatch)> = Vec::new();
        for (name, var) in &iterations {
            for (_, prefix, suffix, expr) in templates.iter().filter(|t| t.0 == section) {
                let token = format!("{prefix}{}{suffix}", capitalize(name));
                group.push((token, Swatch::Rgba(eval.rgba(expr, Some(var))?)));
            }
        }
        for (_, field, expr) in patches.iter().filter(|p| p.0 == section) {
            let value = Swatch::Rgba(eval.rgba(expr, None)?);
            match group.iter_mut().find(|(n, _)| n == field) {
                Some(slot) => slot.1 = value,
                None => group.push((field.clone(), value)),
            }
        }
        if group.is_empty() {
            return Err(Error::new("palette section without entries"));
        }
        out.extend(group);
    }
    Ok(out)
}

fn swatch(e: &Expr, palette: &Palette) -> Result<Swatch, Error> {
    match e {
        Expr::Str(s) => Ok(Swatch::Rgba(color(s)?)),
        Expr::Ident(n) if palette.scalar(n).is_some() => Ok(Swatch::Const(crate::emit::const_name(n))),
        Expr::Index(table, key) => {
            let (Expr::Ident(table), Expr::Num(key)) = (&**table, &**key) else {
                return Err(Error::new(format!("unsupported index expression {e:?}")));
            };
            if table == "brand" {
                let step: u16 = number(key)?;
                return if step.is_multiple_of(10) && (10..=160).contains(&step) {
                    Ok(Swatch::Brand(step))
                } else {
                    Err(Error::new(format!("brand step {step} out of range")))
                };
            }
            palette
                .numeric(table, key)
                .map(|_| Swatch::Const(format!("{}_{key}", crate::emit::const_name(table))))
                .ok_or_else(|| Error::new(format!("unknown color {table}[{key}]")))
        }
        other => Err(Error::new(format!("unsupported alias expression {other:?}"))),
    }
}

fn theme(root: &Path, stem: &str, palette: &Palette, eval: &Eval) -> Result<Vec<(String, Swatch)>, Error> {
    let main = load(root, &format!("alias/{stem}Color.ts"))?;
    let mut out = Vec::new();
    for (name, e) in main.object("generateColorTokens")? {
        out.push((
            name.clone(),
            swatch(e, palette).map_err(|e| Error::new(format!("{stem}Color.ts {name}: {e:?}")))?,
        ));
    }
    out.extend(palette_tokens(
        &lex_only(root, &format!("alias/{stem}ColorPalette.ts"))?,
        eval,
    )?);
    Ok(out)
}

fn spacings(file: &File) -> Result<Vec<(String, f32)>, Error> {
    let base = file.object("spacings")?;
    let mut out = Vec::new();
    for group in ["horizontalSpacings", "verticalSpacings"] {
        for (name, e) in file.object(group)? {
            let Expr::Member(obj, key) = e else {
                return Err(Error::new(format!("{name}: expected spacings reference")));
            };
            if !matches!(&**obj, Expr::Ident(n) if n == "spacings") {
                return Err(Error::new(format!("{name}: expected spacings reference")));
            }
            let (_, v) = base
                .iter()
                .find(|(k, _)| k == key)
                .ok_or_else(|| Error::new(format!("{name}: unknown spacing {key}")))?;
            out.push((name.clone(), px(string(v)?)?));
        }
    }
    Ok(out)
}

fn curve(text: &str) -> Result<[f32; 4], Error> {
    let inner = text
        .strip_prefix("cubic-bezier(")
        .and_then(|t| t.strip_suffix(')'))
        .ok_or_else(|| Error::new(format!("unsupported curve {text:?}")))?;
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    let [a, b, c, d] = parts.as_slice() else {
        return Err(Error::new(format!("unsupported curve {text:?}")));
    };
    Ok([number(a)?, number(b)?, number(c)?, number(d)?])
}

fn shadows(file: &File) -> Result<Vec<(String, [Layer; 2])>, Error> {
    let toks = &file.tokens;
    let mut out = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        let (Tok::Punct('['), Some(Tok::Str(key)), Some(Tok::Punct(']')), Some(Tok::Punct(':')), Some(Tok::Str(value))) = (
            &t.tok,
            toks.get(i + 1).map(|t| &t.tok),
            toks.get(i + 2).map(|t| &t.tok),
            toks.get(i + 3).map(|t| &t.tok),
            toks.get(i + 4).map(|t| &t.tok),
        ) else {
            continue;
        };
        let [Part::Text(name), Part::Var(suffix)] = key.as_slice() else {
            return Err(Error::new(format!("shadows.ts:{}: unsupported key", t.line)));
        };
        if suffix != "tokenSuffix" {
            return Err(Error::new(format!(
                "shadows.ts:{}: unexpected variable {suffix}",
                t.line
            )));
        }
        let [
            Part::Text(ambient),
            Part::Var(a_var),
            Part::Text(key_text),
            Part::Var(k_var),
        ] = value.as_slice()
        else {
            return Err(Error::new(format!("shadows.ts:{}: unsupported shadow value", t.line)));
        };
        if a_var != "ambientColor" || k_var != "keyColor" {
            return Err(Error::new(format!("shadows.ts:{}: unexpected shadow colors", t.line)));
        }
        let layer = |text: &str| -> Result<Layer, Error> {
            let fields: Vec<&str> = text.split_whitespace().collect();
            let [x, y, blur] = fields.as_slice() else {
                return Err(Error::new(format!("shadows.ts:{}: unsupported layer {text:?}", t.line)));
            };
            Ok(Layer {
                x: px(x)?,
                y: px(y)?,
                blur: px(blur)?,
            })
        };
        let key_layer = key_text
            .strip_prefix(", ")
            .ok_or_else(|| Error::new(format!("shadows.ts:{}: unsupported layer separator", t.line)))?;
        out.push((name.clone(), [layer(ambient)?, layer(key_layer)?]));
    }
    if out.len() != 6 {
        return Err(Error::new(format!(
            "shadows.ts: expected 6 shadow levels, found {}",
            out.len()
        )));
    }
    Ok(out)
}

fn check_shadow_colors(root: &Path) -> Result<(), Error> {
    for stem in ["Light", "Dark", "HighContrast"] {
        let path = root.join(format!("utils/create{stem}Theme.ts"));
        let src = fs::read_to_string(&path).map_err(|e| Error::new(format!("{}: {e}", path.display())))?;
        for needle in [
            "createShadowTokens(colorTokens.colorNeutralShadowAmbient, colorTokens.colorNeutralShadowKey)",
            "createShadowTokens(colorTokens.colorBrandShadowAmbient, colorTokens.colorBrandShadowKey, 'Brand')",
        ] {
            if !src.contains(needle) {
                return Err(Error::new(format!("{}: shadow color wiring changed", path.display())));
            }
        }
    }
    Ok(())
}

fn reconcile(
    light: Vec<(String, Swatch)>,
    dark: Vec<(String, Swatch)>,
    hc: Vec<(String, Swatch)>,
) -> Result<Vec<AliasToken>, Error> {
    let mut dark: BTreeMap<String, Swatch> = dark.into_iter().collect();
    let mut hc: BTreeMap<String, Swatch> = hc.into_iter().collect();
    let mut out = Vec::new();
    for (name, light) in light {
        let missing = |theme: &str| Error::new(format!("{name}: missing from {theme} theme"));
        let dark = dark.remove(&name).ok_or_else(|| missing("dark"))?;
        let high_contrast = hc.remove(&name).ok_or_else(|| missing("high contrast"))?;
        if matches!(high_contrast, Swatch::Brand(_)) {
            return Err(Error::new(format!(
                "{name}: high contrast theme references the brand ramp"
            )));
        }
        out.push(AliasToken {
            name,
            light,
            dark,
            high_contrast,
        });
    }
    if let Some(extra) = dark.keys().chain(hc.keys()).next() {
        return Err(Error::new(format!("{extra}: not in the light theme")));
    }
    Ok(out)
}

pub(crate) fn extract(root: &Path) -> Result<Model, Error> {
    let colors = load(root, "global/colors.ts")?;
    let palette = palette(&colors)?;
    let shared = shared(
        &load(root, "sharedColorNames.ts")?,
        &load(root, "statusColorMapping.ts")?,
        &load(root, "global/colorPalette.ts")?,
    )?;
    let eval = Eval {
        palette: &palette,
        shared: &shared,
    };
    let light = theme(root, "light", &palette, &eval)?;
    let dark = theme(root, "dark", &palette, &eval)?;
    let hc = theme(root, "highContrast", &palette, &eval)?;
    let tokens = reconcile(light, dark, hc)?;
    check_shadow_colors(root)?;

    let fonts = load(root, "global/fonts.ts")?;
    let ms = |e: &Expr| -> Result<u32, Error> {
        let text = string(e)?;
        number(
            text.strip_suffix("ms")
                .ok_or_else(|| Error::new(format!("not a ms duration: {text:?}")))?,
        )
    };
    let weight = |e: &Expr| match e {
        Expr::Num(n) => number(n),
        other => Err(Error::new(format!("expected number, found {other:?}"))),
    };
    let brand_web = brand(&load(root, "global/brandColors.ts")?)?;
    Ok(Model {
        palette,
        brand_web,
        radii: camel_group(
            load(root, "global/borderRadius.ts")?.object("borderRadius")?,
            "borderRadius",
            |e| px(string(e)?),
        )?,
        strokes: camel_group(
            load(root, "global/strokeWidths.ts")?.object("strokeWidths")?,
            "strokeWidth",
            |e| px(string(e)?),
        )?,
        spacings: spacings(&load(root, "global/spacings.ts")?)?,
        font_sizes: camel_group(fonts.object("fontSizes")?, "fontSize", |e| px(string(e)?))?,
        line_heights: camel_group(fonts.object("lineHeights")?, "lineHeight", |e| px(string(e)?))?,
        font_weights: camel_group(fonts.object("fontWeights")?, "fontWeight", weight)?,
        font_families: camel_group(fonts.object("fontFamilies")?, "fontFamily", |e| {
            Ok(string(e)?.to_owned())
        })?,
        durations: camel_group(load(root, "global/durations.ts")?.object("durations")?, "duration", ms)?,
        curves: camel_group(load(root, "global/curves.ts")?.object("curves")?, "curve", |e| {
            curve(string(e)?)
        })?,
        shadows: shadows(&lex_only(root, "utils/shadows.ts")?)?,
        tokens,
    })
}

#[expect(clippy::cast_sign_loss, reason = "clamped to 0..=255")]
fn alpha_byte(alpha: f32) -> u8 {
    (alpha * 255.0).round().clamp(0.0, 255.0) as u8
}

//! Parser for the machine-generated Compose token objects. Accepts only the forms the token
//! generator emits; anything else is an error with its source line.

use std::fmt;

#[derive(Debug, PartialEq)]
pub(crate) struct Error {
    pub(crate) line: usize,
    pub(crate) message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.line, self.message)
    }
}

fn err<T>(line: usize, message: impl Into<String>) -> Result<T, Error> {
    Err(Error {
        line,
        message: message.into(),
    })
}

#[derive(Debug)]
pub(crate) struct Object {
    pub(crate) name: String,
    pub(crate) version: Option<String>,
    pub(crate) decls: Vec<Decl>,
}

#[derive(Debug)]
pub(crate) struct Decl {
    pub(crate) line: usize,
    pub(crate) name: String,
    pub(crate) value: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Value {
    Dp(f32),
    Sp(f32),
    Float(f32),
    Double(f64),
    /// `ColorToken(id)` and the other key-token constructors.
    Key(u32),
    Ref {
        object: String,
        name: String,
    },
    RoundedAll(f32),
    /// `[top_start, top_end, bottom_end, bottom_start]` in dp.
    RoundedCorners([f32; 4]),
    CornerSize(f32),
    Circle,
    Rectangle,
    Easing([f32; 4]),
    FontFamily(String),
    FontWeight(String),
}

const KEY_CONSTRUCTORS: [&str; 4] = ["ColorToken", "ShapeToken", "TypographyToken", "MotionSchemeToken"];

pub(crate) fn parse(src: &str) -> Result<Object, Error> {
    let version = src.lines().find_map(|l| {
        let l = l.trim();
        l.strip_prefix("// VERSION:")
            .or_else(|| l.strip_prefix("// Version:"))
            .map(|v| v.trim().to_owned())
    });
    let lines = strip_comments(src)?;
    let mut object: Option<(String, Vec<(usize, String)>)> = None;
    let mut closed = false;
    for (idx, line) in lines.iter().enumerate() {
        let no = idx + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match &mut object {
            Some((_, stmts)) if !closed => {
                if line.starts_with('}') {
                    if trimmed != "}" {
                        return err(no, format!("unexpected text after object end: {trimmed}"));
                    }
                    closed = true;
                } else if starts_decl(trimmed) {
                    stmts.push((no, trimmed.to_owned()));
                } else if let Some((_, stmt)) = stmts.last_mut() {
                    stmt.push(' ');
                    stmt.push_str(trimmed);
                } else {
                    return err(no, format!("expected a declaration, got: {trimmed}"));
                }
            }
            Some(_) => return err(no, format!("unexpected text after object: {trimmed}")),
            None => {
                if let Some(name) = trimmed
                    .strip_prefix("internal object ")
                    .and_then(|r| r.strip_suffix(" {"))
                {
                    if !is_ident(name) {
                        return err(no, format!("invalid object name: {name}"));
                    }
                    object = Some((name.to_owned(), Vec::new()));
                } else if !(trimmed.starts_with("package ") || trimmed.starts_with("import ") || is_key_class(trimmed))
                {
                    return err(no, format!("unexpected top-level text: {trimmed}"));
                }
            }
        }
    }
    let Some((name, stmts)) = object else {
        return err(lines.len(), "no `internal object` found");
    };
    if !closed {
        return err(lines.len(), format!("object {name} is not closed"));
    }
    let decls = stmts
        .into_iter()
        .map(|(line, stmt)| parse_decl(line, &stmt).map_err(|message| Error { line, message }))
        .collect::<Result<_, _>>()?;
    Ok(Object { name, version, decls })
}

fn strip_comments(src: &str) -> Result<Vec<String>, Error> {
    let mut in_block = false;
    let mut out = Vec::new();
    for (idx, line) in src.lines().enumerate() {
        let mut rest = line;
        let mut kept = String::new();
        loop {
            if in_block {
                match rest.split_once("*/") {
                    Some((_, after)) => {
                        rest = after;
                        in_block = false;
                    }
                    None => break,
                }
            } else {
                let block = rest.find("/*");
                match (block, rest.find("//")) {
                    (Some(b), l) if l.is_none_or(|l| b < l) => {
                        let (head, tail) = rest.split_at(b);
                        kept.push_str(head);
                        rest = tail.split_at(2).1;
                        in_block = true;
                    }
                    (_, Some(l)) => {
                        kept.push_str(rest.split_at(l).0);
                        break;
                    }
                    _ => {
                        kept.push_str(rest);
                        break;
                    }
                }
            }
        }
        if kept.contains('"') {
            return err(idx + 1, "string literals are not supported");
        }
        out.push(kept);
    }
    if in_block {
        return err(out.len(), "unterminated block comment");
    }
    Ok(out)
}

fn is_key_class(line: &str) -> bool {
    line.strip_prefix("@JvmInline internal value class ")
        .and_then(|r| r.strip_suffix("(val id: Int)"))
        .is_some_and(|name| KEY_CONSTRUCTORS.contains(&name))
}

fn starts_decl(line: &str) -> bool {
    ["val ", "inline val ", "const val "]
        .iter()
        .any(|p| line.starts_with(p))
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic()) && chars.all(|c| c.is_ascii_alphanumeric())
}

fn parse_decl(line: usize, stmt: &str) -> Result<Decl, String> {
    let rest = stmt
        .strip_prefix("const val ")
        .or_else(|| stmt.strip_prefix("inline val "))
        .or_else(|| stmt.strip_prefix("val "))
        .ok_or_else(|| format!("not a declaration: {stmt}"))?;
    let name_end = rest.find(|c: char| !c.is_ascii_alphanumeric()).unwrap_or(rest.len());
    let (name, rest) = rest.split_at(name_end);
    if !is_ident(name) {
        return Err(format!("invalid name in: {stmt}"));
    }
    let rest = rest.trim_start();
    let (ty, rest) = match rest.strip_prefix(':') {
        Some(r) => {
            let r = r.trim_start();
            let end = r.find([' ', '=']).ok_or_else(|| format!("missing value in: {stmt}"))?;
            let (path, r) = r.split_at(end);
            (Some(path.rsplit('.').next().unwrap_or(path)), r.trim_start())
        }
        None => (None, rest),
    };
    let rhs = rest
        .strip_prefix("get() =")
        .or_else(|| rest.strip_prefix('='))
        .ok_or_else(|| format!("expected `=` or `get() =` in: {stmt}"))?
        .trim();
    let value = parse_value(rhs)?;
    if let Some(ty) = ty {
        check_type(ty, &value).map_err(|e| format!("{name}: {e}"))?;
    }
    Ok(Decl {
        line,
        name: name.to_owned(),
        value,
    })
}

fn check_type(ty: &str, value: &Value) -> Result<(), String> {
    let ok = match (ty, value) {
        ("Dp", Value::Dp(_))
        | ("TextUnit", Value::Sp(_))
        | ("CubicBezierEasing", Value::Easing(_))
        | ("FontFamily", Value::FontFamily(_))
        | ("FontWeight", Value::FontWeight(_)) => true,
        ("Dp", Value::Ref { object, .. }) => object == "ElevationTokens",
        ("ColorToken", Value::Ref { object, .. }) => object == "ColorSchemeKeyTokens",
        ("ShapeToken", Value::Ref { object, .. }) => object == "ShapeKeyTokens",
        ("TypographyToken", Value::Ref { object, .. }) => object == "TypographyKeyTokens",
        ("Color", Value::Ref { object, .. }) => object == "PaletteTokens",
        ("FontFamily" | "FontWeight", Value::Ref { object, .. }) => object == "TypefaceTokens",
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(format!("value {value:?} does not match declared type {ty}"))
    }
}

pub(crate) fn parse_value(rhs: &str) -> Result<Value, String> {
    if let Some(v) = parse_number(rhs)? {
        return Ok(v);
    }
    match rhs {
        "CircleShape" => return Ok(Value::Circle),
        "RectangleShape" => return Ok(Value::Rectangle),
        _ => {}
    }
    if let Some(open) = rhs.find('(') {
        let (callee, args) = rhs.split_at(open);
        let args = args
            .strip_prefix('(')
            .and_then(|a| a.strip_suffix(')'))
            .ok_or_else(|| format!("unbalanced call: {rhs}"))?;
        let args: Vec<&str> = args.split(',').map(str::trim).filter(|a| !a.is_empty()).collect();
        return parse_call(callee, &args).map_err(|e| format!("{e} in: {rhs}"));
    }
    if let Some((object, name)) = rhs.split_once('.')
        && is_ident(object)
        && is_ident(name)
    {
        return Ok(match object {
            "FontFamily" => Value::FontFamily(name.to_owned()),
            "FontWeight" => Value::FontWeight(name.to_owned()),
            _ if object.ends_with("Tokens") => Value::Ref {
                object: object.to_owned(),
                name: name.to_owned(),
            },
            _ => return Err(format!("unsupported reference: {rhs}")),
        });
    }
    Err(format!("unsupported value: {rhs}"))
}

fn parse_call(callee: &str, args: &[&str]) -> Result<Value, String> {
    let dp = |a: &str| match parse_number(a)? {
        Some(Value::Dp(v)) => Ok(v),
        _ => Err(format!("expected a dp literal, got {a}")),
    };
    let float = |a: &str| match parse_number(a)? {
        Some(Value::Float(v)) => Ok(v),
        _ => Err(format!("expected a float literal, got {a}")),
    };
    match (callee, args) {
        (k, [id]) if KEY_CONSTRUCTORS.contains(&k) => id
            .parse()
            .map(Value::Key)
            .map_err(|e| format!("invalid key id {id}: {e}")),
        ("CornerSize", [a]) => dp(a).map(Value::CornerSize),
        ("RoundedCornerShape", [a]) if !a.contains('=') => dp(a).map(Value::RoundedAll),
        ("RoundedCornerShape", named) => {
            let mut corners = [None; 4];
            for arg in named {
                let (key, val) = arg
                    .split_once('=')
                    .ok_or_else(|| format!("expected a named argument, got {arg}"))?;
                let slot = ["topStart", "topEnd", "bottomEnd", "bottomStart"]
                    .iter()
                    .position(|k| *k == key.trim())
                    .ok_or_else(|| format!("unknown corner {key}"))?;
                if corners[slot].replace(dp(val.trim())?).is_some() {
                    return Err(format!("duplicate corner {key}"));
                }
            }
            let [Some(a), Some(b), Some(c), Some(d)] = corners else {
                return Err("all four corners are required".to_owned());
            };
            Ok(Value::RoundedCorners([a, b, c, d]))
        }
        ("CubicBezierEasing", [a, b, c, d]) => Ok(Value::Easing([float(a)?, float(b)?, float(c)?, float(d)?])),
        _ => Err(format!("unsupported call {callee} with {} arguments", args.len())),
    }
}

fn parse_number(s: &str) -> Result<Option<Value>, String> {
    let bytes = s.as_bytes();
    let mut end = usize::from(bytes.first() == Some(&b'-'));
    let digits_start = end;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    if end == digits_start {
        return Ok(None);
    }
    let mut is_int = true;
    if bytes.get(end) == Some(&b'.') && bytes.get(end + 1).is_some_and(u8::is_ascii_digit) {
        is_int = false;
        end += 1;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
    }
    let (num, suffix) = s.split_at(end);
    let f32_of = |n: &str| n.parse::<f32>().map_err(|e| format!("invalid number {n}: {e}"));
    match suffix {
        ".dp" => f32_of(num).map(|v| Some(Value::Dp(v))),
        ".sp" => f32_of(num).map(|v| Some(Value::Sp(v))),
        "f" => f32_of(num).map(|v| Some(Value::Float(v))),
        "" if !is_int => num
            .parse::<f64>()
            .map(|v| Some(Value::Double(v)))
            .map_err(|e| format!("invalid number {num}: {e}")),
        _ => Err(format!("unsupported number literal: {s}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_generated_forms() {
        let src = r#"/*
 * header "quoted"
 */
// VERSION: v0_1
package p

import a.b

internal object FooTokens {
    inline val Height: androidx.compose.ui.unit.Dp
        get() = 56.0.dp

    const val Opacity = 0.38f
    val Shape =
        RoundedCornerShape(
            topStart = 4.0.dp,
            topEnd = 4.0.dp,
            bottomEnd = 0.0.dp,
            bottomStart = 0.0.dp,
        )
    inline val Color: ColorToken
        get() = ColorSchemeKeyTokens.Primary
}
"#;
        let obj = parse(src).expect("valid object");
        assert_eq!(obj.name, "FooTokens");
        assert_eq!(obj.version.as_deref(), Some("v0_1"));
        let values: Vec<_> = obj.decls.iter().map(|d| (d.line, d.value.clone())).collect();
        assert_eq!(
            values,
            [
                (10, Value::Dp(56.0)),
                (13, Value::Float(0.38)),
                (14, Value::RoundedCorners([4.0, 4.0, 0.0, 0.0])),
                (
                    21,
                    Value::Ref {
                        object: "ColorSchemeKeyTokens".into(),
                        name: "Primary".into()
                    }
                ),
            ]
        );
    }

    #[test]
    fn unknown_forms_fail_with_line() {
        let src = "internal object FooTokens {\n    val Size = DpSize(1.0.dp, 2.0.dp)\n}\n";
        let e = parse(src).expect_err("DpSize is unsupported");
        assert_eq!(e.line, 2, "{e}");
        let e = parse("internal object FooTokens {\n    inline val X: ColorToken\n        get() = 1.0.dp\n}\n")
            .expect_err("type mismatch");
        assert_eq!(e.line, 2, "{e}");
    }
}

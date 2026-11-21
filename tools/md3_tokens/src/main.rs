//! Generates `egui_mineral_material`'s `spec::tokens` modules from the vendored Compose Material3 token sources.
//!
//! `fetch.sh` fetches the sources at the commit in `PIN` into `vendor/<sha>/`; `generate` is offline and deterministic.

mod emit;
mod kotlin;

use std::{
    collections::BTreeMap,
    fmt, fs,
    path::{Path, PathBuf},
};

pub(crate) const REPO: &str = "androidx/androidx";
pub(crate) const SOURCE_DIR: &str =
    "compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens";

const SKIP: [(&str, &str); 5] = [
    (
        "ColorDarkTokens.kt",
        "baseline scheme; color.rs derives schemes with material-colors",
    ),
    (
        "ColorLightTokens.kt",
        "baseline scheme; color.rs derives schemes with material-colors",
    ),
    (
        "PaletteTokens.kt",
        "baseline palette; color.rs derives palettes with material-colors",
    ),
    ("MotionSchemeKeyTokens.kt", "no token object references it"),
    (
        "TypographyTokens.kt",
        "Compose TextStyle assembly of TypeScaleTokens, no values",
    ),
];

pub(crate) const CONSUMED: [&str; 6] = [
    "ColorSchemeKeyTokens",
    "ElevationTokens",
    "ShapeKeyTokens",
    "ShapeTokens",
    "TypefaceTokens",
    "TypographyKeyTokens",
];

pub(crate) struct Error(String);

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }

    fn io(path: &Path, e: &std::io::Error) -> Self {
        Self(format!("{}: {e}", path.display()))
    }
}

pub(crate) struct Source {
    pub(crate) file: String,
    pub(crate) copyright: String,
    pub(crate) object: kotlin::Object,
}

fn main() -> Result<(), Error> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [cmd] if cmd == "generate" => generate(),
        _ => Err(Error::new(
            "usage: md3_tokens generate (vendor sources with tools/md3_tokens/fetch.sh)",
        )),
    }
}

fn generate() -> Result<(), Error> {
    let tool = Path::new(env!("CARGO_MANIFEST_DIR"));
    let (sha, vendor) = vendor_dir(&tool.join("vendor"))?;
    let out = tool.join("../../crates/egui_mineral_material/src/spec/tokens");
    let sources = read_sources(&vendor)?;
    let files = emit::files(&sha, &sources)?;

    fs::create_dir_all(&out).map_err(|e| Error::io(&out, &e))?;
    for entry in fs::read_dir(&out).map_err(|e| Error::io(&out, &e))? {
        let path = entry.map_err(|e| Error::io(&out, &e))?.path();
        let stale = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_none_or(|n| !files.contains_key(n));
        if stale {
            fs::remove_file(&path).map_err(|e| Error::io(&path, &e))?;
        }
    }
    for (name, text) in &files {
        let path = out.join(name);
        if fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
            fs::write(&path, text).map_err(|e| Error::io(&path, &e))?;
        }
    }
    Ok(())
}

fn vendor_dir(vendor: &Path) -> Result<(String, PathBuf), Error> {
    let mut dirs = Vec::new();
    for entry in fs::read_dir(vendor).map_err(|e| Error::io(vendor, &e))? {
        dirs.push(entry.map_err(|e| Error::io(vendor, &e))?.path());
    }
    let [dir] = dirs.as_slice() else {
        return Err(Error::new(format!(
            "{}: expected exactly one vendored commit directory, found {}",
            vendor.display(),
            dirs.len()
        )));
    };
    let sha = dir
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| n.len() == 40 && n.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
        .ok_or_else(|| Error::new(format!("{}: not a commit sha directory", dir.display())))?;
    Ok((sha.to_owned(), dir.clone()))
}

fn read_sources(dir: &Path) -> Result<Vec<Source>, Error> {
    let mut files = BTreeMap::new();
    for entry in fs::read_dir(dir).map_err(|e| Error::io(dir, &e))? {
        let path = entry.map_err(|e| Error::io(dir, &e))?.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|n| n.ends_with("Tokens.kt"))
            .ok_or_else(|| Error::new(format!("{}: not a token source", path.display())))?
            .to_owned();
        files.insert(name, path);
    }
    for (skip, _) in SKIP {
        if !files.contains_key(skip) {
            return Err(Error::new(format!("skip list entry {skip} is not vendored")));
        }
    }
    let mut sources = Vec::new();
    for (file, path) in files {
        if SKIP.iter().any(|(s, _)| *s == file) {
            continue;
        }
        let src = fs::read_to_string(&path).map_err(|e| Error::io(&path, &e))?;
        let object =
            kotlin::parse(&src).map_err(|e| Error::new(format!("{}:{}: {}", path.display(), e.line, e.message)))?;
        let copyright = src
            .lines()
            .find_map(|l| l.trim_start_matches([' ', '*']).strip_prefix("Copyright "))
            .ok_or_else(|| Error::new(format!("{}: no copyright line", path.display())))?
            .trim()
            .to_owned();
        sources.push(Source {
            file,
            copyright,
            object,
        });
    }
    Ok(sources)
}

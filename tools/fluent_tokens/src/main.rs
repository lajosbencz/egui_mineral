//! Generates `egui_mineral_fluent`'s `spec::tokens` modules from the vendored Fluent UI React v9 token sources.
//!
//! Also generates its `icons` module from the vendored Fluent UI System Icons codepoint map.
//!
//! `fetch.sh` fetches the sources at the commits in `PIN` into `vendor/<sha>/` and `vendor-icons/<icons-sha>/`;
//! `generate` is offline and deterministic.

mod emit;
mod icons;
mod model;
mod ts;

use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

pub(crate) const REPO: &str = "microsoft/fluentui";
pub(crate) const SOURCE_DIR: &str = "packages/tokens/src";

const SKIP: [(&str, &str); 8] = [
    ("alias/teamsDarkColor.ts", "Teams theme, not part of the web themes"),
    ("alias/teamsFontFamilies.ts", "Teams theme, not part of the web themes"),
    ("global/index.ts", "re-exports"),
    (
        "global/typographyStyles.ts",
        "CSS-in-JS style assembly of font tokens, no values",
    ),
    ("types.ts", "type declarations"),
    (
        "utils/createLightTheme.ts",
        "theme assembly; checked for the shadow color wiring only",
    ),
    (
        "utils/createDarkTheme.ts",
        "theme assembly; checked for the shadow color wiring only",
    ),
    (
        "utils/createHighContrastTheme.ts",
        "theme assembly; checked for the shadow color wiring only",
    ),
];

const CONSUMED: [&str; 18] = [
    "alias/darkColor.ts",
    "alias/darkColorPalette.ts",
    "alias/highContrastColor.ts",
    "alias/highContrastColorPalette.ts",
    "alias/lightColor.ts",
    "alias/lightColorPalette.ts",
    "global/borderRadius.ts",
    "global/brandColors.ts",
    "global/colorPalette.ts",
    "global/colors.ts",
    "global/curves.ts",
    "global/durations.ts",
    "global/fonts.ts",
    "global/spacings.ts",
    "global/strokeWidths.ts",
    "sharedColorNames.ts",
    "statusColorMapping.ts",
    "utils/shadows.ts",
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

fn main() -> Result<(), Error> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [cmd] if cmd == "generate" => generate(),
        _ => Err(Error::new(
            "usage: fluent_tokens generate (vendor sources with tools/fluent_tokens/fetch.sh)",
        )),
    }
}

fn generate() -> Result<(), Error> {
    let tool = Path::new(env!("CARGO_MANIFEST_DIR"));
    let (sha, vendor) = vendor_dir(&tool.join("vendor"))?;
    let root = vendor.join(SOURCE_DIR);
    check_sources(&root)?;
    let copyright = copyright(&vendor.join("LICENSE"))?;
    let files = emit::files(&sha, &copyright, &model::extract(&root)?);

    let (icons_sha, icons_dir) = vendor_dir(&tool.join("vendor-icons"))?;
    let icons = icons::module(
        &icons_sha,
        &self::copyright(&icons_dir.join("LICENSE"))?,
        &icons_dir.join(format!("{}.json", icons::FONT)),
    )?;
    let icons_rs = tool.join("../../crates/egui_mineral_fluent/src/icons.rs");
    if fs::read_to_string(&icons_rs).ok().as_deref() != Some(icons.as_str()) {
        fs::write(&icons_rs, &icons).map_err(|e| Error::io(&icons_rs, &e))?;
    }

    let out = tool.join("../../crates/egui_mineral_fluent/src/spec/tokens");
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

fn copyright(license: &Path) -> Result<String, Error> {
    let text = fs::read_to_string(license).map_err(|e| Error::io(license, &e))?;
    text.lines()
        .find(|l| l.starts_with("Copyright "))
        .map(|l| l.trim().to_owned())
        .ok_or_else(|| Error::new(format!("{}: no copyright line", license.display())))
}

fn ts_files(dir: &Path, base: &Path, out: &mut Vec<String>) -> Result<(), Error> {
    for entry in fs::read_dir(dir).map_err(|e| Error::io(dir, &e))? {
        let path = entry.map_err(|e| Error::io(dir, &e))?.path();
        if path.is_dir() {
            ts_files(&path, base, out)?;
        } else {
            let rel = path
                .strip_prefix(base)
                .map_err(|e| Error::new(e.to_string()))?
                .to_string_lossy()
                .into_owned();
            out.push(rel);
        }
    }
    Ok(())
}

fn check_sources(root: &Path) -> Result<(), Error> {
    let mut vendored = Vec::new();
    ts_files(root, root, &mut vendored)?;
    for f in &vendored {
        if !CONSUMED.contains(&f.as_str()) && !SKIP.iter().any(|(s, _)| s == f) {
            return Err(Error::new(format!("{f}: vendored but neither consumed nor skipped")));
        }
    }
    for f in CONSUMED.iter().chain(SKIP.iter().map(|(s, _)| s)) {
        if !vendored.iter().any(|v| v == f) {
            return Err(Error::new(format!("{f}: listed but not vendored")));
        }
    }
    Ok(())
}

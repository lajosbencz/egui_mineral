//! Subsets the Material Symbols fonts to the `ICON_*` constants named by every package that depends on
//! `egui_material_icons`, the stylekit included. Writes both fonts to `OUT_DIR`.

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use allsorts::{
    Font,
    binary::read::ReadScope,
    font::MatchingPresentation,
    font_data::FontData,
    subset::{CmapTarget, SubsetProfile, subset},
};
use serde_json::Value;

type Res<T> = Result<T, Box<dyn Error>>;

const ICONS: &str = "egui_material_icons";
const FONTS: [&str; 2] = [
    "MaterialSymbolsRounded_Filled-Regular.ttf",
    "MaterialSymbolsRounded-Regular.ttf",
];
const SOURCE_KINDS: [&str; 5] = ["lib", "rlib", "bin", "proc-macro", "cdylib"];

fn main() -> Res<()> {
    let meta = metadata()?;
    let packages: HashMap<&str, &Value> = meta["packages"]
        .as_array()
        .ok_or("metadata: no packages")?
        .iter()
        .filter_map(|p| Some((p["id"].as_str()?, p)))
        .collect();
    let icons_id = packages
        .iter()
        .find(|(_, p)| p["name"] == ICONS)
        .map(|(id, _)| *id)
        .ok_or("metadata: no egui_material_icons")?;
    let mut sources = BTreeSet::new();
    for id in dependents(&meta, icons_id)? {
        let targets = packages
            .get(id)
            .and_then(|p| p["targets"].as_array())
            .ok_or("metadata: no targets")?;
        for t in targets {
            let kinds = t["kind"].as_array().ok_or("metadata: no target kind")?;
            if kinds
                .iter()
                .any(|k| k.as_str().is_some_and(|k| SOURCE_KINDS.contains(&k)))
                && let Some(dir) = t["src_path"].as_str().and_then(|p| Path::new(p).parent())
            {
                sources.insert(dir.to_path_buf());
            }
        }
    }
    let mut names = HashSet::new();
    for dir in &sources {
        println!("cargo::rerun-if-changed={}", dir.display());
        scan(dir, &mut names)?;
    }
    let lock = Path::new(meta["workspace_root"].as_str().ok_or("metadata: no workspace_root")?).join("Cargo.lock");
    println!("cargo::rerun-if-changed={}", lock.display());

    let icons_dir = packages[icons_id]["manifest_path"]
        .as_str()
        .and_then(|p| Path::new(p).parent())
        .ok_or("metadata: no egui_material_icons manifest")?;
    let icons_rs = fs::read_to_string(icons_dir.join("src/icons.rs"))?;
    let codepoints = codepoints(&icons_rs);
    let chars: BTreeSet<char> = names
        .iter()
        .filter_map(|n| codepoints.get(n.as_str()).copied())
        .collect();
    let out = PathBuf::from(env::var("OUT_DIR")?);
    for name in FONTS {
        fs::write(out.join(name), subset_font(&fs::read(icons_dir.join(name))?, &chars)?)?;
    }
    Ok(())
}

fn metadata() -> Res<Value> {
    let out = Command::new(env::var("CARGO")?)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--offline",
            "--filter-platform",
            &env::var("TARGET")?,
        ])
        .arg("--manifest-path")
        .arg(Path::new(&env::var("CARGO_MANIFEST_DIR")?).join("Cargo.toml"))
        .output()?;
    if !out.status.success() {
        return Err(format!("cargo metadata: {}", String::from_utf8_lossy(&out.stderr)).into());
    }
    Ok(serde_json::from_slice(&out.stdout)?)
}

/// Packages depending on `root` directly or transitively.
fn dependents<'a>(meta: &'a Value, root: &'a str) -> Res<HashSet<&'a str>> {
    let nodes = meta["resolve"]["nodes"].as_array().ok_or("metadata: no resolve")?;
    let edges: Vec<(&str, Vec<&str>)> = nodes
        .iter()
        .filter_map(|n| {
            let deps = n["deps"].as_array()?.iter().filter_map(|d| d["pkg"].as_str()).collect();
            Some((n["id"].as_str()?, deps))
        })
        .collect();
    let mut reached = HashSet::from([root]);
    loop {
        let before = reached.len();
        for (id, deps) in &edges {
            if deps.iter().any(|d| reached.contains(d)) {
                reached.insert(id);
            }
        }
        if reached.len() == before {
            reached.remove(root);
            return Ok(reached);
        }
    }
}

fn scan(dir: &Path, names: &mut HashSet<String>) -> Res<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            scan(&path, names)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            identifiers(&fs::read_to_string(&path)?, names);
        }
    }
    Ok(())
}

fn identifiers(src: &str, names: &mut HashSet<String>) {
    for token in src.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
        if token.starts_with("ICON_") {
            names.insert(token.to_owned());
        }
    }
}

/// `pub const ICON_X: crate::MaterialIcon = crate::MaterialIcon::new("\u{e951}");` lines of `icons.rs`.
fn codepoints(icons_rs: &str) -> HashMap<&str, char> {
    icons_rs
        .lines()
        .filter_map(|l| {
            let (name, rest) = l.strip_prefix("pub const ")?.split_once(':')?;
            let (_, hex) = rest.split_once("\\u{")?;
            let (hex, _) = hex.split_once('}')?;
            Some((name, char::from_u32(u32::from_str_radix(hex, 16).ok()?)?))
        })
        .collect()
}

fn subset_font(ttf: &[u8], chars: &BTreeSet<char>) -> Res<Vec<u8>> {
    let data = ReadScope::new(ttf).read::<FontData<'_>>()?;
    let mut font = Font::new(data.table_provider(0)?)?;
    // Glyph 0 (.notdef) is mandatory.
    let mut glyphs = BTreeSet::from([0]);
    for c in chars {
        glyphs.insert(font.lookup_glyph_index(*c, MatchingPresentation::NotRequired, None).0);
    }
    let glyphs: Vec<u16> = glyphs.into_iter().collect();
    Ok(subset(
        &font.font_table_provider,
        &glyphs,
        &SubsetProfile::Minimal,
        CmapTarget::Unicode,
    )?)
}

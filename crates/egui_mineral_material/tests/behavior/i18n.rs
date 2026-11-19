use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

fn files(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            files(&path, ext, out);
        } else if path.extension().is_some_and(|e| e == ext) {
            out.push(path);
        }
    }
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn message_ids(source: &str) -> impl Iterator<Item = String> + '_ {
    source.split('"').skip(1).step_by(2).filter_map(|lit| {
        let rest = lit.strip_prefix("mineral-")?;
        let kebab = !rest.is_empty()
            && rest
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        kebab.then(|| lit.to_owned())
    })
}

fn catalog_ids(ftl: &str) -> BTreeSet<String> {
    ftl.lines()
        .filter_map(|l| l.split_once('=').map(|(id, _)| id))
        .filter(|id| !id.starts_with([' ', '#', '-']) && !id.trim().is_empty())
        .map(|id| id.trim().to_owned())
        .collect()
}

fn check_catalogs(locales: &Path, sources: &[PathBuf]) {
    let mut rs = Vec::new();
    for dir in sources {
        files(dir, "rs", &mut rs);
    }
    let used: BTreeSet<String> = rs
        .iter()
        .flat_map(|f| message_ids(&read(f)).collect::<Vec<_>>())
        .collect();
    assert!(!used.is_empty(), "no message ids found");

    let reference = catalog_ids(&read(&locales.join("en-US.ftl")));
    let missing: Vec<_> = used.difference(&reference).collect();
    assert!(missing.is_empty(), "ids missing from en-US.ftl: {missing:?}");
    let unused: Vec<_> = reference.difference(&used).collect();
    assert!(unused.is_empty(), "en-US.ftl ids no code uses: {unused:?}");

    let mut ftls = Vec::new();
    files(locales, "ftl", &mut ftls);
    for ftl in ftls {
        let ids = catalog_ids(&read(&ftl));
        assert_eq!(ids, reference, "{} differs from en-US.ftl", ftl.display());
    }
}

#[test]
fn material_catalogs_match_the_ids_in_use() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    check_catalogs(
        &root.join("locales"),
        &[root.join("src"), root.join("../egui_mineral/src")],
    );
}

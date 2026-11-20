use std::{collections::BTreeSet, fs, path::Path};

fn catalog_ids(ftl: &str) -> BTreeSet<String> {
    ftl.lines()
        .filter_map(|l| l.split_once('=').map(|(id, _)| id))
        .filter(|id| !id.starts_with([' ', '#', '-']) && !id.trim().is_empty())
        .map(|id| id.trim().to_owned())
        .collect()
}

#[test]
fn translations_define_no_id_the_source_catalog_lacks() {
    let locales = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    let read = |path: &Path| fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let source = catalog_ids(&read(&locales.join("en-US.ftl")));
    assert!(!source.is_empty(), "en-US.ftl has messages");
    for entry in fs::read_dir(&locales).expect("locales") {
        let path = entry.expect("dir entry").path();
        let ids = catalog_ids(&read(&path));
        let stale: Vec<_> = ids.difference(&source).collect();
        assert!(stale.is_empty(), "{}: ids en-US.ftl lacks: {stale:?}", path.display());
    }
}

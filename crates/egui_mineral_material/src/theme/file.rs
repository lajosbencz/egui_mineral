use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use egui_mineral::runtime::Runtime;

use super::Tokens;
use crate::Material;
use crate::color::Contrast;

type Source = Box<dyn std::error::Error + Send + Sync>;

/// Failure loading or watching a theme file.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ThemeError {
    /// Reading the file.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Parsing the TOML.
    #[error("invalid theme TOML: {0}")]
    Toml(#[source] Source),
    /// File watcher setup.
    #[error("theme file watcher: {0}")]
    Watch(#[source] Source),
}

/// Live-reload handle for a theme file; dropping it stops watching.
#[derive(Debug)]
#[must_use = "dropping the watcher stops watching"]
pub struct ThemeWatcher {
    _debouncer: notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>,
}

// Coalesces the events of one save, including non-atomic ones.
const DEBOUNCE: Duration = Duration::from_millis(100);

/// Loads [`Tokens`] from a TOML file and installs them with [`Material::set_tokens`].
///
/// Missing keys keep the [`Tokens::baseline`] value of the mode named by `color.theme` (light when absent).
///
/// # Errors
/// When the file cannot be read or is not valid [`Tokens`] TOML.
pub fn load_toml(rt: &Runtime<Material>, path: &Path) -> Result<(), ThemeError> {
    let tokens = parse(&std::fs::read_to_string(path)?).map_err(|e| ThemeError::Toml(Box::new(e)))?;
    rt.kit().set_tokens(rt.ctx(), tokens);
    Ok(())
}

fn parse(text: &str) -> Result<Tokens, toml::de::Error> {
    use serde::de::Error as _;

    let mut user: toml::Table = toml::from_str(text)?;
    let theme = match user.get("color").and_then(|c| c.get("theme")) {
        Some(t) => t.clone().try_into()?,
        None => egui::Theme::Light,
    };
    let mut merged =
        toml::Value::try_from(Tokens::baseline(theme, Contrast::STANDARD)).map_err(toml::de::Error::custom)?;
    overlay(&mut merged, toml::Value::Table(std::mem::take(&mut user)));
    merged.try_into()
}

fn overlay(base: &mut toml::Value, top: toml::Value) {
    match (base, top) {
        (toml::Value::Table(base), toml::Value::Table(top)) => {
            for (key, value) in top {
                match base.get_mut(&key) {
                    Some(slot) => overlay(slot, value),
                    None => {
                        base.insert(key, value);
                    }
                }
            }
        }
        (base, top) => *base = top,
    }
}

/// [`load_toml`], then again on every change of `path` until the watcher drops.
///
/// # Errors
/// When the initial [`load_toml`] fails or the file watcher cannot be created or started.
pub fn watch(rt: &Arc<Runtime<Material>>, path: impl Into<PathBuf>) -> Result<ThemeWatcher, ThemeError> {
    use notify_debouncer_mini::{DebounceEventResult, DebouncedEventKind, notify};
    let watch_error = |e: notify::Error| ThemeError::Watch(Box::new(e));
    let path: PathBuf = path.into();
    load_toml(rt, &path)?;
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_owned();
    let (rt, target) = (Arc::downgrade(rt), path.clone());
    let mut debouncer = notify_debouncer_mini::new_debouncer(DEBOUNCE, move |res: DebounceEventResult| {
        let events = match res {
            Ok(events) => events,
            Err(e) => {
                tracing::warn!(path = %target.display(), error = %e, "theme watch failed");
                return;
            }
        };
        let changed = events
            .iter()
            .any(|e| e.kind == DebouncedEventKind::Any && e.path.file_name() == target.file_name());
        let Some(rt) = rt.upgrade().filter(|_| changed) else {
            return;
        };
        if let Err(e) = load_toml(&rt, &target) {
            tracing::warn!(path = %target.display(), error = %e, "theme reload failed");
        }
    })
    .map_err(watch_error)?;
    debouncer
        .watcher()
        .watch(&dir, notify::RecursiveMode::NonRecursive)
        .map_err(watch_error)?;
    Ok(ThemeWatcher { _debouncer: debouncer })
}

#[cfg(all(test, feature = "bundled-icons"))]
mod tests {
    use std::error::Error as _;

    use super::*;

    #[test]
    fn errors_carry_their_source() {
        let rt = crate::test_support::runtime(&egui::Context::default());
        let dir = std::env::temp_dir();
        let missing = dir.join(format!("mineral-theme-{}-missing.toml", std::process::id()));
        assert!(matches!(load_toml(&rt, &missing), Err(ThemeError::Io(_))));

        let bad = dir.join(format!("mineral-theme-{}-bad.toml", std::process::id()));
        std::fs::write(&bad, "color = 1").expect("write");
        let err = load_toml(&rt, &bad).expect_err("invalid tokens");
        std::fs::remove_file(&bad).expect("remove");
        assert!(matches!(err, ThemeError::Toml(_)), "{err}");
        assert!(err.source().is_some(), "toml error is the source");
    }

    #[test]
    fn partial_file_keeps_baseline_for_missing_fields() {
        use egui::Theme;

        let baseline = Tokens::baseline(Theme::Dark, Contrast::STANDARD);
        let tokens = parse("[color]\ntheme = \"Dark\"\n\n[state]\nhover = 0.5\n").expect("partial");
        assert!((tokens.state.hover - 0.5).abs() < f32::EPSILON);
        assert_eq!(tokens.state.focus, baseline.state.focus);
        assert_eq!(tokens.color, baseline.color);
        assert_eq!(tokens.shape, baseline.shape);
        // Durations round-trip through f32 seconds.
        let drift = tokens.duration.medium[0].abs_diff(baseline.duration.medium[0]);
        assert!(drift < std::time::Duration::from_millis(1), "{drift:?}");
        let light = Tokens::baseline(Theme::Light, Contrast::STANDARD);
        assert_eq!(parse("").expect("empty").color, light.color);
    }

    #[test]
    fn watch_reloads_an_edited_file() {
        use std::time::{Duration, Instant};

        use egui::Theme;

        use crate::color::Contrast;

        let rt = crate::test_support::runtime(&egui::Context::default());
        let dir = std::env::temp_dir().join(format!("mineral-theme-{}-watch", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("theme.toml");
        let mut tokens = Tokens::baseline(Theme::Light, Contrast::STANDARD);
        std::fs::write(&path, toml::to_string(&tokens).expect("encode")).expect("write");
        let _watcher = watch(&rt, &path).expect("watch");

        tokens.state.hover = 0.5;
        std::fs::write(&path, toml::to_string(&tokens).expect("encode")).expect("rewrite");
        let deadline = Instant::now() + Duration::from_secs(5);
        let reloaded = loop {
            if (rt.kit().tokens(Theme::Light).state.hover - 0.5).abs() < f32::EPSILON {
                break true;
            }
            if Instant::now() > deadline {
                break false;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        std::fs::remove_dir_all(&dir).expect("cleanup");
        assert!(reloaded, "the edit is installed");
    }
}

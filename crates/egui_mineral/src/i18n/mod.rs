//! Fluent messages and locale formatting, owned by [`I18n`]: `IcuFormat` (ICU4X, feature `icu`), else
//! [`AsciiFormat`].
//!
//! ```
//! use egui_mineral::i18n::{AsciiFormat, DateFields, DateLength, I18n, LocaleFormat};
//!
//! let ctx = egui::Context::default();
//! let i18n = I18n::with_format(&ctx, "und".into(), AsciiFormat::default(), &[("en", "greet = Hello { $name }")])?;
//! assert_eq!(i18n.tr("greet", &[("name", "Ann".into())]), "Hello Ann");
//!
//! let date = egui_mineral::jiff::civil::date(2026, 11, 17);
//! assert_eq!(i18n.format().date(date, DateFields::YearMonthDay, DateLength::Numeric), "2026-11-17");
//! # Ok::<(), egui_mineral::i18n::I18nError>(())
//! ```
//!
//! ICU4X bakes data for every CLDR locale. An app ships a subset by pointing `ICU4X_DATA_DIR` at data
//! generated with the `icu4x-datagen` matching its `icu_*` version (all markers; the ICU crates
//! implement every one); locales left out format as CLDR root, direction stays exact:
//!
//! ```sh
//! icu4x-datagen --format baked --markers all --locales en de ar --out icu_data
//! ```
//! ```toml
//! # .cargo/config.toml
//! [env]
//! ICU4X_DATA_DIR = { value = "icu_data", relative = true }
//! ```

mod ascii;
mod format;
#[cfg(feature = "icu")]
mod icu;

#[cfg(feature = "icu")]
use self::icu::fluent_number;
#[cfg(not(feature = "icu"))]
use ascii::fluent_number;

#[cfg(feature = "icu")]
type SystemFormat = IcuFormat;
#[cfg(not(feature = "icu"))]
type SystemFormat = AsciiFormat;

use egui::Rangef;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ops::ControlFlow,
    sync::Arc,
};

use fluent_bundle::{FluentArgs, FluentResource, FluentValue, concurrent::FluentBundle};
use parking_lot::Mutex;
use unic_langid::LanguageIdentifier;

#[cfg(feature = "icu")]
pub use self::icu::IcuFormat;
pub use self::{
    ascii::AsciiFormat,
    format::{
        DateFields, DateLength, DatePart, DayPeriod, HourCycle, ListKind, LocaleFormat, NameWidth, PeriodPosition,
    },
};

/// Text direction; mirrors layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum Direction {
    /// Left to right.
    #[default]
    Ltr,
    /// Right to left; horizontal layouts and arrow keys mirror.
    Rtl,
}

impl Direction {
    /// Horizontal layout in reading order, vertically centered.
    pub fn row(self) -> egui::Layout {
        match self {
            Self::Ltr => egui::Layout::left_to_right(egui::Align::Center),
            Self::Rtl => egui::Layout::right_to_left(egui::Align::Center),
        }
    }

    /// Vertical layout aligned to the reading start.
    pub fn column(self) -> egui::Layout {
        egui::Layout::top_down(self.start_align())
    }

    /// `ltr` or `rtl` by direction, for directional glyphs and signs.
    pub fn pick<T>(self, ltr: T, rtl: T) -> T {
        match self {
            Self::Ltr => ltr,
            Self::Rtl => rtl,
        }
    }

    /// Logical "next" for a physical horizontal arrow key.
    pub fn forward_key(self) -> egui::Key {
        match self {
            Self::Ltr => egui::Key::ArrowRight,
            Self::Rtl => egui::Key::ArrowLeft,
        }
    }

    /// Opposite direction; its `start_x` places parts from the end edge.
    #[must_use]
    pub fn reversed(self) -> Self {
        match self {
            Self::Ltr => Self::Rtl,
            Self::Rtl => Self::Ltr,
        }
    }

    /// Left edge of `part`, given as offsets into `rect` from its reading-start edge.
    pub fn start_x(self, rect: egui::Rect, part: egui::Rangef) -> f32 {
        self.span_x(rect.x_range(), part)
    }

    /// [`Self::start_x`] within a horizontal span.
    pub fn span_x(self, span: egui::Rangef, part: egui::Rangef) -> f32 {
        let start = match self {
            Self::Ltr => span.min,
            Self::Rtl => span.max,
        };
        self.offset_x(start, part)
    }

    /// Direction measuring from `edge`: itself from the start, reversed from the end.
    #[must_use]
    pub fn from_edge(self, edge: crate::layout::Edge) -> Self {
        match edge {
            crate::layout::Edge::Start => self,
            crate::layout::Edge::End => self.reversed(),
        }
    }

    /// Left edge of `part`, given as offsets past `x` in reading order.
    pub fn offset_x(self, x: f32, part: egui::Rangef) -> f32 {
        match self {
            Self::Ltr => x + part.min,
            Self::Rtl => x - part.max,
        }
    }

    /// Span `width` wide from `x` toward the reading end; an infinite `width` stays NaN-free.
    pub fn span_from(self, x: f32, width: f32) -> egui::Rangef {
        match self {
            Self::Ltr => egui::Rangef::new(x, x + width),
            Self::Rtl => egui::Rangef::new(x - width, x),
        }
    }

    /// Physical x of the reading-start edge of `rect`.
    pub fn start_edge(self, rect: egui::Rect) -> f32 {
        self.span_x(rect.x_range(), Rangef::point(0.0))
    }

    /// Physical x of the reading-end edge of `rect`.
    pub fn end_edge(self, rect: egui::Rect) -> f32 {
        self.reversed().start_edge(rect)
    }

    /// `rect` laid out in left-to-right terms within `within`, mirrored horizontally for RTL.
    pub fn mirror(self, rect: egui::Rect, within: egui::Rect) -> egui::Rect {
        match self {
            Self::Ltr => rect,
            Self::Rtl => egui::Rect::from_x_y_ranges(
                self.mirror_x(rect.right(), within)..=self.mirror_x(rect.left(), within),
                rect.y_range(),
            ),
        }
    }

    /// Horizontal alignment at the reading start.
    pub fn start_align(self) -> egui::Align {
        match self {
            Self::Ltr => egui::Align::Min,
            Self::Rtl => egui::Align::Max,
        }
    }

    /// Horizontal alignment at the reading end.
    pub fn end_align(self) -> egui::Align {
        self.reversed().start_align()
    }

    /// `1.0` when reading order runs left to right, `-1.0` otherwise.
    pub fn sign(self) -> f32 {
        match self {
            Self::Ltr => 1.0,
            Self::Rtl => -1.0,
        }
    }

    /// Physical sides of a logical `start` and `end` pair (margins, insets, flags).
    pub fn sides<T>(self, start: T, end: T) -> Sides<T> {
        match self {
            Self::Ltr => Sides {
                left: start,
                right: end,
            },
            Self::Rtl => Sides {
                left: end,
                right: start,
            },
        }
    }

    /// Physical column, from the left, of reading-order slot `i` among `n`.
    pub fn slot(self, i: usize, n: usize) -> usize {
        match self {
            Self::Ltr => i,
            Self::Rtl => n.saturating_sub(1).saturating_sub(i),
        }
    }

    /// Cursor taking parts of `rect` in reading order from its start edge.
    pub fn cursor(self, rect: egui::Rect) -> RowCursor {
        RowCursor {
            rect,
            dir: self,
            offset: 0.0,
        }
    }

    /// Physical `x` in logical (reading-order) coordinates of `within`, and back.
    pub fn mirror_x(self, x: f32, within: egui::Rect) -> f32 {
        match self {
            Self::Ltr => x,
            Self::Rtl => within.left() + within.right() - x,
        }
    }
}

/// Physical left and right values of a logical pair ([`Direction::sides`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Sides<T> {
    /// Value on the left edge.
    pub left: T,
    /// Value on the right edge.
    pub right: T,
}

/// Reading-order layout of one row ([`Direction::cursor`]): parts advance from the start edge, mirrored in RTL.
#[derive(Clone, Copy, Debug)]
pub struct RowCursor {
    rect: egui::Rect,
    dir: Direction,
    offset: f32,
}

impl RowCursor {
    /// Moves `w` points toward the end edge.
    pub fn skip(&mut self, w: f32) {
        self.offset += w;
    }

    /// `w` x `h` slot at the cursor, vertically centered in the row; the cursor stays.
    pub fn peek(&self, w: f32, h: f32) -> egui::Rect {
        let x = self.dir.start_x(self.rect, Rangef::new(self.offset, self.offset + w));
        egui::Rect::from_min_size(egui::pos2(x, self.rect.center().y - h / 2.0), egui::vec2(w, h))
    }

    /// [`Self::peek`], then moves past the slot.
    pub fn take(&mut self, w: f32, h: f32) -> egui::Rect {
        let r = self.peek(w, h);
        self.skip(w);
        r
    }

    /// Physical x of the cursor: where the next part's start edge sits.
    pub fn x(&self) -> f32 {
        self.dir.start_x(self.rect, Rangef::point(self.offset))
    }

    /// Points taken from the start edge so far.
    pub fn offset(&self) -> f32 {
        self.offset
    }

    /// Row width left between the cursor and the end edge.
    pub fn remaining(&self) -> f32 {
        (self.rect.width() - self.offset).max(0.0)
    }
}

/// Fluent message argument.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Arg<'a> {
    /// Text argument.
    Str(Cow<'a, str>),
    /// Numeric argument; Fluent plural rules and number formatting apply.
    Num(f64),
}

impl<'a> From<&'a str> for Arg<'a> {
    fn from(s: &'a str) -> Self {
        Self::Str(Cow::Borrowed(s))
    }
}

impl<'a> From<&'a String> for Arg<'a> {
    fn from(s: &'a String) -> Self {
        Self::Str(Cow::Borrowed(s))
    }
}

impl From<String> for Arg<'_> {
    fn from(s: String) -> Self {
        Self::Str(Cow::Owned(s))
    }
}

macro_rules! num_arg {
    (lossless: $($t:ty),*; lossy: $($l:ty),*) => {
        $(impl From<$t> for Arg<'_> {
            fn from(n: $t) -> Self {
                Self::Num(f64::from(n))
            }
        })*
        $(impl From<$l> for Arg<'_> {
            #[expect(clippy::cast_precision_loss, reason = "magnitudes beyond 2^53 round")]
            fn from(n: $l) -> Self {
                Self::Num(n as f64)
            }
        })*
    };
}
num_arg!(lossless: f64, f32, i32, i16, i8, u32, u16, u8; lossy: i64, u64, usize);

impl Arg<'_> {
    /// Clones borrowed text so the arg outlives its source.
    pub fn into_owned(self) -> Arg<'static> {
        match self {
            Self::Str(s) => Arg::Str(Cow::Owned(s.into_owned())),
            Self::Num(n) => Arg::Num(n),
        }
    }
}

/// Message arguments by name.
pub type Args<'a> = [(&'a str, Arg<'a>)];

/// App message catalog for one locale.
pub trait Localizer: Send + Sync {
    /// BCP 47 tag of the primary catalog.
    fn locale(&self) -> &str;
    /// `None` when the message is missing. Messages without arguments are cached until the locale, localizer or
    /// catalogs change.
    fn message(&self, id: &str, args: &Args<'_>) -> Option<String>;
}

/// Failure selecting a locale or loading a message catalog.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum I18nError {
    /// Invalid or unloadable BCP 47 tag; `tag` is empty when the chain is empty.
    #[error("invalid locale `{tag}`{}", cause_suffix(source.as_deref()))]
    Locale {
        /// Offending tag.
        tag: String,
        /// Parser or data loader error, when there is one.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    /// FTL parse or bundle error.
    #[error("ftl: {0}")]
    Resource(#[source] Box<dyn std::error::Error + Send + Sync>),
}

fn cause_suffix(source: Option<&(dyn std::error::Error + Send + Sync)>) -> String {
    source.map_or_else(String::new, |e| format!(": {e}"))
}

#[derive(Debug)]
pub(super) struct Messages(Vec<String>);

impl Messages {
    pub(super) fn boxed(
        errs: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Box<dyn std::error::Error + Send + Sync> {
        Box::new(Self(errs.into_iter().map(|e| e.to_string()).collect()))
    }
}

impl std::fmt::Display for Messages {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.join("; "))
    }
}

impl std::error::Error for Messages {}

/// [`Localizer`] over Fluent bundles with locale fallback.
pub struct FluentLocalizer {
    locale: String,
    bundles: Vec<FluentBundle<FluentResource>>,
}

crate::kit::opaque_debug!(FluentLocalizer);

impl FluentLocalizer {
    /// `chain` is `(locale, ftl source)` in fallback order; the first is the primary locale.
    ///
    /// # Errors
    /// [`I18nError::Locale`] when a locale does not parse or `chain` is empty, [`I18nError::Resource`] when FTL does not parse or register.
    pub fn new<'a>(chain: impl IntoIterator<Item = (&'a str, &'a str)>) -> Result<Self, I18nError> {
        let mut bundles = Vec::new();
        for (locale, ftl) in chain {
            let lang: LanguageIdentifier = locale.parse().map_err(|e| I18nError::Locale {
                tag: locale.into(),
                source: Some(Box::new(e)),
            })?;
            let res = FluentResource::try_new(ftl.to_owned())
                .map_err(|(_, errs)| I18nError::Resource(Messages::boxed(errs)))?;
            let mut bundle = FluentBundle::new_concurrent(vec![lang]);
            // egui renders the default FSI/PDI isolation marks as glyph boxes.
            bundle.set_use_isolating(false);
            bundle.set_formatter(Some(fluent_number));
            bundle
                .add_builtins()
                .map_err(|e| I18nError::Resource(Messages::boxed([e])))?;
            bundle
                .add_resource(res)
                .map_err(|errs| I18nError::Resource(Messages::boxed(errs)))?;
            bundles.push(bundle);
        }
        let locale = bundles
            .first()
            .map(|b| b.locales[0].to_string())
            .ok_or_else(|| I18nError::Locale {
                tag: String::new(),
                source: None,
            })?;
        Ok(Self { locale, bundles })
    }
}

impl Localizer for FluentLocalizer {
    fn locale(&self) -> &str {
        &self.locale
    }

    fn message(&self, id: &str, args: &Args<'_>) -> Option<String> {
        let mut fargs = FluentArgs::new();
        for (k, v) in args {
            match v {
                Arg::Str(s) => fargs.set(*k, FluentValue::from(s.as_ref())),
                Arg::Num(n) => fargs.set(*k, FluentValue::from(*n)),
            }
        }
        self.bundles.iter().find_map(|bundle| {
            let pattern = bundle.get_message(id)?.value()?;
            let mut errs = Vec::new();
            let out = bundle.format_pattern(pattern, Some(&fargs), &mut errs);
            if !errs.is_empty() {
                crate::kit::sanitize::report(format_args!("fluent format errors in message {id:?}: {errs:?}"));
            }
            Some(out.into_owned())
        })
    }
}

const ROOT: &str = "und";

/// Library catalog: `(locale, ftl source)`; a library's first catalog is its source language.
pub type Catalog = (&'static str, &'static str);

struct State {
    locale: String,
    app: Option<Arc<dyn Localizer>>,
    format: Arc<dyn LocaleFormat>,
    sources: Vec<Catalog>,
    defaults: Option<Arc<FluentLocalizer>>,
    plain: HashMap<String, Arc<str>>,
    missing: HashSet<String>,
    generation: u64,
}

impl State {
    fn set_defaults(&mut self, defaults: Option<Arc<FluentLocalizer>>) {
        self.defaults = defaults;
        self.plain.clear();
        self.missing.clear();
        self.generation = self.generation.wrapping_add(1);
    }
}

fn negotiate(sources: &[Catalog], locale: &str) -> Result<Option<Arc<FluentLocalizer>>, I18nError> {
    let parsed: Vec<(LanguageIdentifier, Catalog)> = sources
        .iter()
        .map(|&(tag, ftl)| {
            tag.parse()
                .map(|lang| (lang, (tag, ftl)))
                .map_err(|e| I18nError::Locale {
                    tag: tag.into(),
                    source: Some(Box::new(e)),
                })
        })
        .collect::<Result<_, _>>()?;
    let Some((fallback, _)) = parsed.first() else {
        return Ok(None);
    };
    let fallback = fallback.clone();
    let available: Vec<LanguageIdentifier> = parsed.iter().map(|(lang, _)| lang.clone()).collect();
    let requested: Vec<LanguageIdentifier> = locale.parse().into_iter().collect();
    let mut order: Vec<LanguageIdentifier> = fluent_langneg::negotiate_languages(
        &requested,
        &available,
        Some(&fallback),
        fluent_langneg::NegotiationStrategy::Filtering,
    )
    .into_iter()
    .cloned()
    .collect();
    if !order.contains(&fallback) {
        order.push(fallback);
    }
    let chain = order
        .iter()
        .flat_map(|lang| parsed.iter().filter(move |(l, _)| l == lang).map(|(_, c)| *c));
    Ok(Some(Arc::new(FluentLocalizer::new(chain)?)))
}

/// Active locale, formatter and message catalogs; owned by [`Core`](crate::runtime::Core).
///
/// Message lookup: app [`Localizer`], then library catalogs negotiated for the active locale
/// (the first registered catalog's locale last), then the id itself with a warning.
pub struct I18n {
    ctx: egui::Context,
    state: Mutex<State>,
}

crate::kit::opaque_debug!(I18n);

impl I18n {
    /// OS/browser locale, else the CLDR root (`und`), with library `catalogs`.
    ///
    /// # Errors
    /// [`I18nError::Locale`] when the root locale fails to load, [`I18nError::Resource`] when a catalog does not
    /// parse.
    pub fn new(ctx: &egui::Context, catalogs: &[Catalog]) -> Result<Self, I18nError> {
        let system = system_locale().and_then(|l| match SystemFormat::new(&l) {
            Ok(f) => Some((l, f)),
            Err(error) => {
                tracing::warn!(target: "egui_mineral", tag = %l, %error, "system locale unsupported, using root");
                None
            }
        });
        let (locale, format) = match system {
            Some(found) => found,
            None => (ROOT.to_owned(), SystemFormat::new(ROOT)?),
        };
        Self::with_format(ctx, locale, format, catalogs)
    }

    /// `format` for `locale`, e.g. a fixed-locale [`AsciiFormat`].
    ///
    /// # Errors
    /// [`I18nError::Locale`] when `locale` does not parse, [`I18nError::Resource`] when a catalog does not parse.
    pub fn with_format(
        ctx: &egui::Context,
        locale: String,
        format: impl LocaleFormat + 'static,
        catalogs: &[Catalog],
    ) -> Result<Self, I18nError> {
        locale.parse::<LanguageIdentifier>().map_err(|e| I18nError::Locale {
            tag: locale.clone(),
            source: Some(Box::new(e)),
        })?;
        for &(tag, ftl) in catalogs {
            FluentLocalizer::new([(tag, ftl)])?;
        }
        let defaults = negotiate(catalogs, &locale)?;
        let state = State {
            locale,
            app: None,
            format: Arc::new(format),
            sources: catalogs.to_vec(),
            defaults,
            plain: HashMap::new(),
            missing: HashSet::new(),
            generation: 0,
        };
        Ok(Self {
            ctx: ctx.clone(),
            state: Mutex::new(state),
        })
    }

    fn with<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
        f(&mut self.state.lock())
    }

    #[expect(clippy::disallowed_methods, reason = "one pass after the locale or format changed")]
    fn changed(&self) {
        self.ctx.request_repaint();
    }

    /// Switches formatting, direction and messages to `tag`.
    ///
    /// # Errors
    /// [`I18nError::Locale`] when `tag` does not parse; a locale without baked ICU4X data formats as CLDR root.
    pub fn set_locale(&self, tag: &str) -> Result<(), I18nError> {
        self.switch(tag, None)
    }

    fn switch(&self, tag: &str, app: Option<&Arc<dyn Localizer>>) -> Result<(), I18nError> {
        let format: Arc<dyn LocaleFormat> = Arc::new(SystemFormat::new(tag)?);
        // Negotiation parses catalogs: it runs outside the state lock and retries when state changed meanwhile.
        loop {
            let (sources, generation) = self.with(|s| (s.sources.clone(), s.generation));
            let defaults = negotiate(&sources, tag)?;
            let applied = self.with(|s| {
                if s.generation != generation {
                    return false;
                }
                tag.clone_into(&mut s.locale);
                s.format = Arc::clone(&format);
                if app.is_some() {
                    s.app = app.cloned();
                }
                s.set_defaults(defaults);
                true
            });
            if applied {
                break;
            }
        }
        self.changed();
        Ok(())
    }

    /// Installs the app catalog and switches to its locale.
    ///
    /// # Errors
    /// As [`Self::set_locale`] for the locale of `localizer`.
    pub fn set_localizer(&self, localizer: impl Localizer + 'static) -> Result<(), I18nError> {
        let tag = localizer.locale().to_owned();
        let app: Arc<dyn Localizer> = Arc::new(localizer);
        self.switch(&tag, Some(&app))
    }

    /// App catalog set by [`Self::set_localizer`]; `None` while only library catalogs are registered.
    pub fn localizer(&self) -> Option<Arc<dyn Localizer>> {
        self.with(|s| s.app.clone())
    }

    /// Overrides formatting (e.g. [`AsciiFormat`]); keeps messages.
    pub fn set_format(&self, format: impl LocaleFormat + 'static) {
        self.with(|s| {
            s.format = Arc::new(format);
            s.generation = s.generation.wrapping_add(1);
        });
        self.changed();
    }

    /// Registers a library catalog after construction; duplicates are ignored.
    ///
    /// # Errors
    /// As [`FluentLocalizer::new`] for the catalog.
    pub fn add_resource(&self, catalog: Catalog) -> Result<(), I18nError> {
        FluentLocalizer::new([catalog])?;
        let added = loop {
            let snapshot = self.with(|s| {
                let known = s
                    .sources
                    .iter()
                    .any(|&(l, f)| l == catalog.0 && std::ptr::eq(f, catalog.1));
                (!known).then(|| (s.sources.clone(), s.locale.clone(), s.generation))
            });
            let Some((mut sources, locale, generation)) = snapshot else {
                break false;
            };
            sources.push(catalog);
            let defaults = negotiate(&sources, &locale)?;
            let applied = self.with(|s| {
                if s.generation != generation {
                    return false;
                }
                s.sources = sources;
                s.set_defaults(defaults);
                true
            });
            if applied {
                break true;
            }
        };
        if added {
            self.changed();
        }
        Ok(())
    }

    /// Changes whenever the locale, format or catalogs change; keys caches of localized output.
    pub fn revision(&self) -> u64 {
        self.with(|s| s.generation)
    }

    /// Active locale tag.
    pub fn locale(&self) -> String {
        self.with(|s| s.locale.clone())
    }

    /// Active formatter for dates, times, numbers and lists.
    pub fn format(&self) -> Arc<dyn LocaleFormat> {
        self.with(|s| s.format.clone())
    }

    /// Active text direction.
    pub fn direction(&self) -> Direction {
        self.with(|s| s.format.direction())
    }

    /// `i18n.tr("mineral-min-length", &[("min", 8.into())])`
    pub fn tr(&self, id: &str, args: &Args<'_>) -> String {
        // Snapshot under one lock; messages format outside it, an app localizer may call back in.
        let snapshot = self.with(|s| {
            if s.missing.contains(id) {
                return ControlFlow::Break(None);
            }
            match s.plain.get(id).filter(|_| args.is_empty()) {
                Some(m) => ControlFlow::Break(Some(Arc::clone(m))),
                None => ControlFlow::Continue((s.app.clone(), s.defaults.clone(), s.generation)),
            }
        });
        let (app, defaults, generation) = match snapshot {
            ControlFlow::Continue(snapshot) => snapshot,
            ControlFlow::Break(cached) => return cached.map_or_else(|| id.to_owned(), |m| m.to_string()),
        };
        let found = app
            .and_then(|a| a.message(id, args))
            .or_else(|| defaults.and_then(|d| d.message(id, args)));
        // Messages with arguments are not cached: only a miss or a plain message takes the lock again.
        if found.is_none() || args.is_empty() {
            let plain = found.as_deref().map(Arc::<str>::from);
            self.with(|s| {
                if s.generation != generation {
                    return;
                }
                match plain {
                    None => {
                        tracing::warn!(target: "egui_mineral", id, "missing message");
                        s.missing.insert(id.to_owned());
                    }
                    Some(m) => {
                        s.plain.insert(id.to_owned(), m);
                    }
                }
            });
        }
        found.unwrap_or_else(|| id.to_owned())
    }
}

/// OS locale tag; `None` when unknown or not a valid language identifier.
pub fn system_locale() -> Option<String> {
    sys_locale::get_locale().filter(|l| l.parse::<LanguageIdentifier>().is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_places_parts_from_the_reading_start() {
        let r = egui::Rect::from_min_max(egui::pos2(10.0, 0.0), egui::pos2(110.0, 20.0));
        assert_eq!(Direction::Ltr.start_x(r, Rangef::new(5.0, 5.0 + 20.0)), 15.0);
        assert_eq!(Direction::Rtl.start_x(r, Rangef::new(5.0, 5.0 + 20.0)), 85.0);
        let part = egui::Rect::from_min_max(egui::pos2(15.0, 0.0), egui::pos2(35.0, 20.0));
        assert_eq!(Direction::Rtl.mirror(part, r).x_range(), egui::Rangef::new(85.0, 105.0));
        assert_eq!(Direction::Ltr.mirror(part, r), part);
        let mut c = Direction::Rtl.cursor(r);
        assert_eq!(c.take(20.0, 10.0).x_range(), egui::Rangef::new(90.0, 110.0));
        c.skip(4.0);
        assert_eq!(c.x(), 86.0);
        assert_eq!(c.remaining(), 76.0);
        assert_eq!(
            Direction::Rtl.sides("start", "end"),
            Sides {
                left: "end",
                right: "start"
            }
        );
        assert_eq!(
            (Direction::Rtl.start_edge(r), Direction::Rtl.end_edge(r)),
            (110.0, 10.0)
        );
        assert_eq!(Direction::Rtl.offset_x(50.0, Rangef::new(5.0, 5.0 + 10.0)), 35.0);
        assert_eq!(
            Direction::Rtl.span_from(50.0, f32::INFINITY),
            egui::Rangef::new(f32::NEG_INFINITY, 50.0)
        );
        assert_eq!((Direction::Ltr.slot(1, 7), Direction::Rtl.slot(1, 7)), (1, 5));
    }

    #[test]
    fn errors_carry_their_cause() {
        use std::error::Error;

        let bad_tag = FluentLocalizer::new([("not a tag!", "a = b")]).unwrap_err();
        assert!(matches!(&bad_tag, I18nError::Locale { tag, .. } if tag == "not a tag!"));
        assert!(bad_tag.source().is_some(), "the parser error is the source");

        let bad_ftl = FluentLocalizer::new([("en", "= no id")]).unwrap_err();
        assert!(matches!(bad_ftl, I18nError::Resource(_)));
        assert!(bad_ftl.source().is_some());

        let empty = FluentLocalizer::new(std::iter::empty::<(&str, &str)>()).unwrap_err();
        assert!(matches!(&empty, I18nError::Locale { tag, .. } if tag.is_empty()));
        assert!(empty.source().is_none(), "an empty chain has no cause");
    }

    #[test]
    fn fallback_and_args() {
        let l = FluentLocalizer::new([
            ("de", "hello = Hallo { $name }!"),
            ("en", "hello = Hello { $name }!\nbye = Bye"),
        ])
        .unwrap();
        assert_eq!(
            l.message("hello", &[("name", "Ann".into())]).as_deref(),
            Some("Hallo Ann!")
        );
        assert_eq!(l.message("bye", &[]).as_deref(), Some("Bye"));
        assert_eq!(l.message("missing", &[]), None);
    }

    #[test]
    fn library_catalog_negotiation() {
        let ctx = egui::Context::default();
        let i18n = I18n::new(&ctx, &[]).unwrap();
        assert_eq!(i18n.tr("mineral-submit", &[]), "mineral-submit", "no catalog: the id");
        i18n.add_resource((
            "en-US",
            "mineral-submit = Submit\nmineral-number = Enter a number\nmineral-percent = { NUMBER($value, maximumFractionDigits: 0) }%",
        ))
        .unwrap();
        i18n.add_resource(("de", "mineral-submit = Absenden")).unwrap();
        assert!(i18n.add_resource(("fr", "broken = {")).is_err());
        assert!(i18n.set_locale("not a tag!").is_err());
        i18n.set_locale("de-AT").unwrap();
        assert_eq!(i18n.tr("mineral-submit", &[]), "Absenden");
        assert_eq!(
            i18n.tr("mineral-number", &[]),
            "Enter a number",
            "source catalog fills gaps"
        );
        i18n.set_locale("fr").unwrap();
        assert_eq!(i18n.tr("mineral-submit", &[]), "Submit");
        assert_eq!(
            i18n.tr("mineral-percent", &[("value", 42.4.into())]),
            "42%",
            "builtin NUMBER"
        );
        i18n.set_locale("ar").unwrap();
        assert_eq!(i18n.direction(), Direction::Rtl);
    }

    #[test]
    fn plain_messages_resolve_once_per_localizer() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct Counting(Arc<AtomicUsize>, &'static str);
        impl Localizer for Counting {
            fn locale(&self) -> &'static str {
                "en"
            }
            fn message(&self, id: &str, args: &Args<'_>) -> Option<String> {
                self.0.fetch_add(1, Ordering::Relaxed);
                (id != "gone").then(|| format!("{}{}", self.1, args.len()))
            }
        }
        let ctx = egui::Context::default();
        let i18n = I18n::new(&ctx, &[]).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        i18n.set_localizer(Counting(calls.clone(), "a")).unwrap();
        assert_eq!(i18n.tr("m", &[]), "a0");
        assert_eq!(i18n.tr("m", &[]), "a0");
        assert_eq!(calls.load(Ordering::Relaxed), 1, "cached");
        assert_eq!(i18n.tr("m", &[("n", 1.into())]), "a1");
        assert_eq!(i18n.tr("m", &[("n", 1.into())]), "a1");
        assert_eq!(calls.load(Ordering::Relaxed), 3, "arguments are not cached");
        assert_eq!(i18n.tr("gone", &[("n", 1.into())]), "gone");
        assert_eq!(i18n.tr("gone", &[]), "gone");
        assert_eq!(calls.load(Ordering::Relaxed), 4, "a missing id is looked up once");
        i18n.set_localizer(Counting(calls, "b")).unwrap();
        assert_eq!(i18n.tr("m", &[]), "b0", "a new localizer drops the cache");
    }

    #[test]
    fn added_catalog_repaints() {
        let ctx = egui::Context::default();
        let i18n = I18n::new(&ctx, &[("en-US", "greeting = One")]).unwrap();
        for _ in 0..8 {
            if !ctx.has_requested_repaint() {
                break;
            }
            ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
        }
        assert!(!ctx.has_requested_repaint(), "settled");
        i18n.add_resource(("en-US", "farewell = Two")).unwrap();
        assert!(ctx.has_requested_repaint(), "a new catalog repaints");
        assert_eq!(i18n.tr("farewell", &[]), "Two");
    }

    #[test]
    fn invalid_catalog_fails_construction() {
        let ctx = egui::Context::default();
        assert!(I18n::new(&ctx, &[("en", "broken = {")]).is_err());
        assert!(I18n::with_format(&ctx, "not a tag!".into(), AsciiFormat::default(), &[]).is_err());
    }
}

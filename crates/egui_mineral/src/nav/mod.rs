#![warn(clippy::arithmetic_side_effects)]

pub use egui_mineral_macros::Route;

/// Implemented by `#[derive(Route)]`, which also derives `Display` (the [`path`](Self::path)) and `FromStr`
/// (the [`parse`](Self::parse)); `parse(&r.path()) == Ok(r)` must hold. A parameter that
/// displays empty is written as the segment `~`; a value starting with `~` escapes it as `%7E`.
///
/// ```
/// use egui_mineral::nav::Route;
///
/// #[derive(Clone, Debug, PartialEq, Route)]
/// enum AppRoute {
///     #[route("/")]
///     Home,
///     #[route("/user/{id}/posts/{tag?}")]
///     Posts { id: u32, tag: Option<String> },
/// }
///
/// let r = AppRoute::Posts { id: 7, tag: None };
/// assert_eq!(r.path(), "/user/7/posts");
/// assert_eq!(AppRoute::parse("/user/7/posts")?, r);
/// # Ok::<(), egui_mineral::nav::RouteError>(())
/// ```
pub trait Route: Sized + Clone + std::fmt::Display + std::str::FromStr<Err = RouteError> {
    /// Absolute path with percent-encoded segments, e.g. `/users/42`.
    fn path(&self) -> String;
    /// Inverse of [`Self::path`]; ignores empty segments.
    ///
    /// # Errors
    /// [`RouteError`] when `path` matches no variant or a segment does not parse.
    fn parse(path: &str) -> Result<Self, RouteError>;
}

/// Failure matching or navigating a route.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RouteError {
    /// No variant matches `path`.
    #[error("no route matches `{path}`")]
    NoMatch {
        /// Full path being parsed.
        path: String,
    },
    /// Matched a variant, but a segment failed `FromStr` for its field.
    #[error("`{path}`: segment `{segment}` is not a valid `{field}`")]
    InvalidParam {
        /// Full path being parsed.
        path: String,
        /// Route field the segment binds to.
        field: &'static str,
        /// Raw, still percent-encoded segment.
        segment: String,
    },
    /// [`Router::from_paths`] got no paths.
    #[error("route stack is empty")]
    EmptyStack,
}

/// Typed route stack over a `#[derive(Route)]` enum; the root always exists.
#[derive(Clone, Debug)]
pub struct Router<R> {
    root: R,
    above: Vec<R>,
}

impl<R: Route> Router<R> {
    /// Stack holding only `root`.
    pub fn new(root: R) -> Self {
        Self {
            root,
            above: Vec::new(),
        }
    }

    /// Top of the stack.
    pub fn current(&self) -> &R {
        self.above.last().unwrap_or(&self.root)
    }

    /// Bottom (root) to top.
    pub fn stack(&self) -> impl DoubleEndedIterator<Item = &R> {
        std::iter::once(&self.root).chain(&self.above)
    }

    /// Routes on the stack, at least 1.
    pub fn depth(&self) -> usize {
        self.above.len().saturating_add(1)
    }

    /// Pushes on top; [`Self::back`] returns to the previous route.
    pub fn push(&mut self, route: R) {
        self.above.push(route);
    }

    /// Swaps the top route without growing the stack.
    pub fn replace(&mut self, route: R) {
        *self.above.last_mut().unwrap_or(&mut self.root) = route;
    }

    /// Pops the top; `false` and no change at the root.
    pub fn back(&mut self) -> bool {
        self.above.pop().is_some()
    }

    /// More than one route on the stack.
    pub fn can_go_back(&self) -> bool {
        !self.above.is_empty()
    }

    /// Keeps the root and puts `route` on top, or replaces the root when both share a path.
    pub fn reset_to(&mut self, route: R) {
        self.above.clear();
        if self.root.path() == route.path() {
            self.root = route;
        } else {
            self.above.push(route);
        }
    }

    /// Replaces the stack with every parseable prefix of `url`, so back navigates up the hierarchy.
    /// Query and fragment are ignored. Leaves the stack untouched if the full path is not a route.
    ///
    /// # Errors
    /// [`RouteError`] when the full path is not a route.
    pub fn deep_link(&mut self, url: &str) -> Result<(), RouteError> {
        let segs = __private::segments(url);
        let target = R::parse(&format!("/{}", segs.join("/")))?;
        let mut chain: Vec<R> = (0..segs.len())
            .filter_map(|n| R::parse(&format!("/{}", segs[..n].join("/"))).ok())
            .collect();
        chain.push(target);
        chain.dedup_by(|a, b| a.path() == b.path());
        if let Some(router) = Self::from_chain(chain) {
            *self = router;
        }
        Ok(())
    }

    /// Paths bottom to top, for persistence; restore with [`Self::from_paths`].
    pub fn to_paths(&self) -> Vec<String> {
        self.stack().map(Route::path).collect()
    }

    /// Restores a stack saved by [`Self::to_paths`].
    ///
    /// # Errors
    /// Error of the first path that does not parse; [`RouteError::EmptyStack`] when `paths` is empty.
    pub fn from_paths(paths: &[String]) -> Result<Self, RouteError> {
        let chain: Vec<R> = paths.iter().map(|p| R::parse(p)).collect::<Result<_, _>>()?;
        Self::from_chain(chain).ok_or(RouteError::EmptyStack)
    }

    fn from_chain(chain: Vec<R>) -> Option<Self> {
        let mut routes = chain.into_iter();
        let root = routes.next()?;
        Some(Self {
            root,
            above: routes.collect(),
        })
    }
}

#[doc(hidden)]
pub mod __private {
    use std::borrow::Cow;

    use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};

    const SEGMENT: &AsciiSet = &CONTROLS
        .add(b' ')
        .add(b'"')
        .add(b'#')
        .add(b'%')
        .add(b'/')
        .add(b'?')
        .add(b'<')
        .add(b'>');

    pub fn encode(s: &str) -> Cow<'_, str> {
        utf8_percent_encode(s, SEGMENT).into()
    }

    const EMPTY: &str = "~";

    /// Encoded path segment of a route field; empty values become `~`.
    pub fn segment(_field: &str, value: &impl std::fmt::Display) -> String {
        let s = value.to_string();
        if s.is_empty() {
            return EMPTY.to_owned();
        }
        match s.strip_prefix(EMPTY) {
            Some(rest) => format!("%7E{}", encode(rest)),
            None => encode(&s).into_owned(),
        }
    }

    pub fn param<T: std::str::FromStr>(path: &str, field: &'static str, segment: &str) -> Result<T, super::RouteError> {
        let decoded = if segment == EMPTY {
            Ok(Cow::Borrowed(""))
        } else {
            percent_decode_str(segment).decode_utf8()
        };
        decoded
            .ok()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| super::RouteError::InvalidParam {
                path: path.to_owned(),
                field,
                segment: segment.to_owned(),
            })
    }

    pub fn segments(url: &str) -> Vec<&str> {
        let path = url.split(['?', '#']).next().unwrap_or("");
        path.split('/').filter(|s| !s.is_empty()).collect()
    }
}

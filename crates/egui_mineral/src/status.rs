//! Status bar data: items a stylekit's status bar lays out with `kit::strip` and paints.

use egui::Color32;

use crate::{
    icon::Icon,
    layout::{Elide, Fraction},
};

/// Progress indicator of a [`StatusItem`]: the fraction done, `None` for indeterminate.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum StatusProgress {
    /// Circular indicator.
    Circular(Option<Fraction>),
    /// Linear indicator.
    Linear(Option<Fraction>),
}

/// Part before the text of a [`StatusItem`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum StatusLead {
    /// Value of [`StatusItem::icon`].
    Icon(Icon),
    /// Value of [`StatusItem::progress`].
    Progress(StatusProgress),
}

/// Color treatment of a [`StatusItem`]; the stylekit maps it to its roles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StatusTone {
    /// Bar container and content colors.
    #[default]
    Default,
    /// Error colors.
    Error,
    /// App-chosen colors, e.g. a warning tone.
    Custom {
        /// Strip behind the item.
        container: Color32,
        /// Text, icon and indicator.
        content: Color32,
    },
}

/// Text, with an optional icon or progress indicator, in a stylekit's status bar; the bar's `action` pairs it with
/// the value it reports when clicked.
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct StatusItem {
    text: String,
    lead: Option<StatusLead>,
    tooltip: Option<String>,
    tone: StatusTone,
    elide: Option<Elide>,
    tabular: bool,
}

impl StatusItem {
    /// `text` is shown and is the accessible name.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            lead: None,
            tooltip: None,
            tone: StatusTone::Default,
            elide: None,
            tabular: false,
        }
    }

    /// Icon before the text; replaces a [`progress`](Self::progress) indicator.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.lead = Some(StatusLead::Icon(icon.into()));
        self
    }

    /// Progress indicator before the text, named by the item text; replaces an [`icon`](Self::icon).
    pub fn progress(mut self, progress: StatusProgress) -> Self {
        self.lead = Some(StatusLead::Progress(progress));
        self
    }

    /// Tooltip on hover; also the item's accessible description.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// [`StatusTone`] of the item's strip; default [`StatusTone::Default`].
    pub fn tone(mut self, tone: StatusTone) -> Self {
        self.tone = tone;
        self
    }

    /// Start items only: the one that overflows shrinks to the room left, `elide` choosing the part replaced
    /// by an ellipsis, rather than vanishing with the following items. Default: it vanishes.
    pub fn elide(mut self, elide: Elide) -> Self {
        self.elide = Some(elide);
        self
    }

    /// Digits in cells as wide as the font's widest one ([`kit::paint::tabular_figures`](crate::kit::paint::tabular_figures)),
    /// so a counter keeps its width as it changes; default `false`. A kit may show an item shrunk by
    /// [`Self::elide`] with its figures as shaped.
    pub fn tabular_figures(mut self, tabular_figures: bool) -> Self {
        self.tabular = tabular_figures;
        self
    }

    /// Shown text and accessible name.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Icon or progress indicator before the text.
    pub fn lead(&self) -> Option<StatusLead> {
        self.lead
    }

    /// Value of [`Self::tooltip`].
    pub fn tooltip_text(&self) -> Option<&str> {
        self.tooltip.as_deref()
    }

    /// Value of [`Self::tone`].
    pub fn tone_kind(&self) -> StatusTone {
        self.tone
    }

    /// Value of [`Self::tabular_figures`].
    pub fn has_tabular_figures(&self) -> bool {
        self.tabular
    }

    /// Value of [`Self::elide`].
    pub fn elision(&self) -> Option<Elide> {
        self.elide
    }
}

impl<S: Into<String>> From<S> for StatusItem {
    fn from(text: S) -> Self {
        Self::new(text)
    }
}

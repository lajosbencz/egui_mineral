//! Status bar data: items a stylekit's status bar lays out with `kit::strip` and paints.

use egui::Color32;

use crate::icon::Icon;

/// Progress indicator of a [`StatusItem`]: the fraction done, `None` for indeterminate.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum StatusProgress {
    /// Circular indicator.
    Circular(Option<f32>),
    /// Linear indicator.
    Linear(Option<f32>),
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
}

/// App-chosen colors of a [`StatusItem`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StatusColors {
    /// Strip behind the item.
    pub container: Color32,
    /// Text, icon and indicator.
    pub content: Color32,
}

/// Text, with an optional icon or progress indicator, in a stylekit's status bar.
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct StatusItem {
    text: String,
    lead: Option<StatusLead>,
    tooltip: Option<String>,
    clickable: bool,
    tone: StatusTone,
    colors: Option<StatusColors>,
    elide: bool,
}

impl StatusItem {
    /// `text` is shown and is the accessible name.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            lead: None,
            tooltip: None,
            clickable: false,
            tone: StatusTone::Default,
            colors: None,
            elide: false,
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

    /// Focusable button, reported as clicked by the bar; default `false`.
    pub fn clickable(mut self, clickable: bool) -> Self {
        self.clickable = clickable;
        self
    }

    /// [`StatusTone`] of the item's strip; default [`StatusTone::Default`].
    pub fn tone(mut self, tone: StatusTone) -> Self {
        self.tone = tone;
        self
    }

    /// App-chosen strip and text colors, overriding the [`tone`](Self::tone) (e.g. a warning tone).
    pub fn colors(mut self, container: Color32, content: Color32) -> Self {
        self.colors = Some(StatusColors { container, content });
        self
    }

    /// Start items only: the one that overflows ends in an ellipsis at the room left rather than
    /// vanishing with the following items; default `false`.
    pub fn elide(mut self, elide: bool) -> Self {
        self.elide = elide;
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

    /// Value of [`Self::clickable`].
    pub fn is_clickable(&self) -> bool {
        self.clickable
    }

    /// Value of [`Self::tone`].
    pub fn tone_kind(&self) -> StatusTone {
        self.tone
    }

    /// Value of [`Self::colors`].
    pub fn custom_colors(&self) -> Option<StatusColors> {
        self.colors
    }

    /// Value of [`Self::elide`].
    pub fn is_elided(&self) -> bool {
        self.elide
    }
}

impl<S: Into<String>> From<S> for StatusItem {
    fn from(text: S) -> Self {
        Self::new(text)
    }
}

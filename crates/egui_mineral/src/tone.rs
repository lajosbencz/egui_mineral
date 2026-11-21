use egui::Color32;

/// Semantic color treatment of a status item, bar or table row; each stylekit maps it to its roles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Tone {
    /// Colors of the surface it is on.
    #[default]
    Default,
    /// Error colors.
    Error,
    /// App-chosen colors, e.g. a warning tone.
    Custom {
        /// Behind the content.
        container: Color32,
        /// Text, icon and indicator.
        content: Color32,
    },
}

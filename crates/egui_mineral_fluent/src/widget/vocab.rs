/// Fluent size family (`size` prop) of buttons, links, labels and most controls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Size {
    /// `small`.
    Small,
    /// `medium`.
    #[default]
    Medium,
    /// `large`.
    Large,
}

impl Size {
    /// Every variant, smallest first.
    pub const ALL: [Self; 3] = [Self::Small, Self::Medium, Self::Large];
}

/// Side of the label an icon sits on (`iconPosition`); mirrored in right-to-left locales.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum IconPosition {
    /// `before`: leading.
    #[default]
    Before,
    /// `after`: trailing.
    After,
}

/// Button `appearance` of every button kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonAppearance {
    /// `secondary`: neutral fill with a stroke.
    #[default]
    Secondary,
    /// `primary`: brand fill.
    Primary,
    /// `outline`: transparent fill with a stroke.
    Outline,
    /// `subtle`: no fill until hovered.
    Subtle,
    /// `transparent`: no fill; brand content when hovered.
    Transparent,
}

impl ButtonAppearance {
    /// Every variant.
    pub const ALL: [Self; 5] = [
        Self::Secondary,
        Self::Primary,
        Self::Outline,
        Self::Subtle,
        Self::Transparent,
    ];
}

/// Button `shape` of every button kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonShape {
    /// `rounded`: `borderRadiusMedium`.
    #[default]
    Rounded,
    /// `circular`: pill.
    Circular,
    /// `square`: no rounding.
    Square,
}

impl ButtonShape {
    /// Every variant.
    pub const ALL: [Self; 3] = [Self::Rounded, Self::Circular, Self::Square];
}

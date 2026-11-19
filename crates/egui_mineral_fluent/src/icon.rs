/// Fluent UI System Icon: the regular and filled glyph of the resizable font ([`crate::icons`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FluentIcon {
    regular: char,
    filled: char,
}

impl FluentIcon {
    /// Glyphs at the codepoints of the installed icon font.
    pub const fn new(regular: char, filled: char) -> Self {
        Self { regular, filled }
    }

    /// Outlined form.
    pub const fn regular(self) -> char {
        self.regular
    }

    /// Filled form, shown on hover and when checked (`bundleIcon`).
    pub const fn filled(self) -> char {
        self.filled
    }

    /// [`Self::filled`] when `filled`, else [`Self::regular`].
    pub const fn glyph(self, filled: bool) -> char {
        if filled { self.filled } else { self.regular }
    }
}

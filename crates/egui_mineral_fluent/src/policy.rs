use egui::CursorIcon;
use egui_mineral::{
    layout::TargetPolicy,
    policy::{Cursors, Policies},
};

// react-button useSplitButtonStyles.styles.ts MIN_TARGET_SIZE (WCAG 2.2 target size minimum)
const MOUSE_TARGET: f32 = 24.0;
// No Fluent web value; the core baseline touch target.
const TOUCH_TARGET: f32 = 48.0;

// No Fluent web breakpoint tokens; the core baseline breakpoints.
pub(crate) fn policies(os: egui::os::OperatingSystem) -> Policies {
    let mut p = Policies::baseline();
    p.target = TargetPolicy::baseline().mouse(MOUSE_TARGET).touch(TOUCH_TARGET);
    p.readline = os.is_mac();
    // No mnemonic convention on Apple platforms.
    p.mnemonics = !os.is_mac();
    // react-button useButtonStyles.styles.ts `cursor: pointer` / `not-allowed`
    p.cursors = Cursors::default()
        .enabled(CursorIcon::PointingHand)
        .disabled(CursorIcon::NotAllowed);
    p
}

#[cfg(test)]
mod tests {
    #[test]
    fn policies_set_the_fluent_values() {
        let p = super::policies(egui::os::OperatingSystem::Windows);
        assert_eq!(p.target.mouse, super::MOUSE_TARGET);
        assert!(p.mnemonics);
        assert_eq!(p.cursors.disabled, Some(egui::CursorIcon::NotAllowed));
    }
}

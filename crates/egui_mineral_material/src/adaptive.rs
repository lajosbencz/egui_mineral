use std::time::Duration;

use crate::Material;
use egui::{Align, CursorIcon, vec2};
use egui_mineral::{
    input::{GestureConfig, Repeat},
    layout::{self, Flow, FlowItems, MinTarget, TargetPolicy},
    motion::Fling,
    policy::{Cursors, Policies},
    runtime::Core,
    widget::MineralUi,
};

pub(crate) const WINDOW_CLASS_BREAKPOINTS: [f32; 4] = [600.0, 840.0, 1200.0, 1600.0];

// Density-independent, like `MIN_INTERACTIVE`.
pub(crate) const MIN_INTERACTIVE_DP: f32 = 48.0;

pub(crate) const TARGETS: TargetPolicy = TargetPolicy {
    mouse: 24.0,
    touch: MIN_INTERACTIVE_DP,
};

/// Compose `Modifier.minimumInteractiveComponentSize` at the default
/// `LocalMinimumInteractiveComponentSize` (48dp): reserved on every platform by the interactive
/// components Compose applies it to.
pub const MIN_INTERACTIVE: MinTarget = MinTarget::Points(MIN_INTERACTIVE_DP);

// https://developer.android.com/design/ui/mobile/guides/layout-and-content/grids-and-units#baseline-grid
const GRID: f32 = 4.0;

/// `steps` units of the 4dp baseline grid, in dp.
pub const fn grid(steps: u8) -> f32 {
    GRID * steps as f32
}

// Compose exponentialDecay friction, 4.2 /s
const FLING_DECELERATION: f32 = 0.995_809;
// Compose SwipeToDismissBox positional threshold, 56dp
const DISMISS_FRACTION: f32 = 0.5;
// Compose DismissVelocityThreshold, 125dp/s
const DISMISS_VELOCITY: f32 = 125.0;
const REPEAT: Repeat = Repeat {
    delay: Duration::from_millis(400),
    interval: Duration::from_millis(50),
};
const GESTURES: GestureConfig = GestureConfig {
    slop: 8.0,
    long_press: Duration::from_millis(500),
    swipe_min_velocity: 600.0,
};
// No MD3 or Compose value
const TYPEAHEAD_TIMEOUT: Duration = Duration::from_secs(1);

/// Material window size classes by width in dp.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum WindowClass {
    /// Below 600dp.
    Compact,
    /// 600dp up to 840dp.
    Medium,
    /// 840dp up to 1200dp.
    Expanded,
    /// 1200dp up to 1600dp.
    Large,
    /// 1600dp and wider.
    ExtraLarge,
}

impl WindowClass {
    /// Class of the content rect width, by the breakpoint policy.
    pub fn of(core: &Core) -> Self {
        match layout::breakpoint(core, core.ctx().content_rect().width()) {
            0 => Self::Compact,
            1 => Self::Medium,
            2 => Self::Expanded,
            3 => Self::Large,
            _ => Self::ExtraLarge,
        }
    }
}

pub(crate) fn policies(os: egui::os::OperatingSystem) -> Policies {
    let mut p = Policies::baseline();
    p.target = TARGETS;
    p.breakpoints = WINDOW_CLASS_BREAKPOINTS.into();
    p.fling = Fling {
        deceleration: FLING_DECELERATION,
        dismiss_fraction: DISMISS_FRACTION,
        dismiss_velocity: DISMISS_VELOCITY,
    };
    p.repeat = REPEAT;
    p.gestures = GESTURES;
    p.typeahead = TYPEAHEAD_TIMEOUT;
    p.readline = os.is_mac();
    // No mnemonic convention on Apple platforms.
    p.mnemonics = !os.is_mac();
    p.cursors = Cursors {
        enabled: Some(CursorIcon::PointingHand),
        disabled: Some(CursorIcon::NotAllowed),
    };
    p
}

/// Compose `VisualDensity` step: 0 or negative, 4dp per step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Density(i8);

impl Density {
    /// No density step; spec sizes.
    pub const DEFAULT: Self = Self(0);
    /// Densest step, -24dp; components clamp at their content minimum before it.
    pub const MIN: Self = Self(-6);
    /// Step where fixed-size components stop shrinking (Compose `VisualDensity.Compact`).
    pub(crate) const COMPACT_FLOOR: Self = Self(-3);
    /// Least dense step; equals [`Self::DEFAULT`].
    pub const MAX: Self = Self::DEFAULT;

    /// Density from `step`; clamped to `MIN..=MAX`.
    #[track_caller]
    pub fn new(step: i8) -> Self {
        let valid = step.clamp(Self::MIN.0, Self::MAX.0);
        if valid != step {
            egui_mineral::kit::sanitize::report(format_args!("density must be in -6..=0, got {step}"));
        }
        Self(valid)
    }

    /// 0 or negative; each step is 4dp.
    pub const fn step(self) -> i8 {
        self.0
    }

    /// `size` shrunk by this density, at most down to `content` (never above `size`).
    pub fn dense_to(self, size: f32, content: f32) -> f32 {
        (size + 4.0 * f32::from(self.0)).max(content.min(size))
    }

    /// `size` shrunk by this density, at most down to `min`.
    pub fn dense(self, size: f32, min: Self) -> f32 {
        size + 4.0 * f32::from(self.max(min).step())
    }
}

fn flow(align: Align) -> Flow {
    let gap = grid(2);
    Flow::new(vec2(gap, gap), align)
}

/// Wrapping row, items centered on the cross axis.
#[track_caller]
pub fn wrapping_row<R>(ui: &mut MineralUi<'_, Material>, add: impl FnOnce(&mut FlowItems<'_, Material>) -> R) -> R {
    flow(Align::Center).show(ui, add)
}

/// Wrapping row, item tops aligned.
#[track_caller]
pub fn wrapping_row_top<R>(ui: &mut MineralUi<'_, Material>, add: impl FnOnce(&mut FlowItems<'_, Material>) -> R) -> R {
    flow(Align::Min).show(ui, add)
}

#[cfg(test)]
mod tests {
    #[test]
    fn policies_set_every_design_value() {
        let p = super::policies(egui::os::OperatingSystem::Nix);
        assert_eq!(p.target, super::TARGETS);
        assert_eq!(&*p.breakpoints, super::WINDOW_CLASS_BREAKPOINTS);
        assert_eq!(p.gestures, super::GESTURES);
    }
}

use egui::{Key, Modifiers, Pos2, Vec2};

/// Pointer or touch identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PointerId(pub u64);

impl PointerId {
    /// Mouse and the first touch; the only id the IAL emits.
    pub const PRIMARY: Self = Self(0);
}

/// Pointer device class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PointerKind {
    /// Mouse or trackpad; also the default modality before any input.
    Mouse,
    /// Touch screen; carries `pressure` where reported.
    Touch,
    /// Agent-injected raw input (see [`crate::agent`]).
    Synthetic,
}

/// Pointer sample; `time` is seconds, same clock as `egui::InputState::time`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PointerState {
    /// Pointer identity across down, move and up.
    pub id: PointerId,
    /// Device class of the sample.
    pub kind: PointerKind,
    /// Screen points.
    pub pos: Pos2,
    /// Seconds on the egui input clock.
    pub time: f64,
    /// 0.0..=1.0 where the platform reports touch force.
    pub pressure: Option<f32>,
}

/// Dominant axis and sign of a swipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SwipeDirection {
    /// Negative x dominates.
    Left,
    /// Positive x dominates.
    Right,
    /// Negative y dominates.
    Up,
    /// Positive y dominates.
    Down,
}

/// Recognized touch or pointer gesture.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Gesture {
    /// Press and release within slop, shorter than `GestureConfig::long_press`.
    Tap {
        /// Press position.
        pos: Pos2,
    },
    /// Press held within slop for `GestureConfig::long_press`; fires once, while still held.
    LongPress {
        /// Press position.
        pos: Pos2,
    },
    /// Pan released at or above `GestureConfig::swipe_min_velocity`.
    Swipe {
        /// Press position.
        start: Pos2,
        /// Release position.
        end: Pos2,
        /// Release velocity in points/s.
        velocity: Vec2,
        /// Dominant axis and sign of `velocity`.
        direction: SwipeDirection,
    },
    /// `zoom_delta` is multiplicative per pass.
    Pinch {
        /// Screen points.
        center: Pos2,
        /// Multiplicative zoom factor of this pass; 1.0 is no change.
        zoom_delta: f32,
    },
    /// Primary-pointer move beyond slop while pressed.
    Pan {
        /// Current pointer position.
        pos: Pos2,
        /// Travel since the previous sample, in points.
        delta: Vec2,
    },
}

impl Gesture {
    /// Point the gesture is aimed at: the press for taps, long presses and swipes, the center of a
    /// pinch, the current position of a pan.
    #[must_use]
    pub const fn anchor(&self) -> Pos2 {
        match *self {
            Self::Tap { pos } | Self::LongPress { pos } | Self::Pan { pos, .. } => pos,
            Self::Swipe { start, .. } => start,
            Self::Pinch { center, .. } => center,
        }
    }
}

/// Primary-pointer pan as seen by one pass, from press origin to release.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PanFrame {
    /// Press position.
    pub origin: Pos2,
    /// Current pointer position.
    pub pos: Pos2,
    /// Travel during this pass.
    pub delta: Vec2,
    /// Points/s; zero when the pointer rests or the pan ends cancelled.
    pub velocity: Vec2,
    /// Released (or cancelled) during this pass; the last frame of the pan.
    pub ended: bool,
}

/// Key press, repeat or release with modifiers.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct KeyAction {
    /// Logical key.
    pub key: Key,
    /// `false` on release.
    pub pressed: bool,
    /// OS auto-repeat of a held key.
    pub repeat: bool,
    /// Modifiers held at the time of the event.
    pub modifiers: Modifiers,
}

/// Pointer, touch and key input normalized from egui events by the IAL.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum UnifiedInputEvent {
    /// Hover or drag move.
    PointerMoved(PointerState),
    /// Primary button or touch down.
    PointerDown(PointerState),
    /// Primary button or touch up.
    PointerUp(PointerState),
    /// Recognized by the [`GestureRecognizer`](super::GestureRecognizer).
    Gesture(Gesture),
    /// Key press, repeat or release.
    KeyAction(KeyAction),
}

impl UnifiedInputEvent {
    /// Screen point the event is aimed at; `None` for keys.
    #[must_use]
    pub const fn anchor(&self) -> Option<Pos2> {
        match self {
            Self::PointerMoved(p) | Self::PointerDown(p) | Self::PointerUp(p) => Some(p.pos),
            Self::Gesture(g) => Some(g.anchor()),
            Self::KeyAction(_) => None,
        }
    }
}

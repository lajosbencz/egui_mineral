use egui_mineral::motion::Spring;

use crate::spec::tokens::standard_motion as m;

// Compose MotionScheme.standard()
pub(crate) const DEFAULT_EFFECTS: Spring = Spring {
    damping_ratio: m::SPRING_DEFAULT_EFFECTS_DAMPING,
    stiffness: m::SPRING_DEFAULT_EFFECTS_STIFFNESS,
    rest_threshold: REST,
};
pub(crate) const FAST_EFFECTS: Spring = Spring {
    damping_ratio: m::SPRING_FAST_EFFECTS_DAMPING,
    stiffness: m::SPRING_FAST_EFFECTS_STIFFNESS,
    rest_threshold: REST,
};
pub(crate) const SLOW_EFFECTS: Spring = Spring {
    damping_ratio: m::SPRING_SLOW_EFFECTS_DAMPING,
    stiffness: m::SPRING_SLOW_EFFECTS_STIFFNESS,
    rest_threshold: REST,
};
pub(crate) const DEFAULT_SPATIAL: Spring = Spring {
    damping_ratio: m::SPRING_DEFAULT_SPATIAL_DAMPING,
    stiffness: m::SPRING_DEFAULT_SPATIAL_STIFFNESS,
    rest_threshold: REST,
};
pub(crate) const FAST_SPATIAL: Spring = Spring {
    damping_ratio: m::SPRING_FAST_SPATIAL_DAMPING,
    stiffness: m::SPRING_FAST_SPATIAL_STIFFNESS,
    rest_threshold: REST,
};

// Compose Spring.DefaultDisplacementThreshold, points
pub(crate) const REST: f32 = 0.01;

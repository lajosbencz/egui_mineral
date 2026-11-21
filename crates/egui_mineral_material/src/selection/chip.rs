use crate::IconSet;
use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::actions::button::Surface;
use crate::msg;
use crate::widget::common::elevation_tokens;
use egui_mineral::i18n::I18n;
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use std::sync::Arc;

use egui::{Color32, CornerRadius, Galley, Rangef, Rect, Sense, Stroke, pos2, vec2};
use egui_mineral::widget::Availability;
use egui_mineral::{
    kit,
    layout::{Constraints, Measure, MinTarget},
    select::Check,
    semantic::{self, Role, SemanticNode},
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::actions::button::Elevation;
use crate::widget::{
    common::{self},
    indication, parts,
};
use crate::{
    Icon,
    spec::{
        ColorRole,
        defaults::chip::{CONTENT_SPACE, INPUT_EDGE_SPACE, SPACE},
        tokens::{assist_chip, chips, filter_chip, input_chip, suggestion_chip},
    },
    tokens::Tokens,
};

/// Container of an assist, filter or suggestion [`Chip`] (Compose `*Chip` and `Elevated*Chip`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ChipVariant {
    /// Outlined container without a shadow.
    #[default]
    Flat,
    /// Filled container with a shadow, no outline.
    Elevated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChipKind {
    Assist,
    Filter,
    Input,
    Suggestion,
}

#[derive(Clone, Copy)]
struct Style {
    container: Color32,
    label: Color32,
    leading: Color32,
    trailing: Color32,
    border: Option<Stroke>,
    elevation: Elevation,
    ripple: Color32,
}

#[derive(Debug)]
struct Core<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    kind: ChipKind,
    icon: Option<Icon>,
    selected: Option<bool>,
    checked: Option<&'a mut bool>,
    variant: ChipVariant,
    disabled: bool,
    min_target: MinTarget,
}

/// Events of [`InputChip`], read from `WidgetResponse::output`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct InputChipOutput {
    /// Trailing close icon clicked.
    pub removed: bool,
}

/// Setters of every chip type, a tuple struct over [`Core`].
macro_rules! common_setters {
    () => {
        /// Leading icon.
        pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
            self.0.icon = Some(icon.into());
            self
        }

        crate::widget::setters! {
            /// Greys out the chip and ignores input.
            disabled |s| s.0.disabled
        }

        crate::widget::setters! {
            /// Replaces the `<id_salt>-chip` agent key.
            agent_key(key) |s| s.0.key
        }
    };
}

/// Forwards [`MineralWidget`] of a chip type to its [`Core`].
macro_rules! delegate_widget {
    ($ty:ty, $out:ty, $map:expr) => {
        impl MineralWidget for $ty {
            type Kit = Material;
            type State = ();
            type Output = $out;

            egui_mineral::kit::delegate_widget_meta!(0);

            fn layout(&self, ui: &MineralUi<'_, Material>, c: Constraints, state: &Self::State) -> Measure {
                self.0.layout(ui, c, state)
            }

            fn update(
                &mut self,
                ui: &mut MineralUi<'_, Material>,
                frame: &WidgetFrame<'_>,
                state: &mut (),
            ) -> WidgetResponse<$out> {
                let r = self.0.update(ui, frame, state);
                let changed = r.changed();
                let (response, out) = r.into_parts();
                WidgetResponse::new(response, $map(out)).with_changed(changed)
            }

            fn semantics(&self, _: &I18n, _: &WidgetResponse<$out>, (): &()) -> Option<SemanticNode> {
                Some(self.0.node())
            }
        }
    };
}

/// Assist, filter and suggestion chips. Filter chips report `changed` when flipped; the other
/// kinds use `clicked()`. Input chips are [`InputChip`].
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Chip<'a>(Core<'a>);

impl<'a> Chip<'a> {
    fn new(id_salt: impl Into<KeyedSalt>, kind: ChipKind, label: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self(Core::new(id_salt, kind, label))
    }

    /// `AssistChip`: click-only; read `.clicked()`. Agent key `<id_salt>-chip`.
    pub fn assist(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self::new(id_salt, ChipKind::Assist, label)
    }

    /// Flips `selected` on a body click and reports `changed`.
    pub fn filter(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, selected: &'a mut bool) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        let mut chip = Self::new(id_salt, ChipKind::Filter, label);
        chip.0.selected = Some(*selected);
        chip.0.checked = Some(selected);
        chip
    }

    /// `SuggestionChip`: click-only; read `.clicked()`. Agent key `<id_salt>-chip`.
    pub fn suggestion(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self::new(id_salt, ChipKind::Suggestion, label)
    }

    /// Removable chip: the trailing close icon reports [`InputChipOutput::removed`]. Agent key `<id_salt>-chip`.
    pub fn input(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> InputChip {
        let id_salt: KeyedSalt = id_salt.into();
        InputChip::new(id_salt, label)
    }

    /// Default [`ChipVariant::Flat`].
    pub fn variant(mut self, variant: ChipVariant) -> Self {
        self.0.variant = variant;
        self
    }

    common_setters!();
}

delegate_widget!(Chip<'_>, (), |_: InputChipOutput| ());

/// Removable chip; `changed` reports a body activation, the caller owns the selection shown by
/// [`InputChip::selected`].
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct InputChip(Core<'static>);

impl InputChip {
    fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self(Core::new(id_salt, ChipKind::Input, label))
    }

    /// Display selection; `false` until set.
    pub fn selected(mut self, selected: bool) -> Self {
        self.0.selected = Some(selected);
        self
    }

    pub(crate) fn target(mut self, target: MinTarget) -> Self {
        self.0.min_target = target;
        self
    }

    common_setters!();
}

delegate_widget!(InputChip, InputChipOutput, std::convert::identity);

impl Core<'_> {
    fn new(id_salt: impl Into<KeyedSalt>, kind: ChipKind, label: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            key: egui_mineral::agent_key!("chip"),
            salt: id_salt,
            label: label.into(),
            kind,
            icon: None,
            selected: None,
            checked: None,
            variant: ChipVariant::Flat,
            disabled: false,
            min_target: crate::adaptive::MIN_INTERACTIVE,
        }
    }

    fn is_selected(&self) -> bool {
        self.selected == Some(true)
    }

    fn has_leading(&self) -> bool {
        self.icon.is_some() || (self.kind == ChipKind::Filter && self.is_selected())
    }

    fn leading(&self, icons: &IconSet) -> Option<Icon> {
        if self.kind == ChipKind::Filter && self.is_selected() {
            Some(icons.check)
        } else {
            self.icon
        }
    }

    fn has_remove(&self) -> bool {
        self.kind == ChipKind::Input
    }

    fn padding(&self) -> (f32, f32) {
        if self.kind != ChipKind::Input {
            return (SPACE, SPACE);
        }
        let start = if self.has_leading() { SPACE } else { INPUT_EDGE_SPACE };
        let end = if self.has_remove() { SPACE } else { INPUT_EDGE_SPACE };
        (start, end)
    }

    fn shape(&self) -> crate::spec::ShapeRole {
        match self.kind {
            ChipKind::Assist => assist_chip::CONTAINER_SHAPE,
            ChipKind::Filter => filter_chip::CONTAINER_SHAPE,
            ChipKind::Input => input_chip::CONTAINER_SHAPE,
            ChipKind::Suggestion => suggestion_chip::CONTAINER_SHAPE,
        }
    }

    fn icon_size(&self) -> f32 {
        match self.kind {
            ChipKind::Assist => assist_chip::ICON_SIZE,
            ChipKind::Filter => filter_chip::ICON_SIZE,
            ChipKind::Input => input_chip::LEADING_ICON_SIZE,
            ChipKind::Suggestion => suggestion_chip::LEADING_ICON_SIZE,
        }
    }

    fn galley(&self, ui: &MineralUi<'_, Material>, color: Color32) -> Arc<Galley> {
        let rt = ui.rt();
        parts::type_galley(rt.core(), ui.painter(), &self.label, chips::LABEL_TEXT, color, None)
    }

    fn width(&self, label: f32) -> f32 {
        let (start, end) = self.padding();
        let lead = if self.has_leading() { self.icon_size() } else { 0.0 };
        let trail = if self.has_remove() {
            input_chip::TRAILING_ICON_SIZE
        } else {
            0.0
        };
        start + lead + CONTENT_SPACE + label + CONTENT_SPACE + trail + end
    }

    fn style(&self, t: &Tokens) -> Style {
        let p = Palette {
            t,
            enabled: !self.disabled,
            on: self.is_selected(),
        };
        let (on, enabled) = (p.on, p.enabled);
        let mut s = match (self.kind, self.variant == ChipVariant::Elevated) {
            (ChipKind::Assist, false) => assist_flat_style(&p),
            (ChipKind::Assist, true) => assist_elevated_style(&p),
            (ChipKind::Suggestion, elevated) => suggestion_style(&p, elevated),
            (ChipKind::Filter, elevated) => filter_style(&p, elevated),
            (ChipKind::Input, _) => input_style(&p),
        };
        // `Surface` content color: the selected container's `On*` role, else the ambient `OnSurface`.
        s.ripple = p.t.role(if on {
            ColorRole::OnSecondaryContainer
        } else {
            ColorRole::OnSurface
        });
        if !enabled {
            let (label, icon) = self.disabled_content();
            s.label = p.t.role_alpha(label.0, label.1);
            s.leading = p.t.role_alpha(icon.0, icon.1);
            s.trailing = p.t.role_alpha(icon.0, icon.1);
        }
        s
    }

    fn disabled_content(&self) -> ((ColorRole, f32), (ColorRole, f32)) {
        match self.kind {
            ChipKind::Assist => (
                (
                    assist_chip::DISABLED_LABEL_TEXT_COLOR,
                    assist_chip::DISABLED_LABEL_TEXT_OPACITY,
                ),
                (assist_chip::DISABLED_ICON_COLOR, assist_chip::DISABLED_ICON_OPACITY),
            ),
            ChipKind::Filter => (
                (
                    filter_chip::DISABLED_LABEL_TEXT_COLOR,
                    filter_chip::DISABLED_LABEL_TEXT_OPACITY,
                ),
                (
                    filter_chip::DISABLED_LEADING_ICON_COLOR,
                    filter_chip::DISABLED_LEADING_ICON_OPACITY,
                ),
            ),
            ChipKind::Input => (
                (
                    input_chip::DISABLED_LABEL_TEXT_COLOR,
                    input_chip::DISABLED_LABEL_TEXT_OPACITY,
                ),
                (
                    input_chip::DISABLED_LEADING_ICON_COLOR,
                    input_chip::DISABLED_LEADING_ICON_OPACITY,
                ),
            ),
            ChipKind::Suggestion => (
                (
                    suggestion_chip::DISABLED_LABEL_TEXT_COLOR,
                    suggestion_chip::DISABLED_LABEL_TEXT_OPACITY,
                ),
                (
                    suggestion_chip::DISABLED_LEADING_ICON_COLOR,
                    suggestion_chip::DISABLED_LEADING_ICON_OPACITY,
                ),
            ),
        }
    }

    fn paint_remove(
        &self,
        ui: &MineralUi<'_, Material>,
        rm: &egui::Response,
        center: egui::Pos2,
        color: Color32,
        t: &Tokens,
    ) {
        let rt = ui.rt();
        let ctx = ui.ctx();
        // The largest chip slot (`AVATAR_SIZE`) bounds the remove target's state layer.
        let part = parts::IconPart {
            icon: ui.icons().close,
            icon_size: input_chip::TRAILING_ICON_SIZE,
            layer: egui::Vec2::splat(chips::AVATAR_SIZE),
            radius: CornerRadius::from(chips::AVATAR_SIZE / 2.0),
            color,
            availability: Availability::disabled_if(self.disabled),
        };
        parts::paint_icon_part(ui, rm, center, &part, t);
        semantic::emit(ctx, Some(rm.id), rm.rect, || {
            let name = rt.i18n().tr(msg::ChipRemove {
                label: (&self.label).into(),
            });

            SemanticNode::new(self.key.id(&self.salt).sub("remove"), Role::Button, name)
                .clickable(Availability::disabled_if(self.disabled))
        });
    }
}

struct Palette<'a> {
    t: &'a Tokens,
    enabled: bool,
    on: bool,
}

fn assist_flat_style(p: &Palette<'_>) -> Style {
    use assist_chip as m;
    let enabled = p.enabled;
    let none = Color32::TRANSPARENT;
    let flat = Elevation::flat;
    Style {
        container: none,
        label: p.t.role(m::LABEL_TEXT_COLOR),
        leading: p.t.role(m::ICON_COLOR),
        trailing: p.t.role(m::ICON_COLOR),
        border: Some(Stroke::new(
            m::FLAT_OUTLINE_WIDTH,
            if enabled {
                p.t.role(m::FLAT_OUTLINE_COLOR)
            } else {
                p.t.role_alpha(m::FLAT_DISABLED_OUTLINE_COLOR, m::FLAT_DISABLED_OUTLINE_OPACITY)
            },
        )),
        elevation: flat(m::FLAT_CONTAINER_ELEVATION),
        ripple: none,
    }
}

fn assist_elevated_style(p: &Palette<'_>) -> Style {
    use assist_chip as m;
    let enabled = p.enabled;
    let none = Color32::TRANSPARENT;
    Style {
        container: if enabled {
            p.t.role(m::ELEVATED_CONTAINER_COLOR)
        } else {
            p.t.role_alpha(
                m::ELEVATED_DISABLED_CONTAINER_COLOR,
                m::ELEVATED_DISABLED_CONTAINER_OPACITY,
            )
        },
        label: p.t.role(m::LABEL_TEXT_COLOR),
        leading: p.t.role(m::ICON_COLOR),
        trailing: p.t.role(m::ICON_COLOR),
        border: None,
        elevation: elevation_tokens!(m; rest: ELEVATED_CONTAINER_ELEVATION, hovered: ELEVATED_HOVER_CONTAINER_ELEVATION, focused: ELEVATED_FOCUS_CONTAINER_ELEVATION, pressed: ELEVATED_PRESSED_CONTAINER_ELEVATION, disabled: ELEVATED_DISABLED_CONTAINER_ELEVATION),
        ripple: none,
    }
}

fn suggestion_style(p: &Palette<'_>, elevated: bool) -> Style {
    use suggestion_chip as m;
    let enabled = p.enabled;
    let none = Color32::TRANSPARENT;
    let flat = Elevation::flat;
    Style {
        container: match (elevated, enabled) {
            (false, _) => none,
            (true, true) => p.t.role(m::ELEVATED_CONTAINER_COLOR),
            // Compose takes the assist chip opacity.
            (true, false) => p.t.role_alpha(
                m::ELEVATED_DISABLED_CONTAINER_COLOR,
                assist_chip::ELEVATED_DISABLED_CONTAINER_OPACITY,
            ),
        },
        label: p.t.role(m::LABEL_TEXT_COLOR),
        leading: p.t.role(m::LEADING_ICON_COLOR),
        trailing: p.t.role(m::LEADING_ICON_COLOR),
        border: (!elevated).then(|| {
            Stroke::new(
                m::FLAT_OUTLINE_WIDTH,
                if enabled {
                    p.t.role(m::FLAT_OUTLINE_COLOR)
                } else {
                    p.t.role_alpha(m::FLAT_DISABLED_OUTLINE_COLOR, m::FLAT_DISABLED_OUTLINE_OPACITY)
                },
            )
        }),
        elevation: if elevated {
            elevation_tokens!(m; rest: ELEVATED_CONTAINER_ELEVATION, hovered: ELEVATED_HOVER_CONTAINER_ELEVATION, focused: ELEVATED_FOCUS_CONTAINER_ELEVATION, pressed: ELEVATED_PRESSED_CONTAINER_ELEVATION, disabled: ELEVATED_DISABLED_CONTAINER_ELEVATION)
        } else {
            flat(m::FLAT_CONTAINER_ELEVATION)
        },
        ripple: none,
    }
}

fn filter_style(p: &Palette<'_>, elevated: bool) -> Style {
    use filter_chip as m;
    let (on, enabled) = (p.on, p.enabled);
    let none = Color32::TRANSPARENT;
    let container = match (elevated, on, enabled) {
        (false, false, _) => none,
        (false, true, true) => p.t.role(m::FLAT_SELECTED_CONTAINER_COLOR),
        (false, true, false) => p.t.role_alpha(
            m::FLAT_DISABLED_SELECTED_CONTAINER_COLOR,
            m::FLAT_DISABLED_SELECTED_CONTAINER_OPACITY,
        ),
        (true, false, true) => p.t.role(m::ELEVATED_UNSELECTED_CONTAINER_COLOR),
        (true, true, true) => p.t.role(m::ELEVATED_SELECTED_CONTAINER_COLOR),
        (true, _, false) => p.t.role_alpha(
            m::ELEVATED_DISABLED_CONTAINER_COLOR,
            m::ELEVATED_DISABLED_CONTAINER_OPACITY,
        ),
    };
    Style {
        container,
        label: p.t.role(if on {
            m::SELECTED_LABEL_TEXT_COLOR
        } else {
            m::UNSELECTED_LABEL_TEXT_COLOR
        }),
        leading: p.t.role(if on {
            m::SELECTED_LEADING_ICON_COLOR
        } else {
            m::UNSELECTED_LEADING_ICON_COLOR
        }),
        trailing: p.t.role(if on {
            m::SELECTED_TRAILING_ICON_COLOR
        } else {
            m::UNSELECTED_TRAILING_ICON_COLOR
        }),
        border: (!elevated && !on).then(|| {
            Stroke::new(
                m::FLAT_UNSELECTED_OUTLINE_WIDTH,
                if enabled {
                    p.t.role(m::FLAT_UNSELECTED_OUTLINE_COLOR)
                } else {
                    p.t.role_alpha(
                        m::FLAT_DISABLED_UNSELECTED_OUTLINE_COLOR,
                        m::FLAT_DISABLED_UNSELECTED_OUTLINE_OPACITY,
                    )
                },
            )
        }),
        // `filterChipElevation` takes the selected flat values for both states.
        elevation: if elevated {
            elevation_tokens!(m; rest: ELEVATED_CONTAINER_ELEVATION, hovered: ELEVATED_HOVER_CONTAINER_ELEVATION, focused: ELEVATED_FOCUS_CONTAINER_ELEVATION, pressed: ELEVATED_PRESSED_CONTAINER_ELEVATION, disabled: ELEVATED_DISABLED_CONTAINER_ELEVATION)
        } else {
            elevation_tokens!(m; rest: FLAT_CONTAINER_ELEVATION, hovered: FLAT_SELECTED_HOVER_CONTAINER_ELEVATION, focused: FLAT_SELECTED_FOCUS_CONTAINER_ELEVATION, pressed: FLAT_SELECTED_PRESSED_CONTAINER_ELEVATION, disabled: FLAT_CONTAINER_ELEVATION)
        },
        ripple: none,
    }
}

fn input_style(p: &Palette<'_>) -> Style {
    use input_chip as m;
    let (on, enabled) = (p.on, p.enabled);
    let none = Color32::TRANSPARENT;
    let flat = Elevation::flat;
    Style {
        container: match (on, enabled) {
            (false, _) => none,
            (true, true) => p.t.role(m::SELECTED_CONTAINER_COLOR),
            (true, false) => p.t.role_alpha(
                m::DISABLED_SELECTED_CONTAINER_COLOR,
                m::DISABLED_SELECTED_CONTAINER_OPACITY,
            ),
        },
        label: p.t.role(if on {
            m::SELECTED_LABEL_TEXT_COLOR
        } else {
            m::UNSELECTED_LABEL_TEXT_COLOR
        }),
        leading: p.t.role(if on {
            m::SELECTED_LEADING_ICON_COLOR
        } else {
            m::UNSELECTED_LEADING_ICON_COLOR
        }),
        trailing: p.t.role(if on {
            m::SELECTED_TRAILING_ICON_COLOR
        } else {
            m::UNSELECTED_TRAILING_ICON_COLOR
        }),
        border: (!on).then(|| {
            Stroke::new(
                m::UNSELECTED_OUTLINE_WIDTH,
                if enabled {
                    p.t.role(m::UNSELECTED_OUTLINE_COLOR)
                } else {
                    p.t.role_alpha(
                        m::DISABLED_UNSELECTED_OUTLINE_COLOR,
                        m::DISABLED_UNSELECTED_OUTLINE_OPACITY,
                    )
                },
            )
        }),
        elevation: flat(m::CONTAINER_ELEVATION),
        ripple: none,
    }
}

impl MineralWidget for Core<'_> {
    type Kit = Material;
    type State = ();
    type Output = InputChipOutput;

    egui_mineral::kit::widget_meta!(salt, enabled);

    fn min_target(&self, _: &MineralUi<'_, Self::Kit>) -> MinTarget {
        self.min_target
    }

    fn layout(&self, ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        let label = self.galley(ui, Color32::PLACEHOLDER).size();
        c.constrain(vec2(
            self.width(label.x),
            ui.density()
                .dense_to(chips::HEIGHT, label.y + crate::adaptive::CONTENT_PAD),
        ))
        .into()
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        frame: &WidgetFrame<'_>,
        _state: &mut (),
    ) -> WidgetResponse<InputChipOutput> {
        let rt = ui.rt();
        let t = rt.tokens();
        let r = &frame.response;
        let rect = frame.zones.visual;
        let dir = rt.i18n().direction();
        let x = |from_start: f32| dir.start_x(rect, Rangef::point(from_start));
        let (start, end) = self.padding();
        let trail = input_chip::TRAILING_ICON_SIZE;
        let remove_center = pos2(x(rect.width() - end - trail / 2.0), rect.center().y);
        let remove_hit = Rect::from_center_size(
            remove_center,
            egui::Vec2::splat(crate::adaptive::min_interactive(ui.density())),
        );
        let remove = self.has_remove().then(|| {
            kit::interact::interact(
                ui,
                remove_hit,
                r.id.with("remove"),
                Sense::click(),
                Availability::disabled_if(self.disabled),
            )
        });
        let removed = remove.as_ref().is_some_and(egui::Response::clicked);
        let press = parts::press(ui, frame, Availability::disabled_if(self.disabled));
        let body_clicked = press.clicked;
        if body_clicked {
            parts::flip(&mut self.checked, &mut self.selected);
        }

        let s = self.style(&t);
        let i = press.interaction;
        let cr = common::radius(rect, self.shape().scale().radius(&t.shape));
        let p = ui.painter();
        let surface = Surface {
            rect,
            radius: cr,
            fill: s.container,
            elevation: s.elevation,
            outline: s.border,
        };
        surface.paint_fill(p, i, &t);
        surface.paint_outline(p);

        let mut at = start;
        if let Some(icon) = self.leading(&rt.icons()) {
            let size = self.icon_size();
            common::icon(p, pos2(x(at + size / 2.0), rect.center().y), icon, size, s.leading);
            at += size;
        }
        at += CONTENT_SPACE;
        let text = self.galley(ui, s.label);
        let left = dir.start_x(rect, Rangef::new(at, at + text.size().x));
        p.galley(pos2(left, rect.center().y - text.size().y / 2.0), text, s.label);
        indication::paint(ui, r, rect, cr, s.ripple, i, &t);

        if let Some(rm) = &remove {
            self.paint_remove(ui, rm, remove_center, s.trailing, &t);
        }
        WidgetResponse::new(r.clone(), InputChipOutput { removed })
            .with_changed(body_clicked && self.selected.is_some())
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse<InputChipOutput>, (): &()) -> Option<SemanticNode> {
        Some(self.node())
    }
}

impl Core<'_> {
    fn node(&self) -> SemanticNode {
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::Button, &self.label)
            .clickable(Availability::disabled_if(self.disabled));
        if self.selected.is_some() {
            n.state.checked = self.selected.map(Check::from);
            n.state.selected = self.selected;
        }
        n
    }
}

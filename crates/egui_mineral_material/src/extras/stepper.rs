use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::Rt;
use crate::adaptive::Density;
use crate::msg;
use egui::{Color32, Pos2, Rangef, Rect, Sense, Stroke, UiBuilder, pos2, vec2};
use egui_mineral::i18n::I18n;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::Availability;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    focus::{self, RovingFocus},
    i18n::Direction,
    kit,
    layout::{Axis, Constraints, Measure},
    semantic::{self, Action, Role, SemanticNode, StableId},
    steps::{StepStatus, Steps},
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::widget::{
    common::Interaction,
    indication, parts,
    row_parts::{self, RowColors, RowState},
};
use crate::{SelectionResponse, containment::ListLines};
use crate::{
    spec::{
        defaults,
        tokens::{divider, filled_icon_button, filled_tonal_button, list, outlined_icon_button, x_small_icon_button},
    },
    tokens::Tokens,
};

use row_parts::MARKER;
const MARKER_FONT: crate::spec::TypeScale = filled_tonal_button::LABEL_TEXT_FONT;
// List item space kept before trailing content
const MIN_CONNECTOR: f32 = defaults::list::TRAILING_CONTENT_START_PADDING;
const HEADER_BASE: f32 = defaults::list::START_PADDING + MARKER + defaults::list::END_PADDING;
const LABEL_START: f32 = defaults::list::START_PADDING + MARKER + defaults::list::LEADING_CONTENT_END_PADDING;

/// Title and optional description of one [`Stepper`] step.
#[derive(Clone, Debug)]
#[must_use]
pub struct StepLabel {
    label: String,
    description: Option<String>,
}

impl StepLabel {
    /// `label` is the step title and accessible name.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
        }
    }
    /// Secondary line below the title.
    pub fn supporting(mut self, supporting: impl Into<String>) -> Self {
        self.description = Some(supporting.into());
        self
    }
}

fn status_id(s: StepStatus) -> crate::Msg {
    match s {
        StepStatus::Pending => msg::STEP_PENDING,
        StepStatus::Active => msg::STEP_ACTIVE,
        StepStatus::Complete => msg::STEP_COMPLETE,
        StepStatus::Skipped => msg::STEP_SKIPPED,
        StepStatus::Error => msg::STEP_ERROR,
        StepStatus::Disabled => msg::STEP_DISABLED,
    }
}

fn paint_marker(ui: &MineralUi<'_, Material>, center: Pos2, status: StepStatus, number: &str, t: &Tokens) {
    let rt = ui.rt();
    let p = ui.painter();
    let r = MARKER / 2.0;
    let (fill, fg) = match status {
        StepStatus::Complete | StepStatus::Active => (
            t.role(filled_icon_button::CONTAINER_COLOR),
            t.role(filled_icon_button::COLOR),
        ),
        StepStatus::Error => (t.color.error, t.color.on_error),
        StepStatus::Pending | StepStatus::Skipped => {
            let w = x_small_icon_button::OUTLINED_OUTLINE_WIDTH;
            p.circle_stroke(
                center,
                r - w / 2.0,
                Stroke::new(w, t.role(outlined_icon_button::OUTLINE_COLOR)),
            );
            (Color32::TRANSPARENT, t.role(outlined_icon_button::COLOR))
        }
        StepStatus::Disabled => (
            t.role_alpha(
                filled_icon_button::DISABLED_CONTAINER_COLOR,
                filled_icon_button::DISABLED_CONTAINER_OPACITY,
            ),
            t.role_alpha(filled_icon_button::DISABLED_COLOR, filled_icon_button::DISABLED_OPACITY),
        ),
    };
    let glyph = match status {
        StepStatus::Complete => Some((ui.icons().check, fg)),
        StepStatus::Error => Some((ui.icons().priority_high, fg)),
        _ => None,
    };
    row_parts::marker(p, center, MARKER, fill, glyph);
    if glyph.is_none() {
        let g = parts::type_galley(rt.core(), p, number, MARKER_FONT, fg, None);
        p.galley(center - g.size() / 2.0, g, fg);
    }
}

struct StepHeader<'a> {
    salt: KeyedSalt,
    key: StableId,
    label: &'a StepLabel,
    number: String,
    status: StepStatus,
    optional: Option<String>,
    selectable: bool,
    label_w: f32,
    full_width: bool,
    height: f32,
    tab_stop: focus::TabStop,
    disabled: bool,
}

impl StepHeader<'_> {
    fn text_color(&self, col: &RowColors, t: &Tokens) -> Color32 {
        match self.status {
            StepStatus::Error => t.color.error,
            StepStatus::Active | StepStatus::Complete | StepStatus::Disabled => col.label,
            StepStatus::Pending | StepStatus::Skipped => col.supporting,
        }
    }
}

impl MineralWidget for StepHeader<'_> {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt);

    fn sense(&self) -> Sense {
        let base = if self.disabled {
            Sense::hover()
        } else if self.selectable {
            Sense::click()
        } else if self.status == StepStatus::Active {
            Sense::focusable_noninteractive()
        } else {
            Sense::hover()
        };
        focus::roving_sense(base, self.tab_stop)
    }

    fn layout(&self, _ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        let w = if self.full_width {
            c.max.x
        } else {
            HEADER_BASE
                + if self.label_w > 0.0 {
                    defaults::list::LEADING_CONTENT_END_PADDING + self.label_w
                } else {
                    0.0
                }
        };
        c.constrain(vec2(w, self.height)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let r = &frame.response;
        kit::focus::follow_press(r);
        let rect = frame.zones.visual;
        let changed = self.selectable && parts::press(ui, frame, Availability::Enabled).clicked;
        let dir = ui.i18n().direction();
        let row = RowState {
            disabled: self.status == StepStatus::Disabled,
            ..RowState::default()
        };
        let col = RowColors::of(row, &t);
        let i = Interaction::of(r, Availability::enabled_if(self.selectable));
        indication::paint(ui, r, rect, row.shape(&t, dir), col.label, i, &t);
        let x = |from_start: f32, w: f32| dir.start_x(rect, Rangef::new(from_start, from_start + w));
        paint_marker(
            ui,
            pos2(x(defaults::list::START_PADDING, MARKER) + MARKER / 2.0, rect.center().y),
            self.status,
            &self.number,
            &t,
        );
        if self.label_w > 0.0 {
            let w = if self.full_width {
                rect.width() - LABEL_START - defaults::list::END_PADDING
            } else {
                self.label_w
            };
            let p = ui.painter();
            let main = parts::type_wrapped(
                p,
                &self.label.label,
                list::ITEM_LABEL_TEXT_FONT,
                self.text_color(&col, &t),
                w,
                1,
            );
            let caption = self
                .label
                .description
                .as_ref()
                .or(self.optional.as_ref())
                .map(|s| parts::type_wrapped(p, s, list::ITEM_SUPPORTING_TEXT_FONT, col.supporting, w, 1));
            let total = main.size().y + caption.as_ref().map_or(0.0, |g| g.size().y);
            let mut y = rect.center().y - total / 2.0;
            for g in std::iter::once(main).chain(caption) {
                let h = g.size().y;
                p.galley(pos2(x(LABEL_START, g.size().x), y), g, Color32::PLACEHOLDER);
                y += h;
            }
        }
        WidgetResponse::new(r.clone(), ()).with_changed(changed)
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let name = i18n.tr(msg::Step {
            number: (&self.number).into(),
            label: (&self.label.label).into(),
        });
        let mut n =
            SemanticNode::new(self.key.clone(), Role::Button, name).value(crate::tr(i18n, status_id(self.status)));
        n.state.selected = Some(self.status == StepStatus::Active);
        n.state.invalid = self.status == StepStatus::Error;
        n.state.disabled = self.disabled || (!self.selectable && self.status != StepStatus::Active);
        if self.selectable {
            n.actions.extend([Action::Click, Action::Focus]);
        }
        Some(n)
    }
}

/// Step progress over a [`Steps`] model; non-MD3 (MUI `Stepper` pattern).
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct Stepper<'a> {
    salt: KeyedSalt,
    label: Option<String>,
    steps: &'a mut Steps,
    labels: Vec<StepLabel>,
    axis: Axis,
    disabled: bool,
}

impl<'a> Stepper<'a> {
    /// `steps` is the bound step model; agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, steps: &'a mut Steps) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            label: None,
            steps,
            labels: Vec::new(),
            axis: Axis::Horizontal,
            disabled: false,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Accessible name of the group; default `mineral-stepper`.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Appends the label of the next step.
    pub fn step(mut self, label: impl Into<StepLabel>) -> Self {
        self.labels.push(label.into());
        self
    }

    /// Appends the labels of the next steps.
    pub fn steps(mut self, labels: impl IntoIterator<Item = impl Into<StepLabel>>) -> Self {
        self.labels.extend(labels.into_iter().map(Into::into));
        self
    }

    /// Default [`Axis::Horizontal`].
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    crate::widget::setters! {
        /// Greys out the step headers; steps cannot be picked.
        disabled
    }

    fn header_h(&self, density: Density, i: usize) -> f32 {
        let lines = if self.steps.is_optional(i) || self.labels[i].description.is_some() {
            ListLines::Two
        } else {
            ListLines::One
        };
        row_parts::height(density, lines)
    }

    fn header(&self, rt: &Rt, i: usize, label_w: f32, height: f32) -> StepHeader<'_> {
        let fmt = rt.i18n().format();
        StepHeader {
            salt: self.salt.child(("step", i)),
            key: StableId::of(&self.salt, (i, "step")),
            label: &self.labels[i],
            number: fmt.count(i + 1),
            status: self.steps.status(i),
            optional: self
                .steps
                .is_optional(i)
                .then(|| crate::tr(rt.i18n(), msg::STEP_OPTIONAL)),
            selectable: !self.disabled && self.steps.can_select(i) && i != self.steps.current(),
            label_w,
            full_width: self.axis == Axis::Vertical,
            height,
            tab_stop: focus::TabStop::Skip,
            disabled: self.disabled,
        }
    }

    fn show_header(
        &self,
        ui: &mut MineralUi<'_, Material>,
        pass: &mut StepPass,
        i: usize,
        label_w: f32,
        height: f32,
    ) -> egui::Response {
        let rt = ui.rt();
        let mut h = self.header(rt, i, label_w, height);
        h.tab_stop = if pass.tab_step == Some(i) {
            focus::TabStop::Stop
        } else {
            focus::TabStop::Skip
        };
        let reachable = !self.disabled && (h.selectable || i == pass.current);
        let r = rt.ui(ui).mineral(h);
        if reachable {
            pass.ids.push(r.id);
        }
        if r.changed() {
            pass.selected = Some(i);
        }
        r.into_response()
    }

    fn horizontal(
        &self,
        ui: &mut MineralUi<'_, Material>,
        content: &mut dyn FnMut(&mut MineralUi<'_, Material>, usize),
        pass: &mut StepPass,
    ) {
        let rt = ui.rt();
        let (n, current, dir, line) = (pass.n, pass.current, pass.dir, pass.line);
        let avail = ui.available_width();
        let label_ws: Vec<f32> = self
            .labels
            .iter()
            .take(n)
            .map(|l| {
                let w = |text: &str, font| {
                    parts::type_galley(rt.core(), ui.painter(), text, font, Color32::PLACEHOLDER, None)
                        .size()
                        .x
                };
                let label = w(&l.label, list::ITEM_LABEL_TEXT_FONT);
                l.description
                    .as_deref()
                    .map_or(label, |d| label.max(w(d, list::ITEM_SUPPORTING_TEXT_FONT)))
            })
            .collect();
        let gap = defaults::list::LEADING_CONTENT_END_PADDING;
        let full: f32 = label_ws.iter().map(|w| HEADER_BASE + gap + w).sum();
        let connectors = MIN_CONNECTOR * n.saturating_sub(1) as f32;
        let compact = full + connectors > avail;
        // Compact keeps only the current label, elided into what markers and connectors leave.
        let room = (avail - n as f32 * HEADER_BASE - gap - connectors).max(0.0);
        let header_ws: Vec<f32> = (0..n)
            .map(|i| match (compact, i == current) {
                (false, _) => label_ws[i],
                (true, true) => label_ws[i].min(room),
                (true, false) => 0.0,
            })
            .collect();
        let used: f32 = header_ws
            .iter()
            .map(|w| HEADER_BASE + if *w > 0.0 { gap + w } else { 0.0 })
            .sum();
        let header_h = (0..n).map(|i| self.header_h(ui.density(), i)).fold(0.0, f32::max);
        let connector = if n > 1 {
            ((avail - used) / (n - 1) as f32).max(MIN_CONNECTOR)
        } else {
            0.0
        };
        parts::row(ui, dir, header_h, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (i, label_w) in header_ws.iter().copied().enumerate() {
                if i > 0 {
                    let (r, _) = ui.allocate_exact_size(vec2(connector, header_h), Sense::hover());
                    ui.painter().hline(r.x_range(), r.center().y, line);
                }
                self.show_header(ui, pass, i, label_w, header_h);
            }
        });
        ui.add_space(crate::adaptive::grid(4));
        if n > 0 && !self.steps.is_finished() {
            content(ui, current);
        }
    }

    fn vertical(
        &self,
        ui: &mut MineralUi<'_, Material>,
        content: &mut dyn FnMut(&mut MineralUi<'_, Material>, usize),
        pass: &mut StepPass,
    ) {
        let rt = ui.rt();
        let (n, current, dir, line) = (pass.n, pass.current, pass.dir, pass.line);
        let w = ui.available_width();
        let mut centers: Vec<(f32, f32, f32)> = Vec::with_capacity(n);
        ui.spacing_mut().item_spacing.y = 0.0;
        for i in 0..n {
            let r = self.show_header(ui, pass, i, 1.0, self.header_h(ui.density(), i));
            let mx = dir.start_x(r.rect, Rangef::point(defaults::list::START_PADDING + MARKER / 2.0));
            centers.push((mx, r.rect.center().y - MARKER / 2.0, r.rect.center().y + MARKER / 2.0));
            let indent = LABEL_START;
            let body_top = ui.cursor().min.y;
            if i == current && !self.steps.is_finished() {
                let x0 = dir.span_x(
                    egui::Rangef::new(pass.start.x, pass.start.x + w),
                    Rangef::new(indent, indent + w - indent),
                );
                let rect = Rect::from_min_size(pos2(x0, body_top), vec2(w - indent, f32::INFINITY));
                ui.scope_builder(
                    UiBuilder::new().max_rect(rect).id_salt(pass.id.with(("content", i))),
                    |ui| {
                        ui.add_space(crate::adaptive::grid(2));
                        content(&mut rt.ui(ui), i);
                        ui.add_space(crate::adaptive::grid(4));
                    },
                );
            } else if i + 1 < n {
                ui.add_space(crate::adaptive::grid(2));
            }
        }
        for pair in centers.windows(2) {
            let ((x, _, bottom), (_, top, _)) = (pair[0], pair[1]);
            ui.painter().vline(x, bottom..=top, line);
        }
    }

    /// `content(ui, index)` fills the current step's body.
    pub fn show(
        self,
        ui: &mut MineralUi<'_, Material>,
        mut content: impl FnMut(&mut MineralUi<'_, Material>, usize),
    ) -> SelectionResponse<usize> {
        let rt = ui.rt();
        let out = ui.scope(|ui| self.run(&mut rt.ui(ui), &mut content));
        SelectionResponse::changed_to(out.response, out.inner)
    }

    fn run(
        self,
        ui: &mut MineralUi<'_, Material>,
        content: &mut dyn FnMut(&mut MineralUi<'_, Material>, usize),
    ) -> Option<usize> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = rt.tokens();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let dir = rt.i18n().direction();
        let n = self.labels.len().min(self.steps.count());
        let current = self.steps.current();
        let mut out = None;
        semantic::emit_parent(ctx, id, || {
            let name = crate::tr_or(rt.i18n(), self.label.as_deref(), msg::STEPPER);
            let mut n = SemanticNode::new(StableId::of(&self.salt, "stepper"), Role::Group, name);
            n.state.disabled = self.disabled;
            n
        });
        let line = Stroke::new(divider::THICKNESS, t.role(divider::COLOR));
        let ids = Vec::with_capacity(n);
        let stops: Vec<usize> = (0..n)
            .filter(|&i| !self.disabled && (i == current || self.steps.can_select(i)))
            .collect();
        let tab = focus::tab_stop(ctx, id, stops.iter().position(|&i| i == current), stops.len(), |_| true);
        let tab_step = stops.get(tab).copied();
        let start = ui.cursor().min;
        let mut pass = StepPass {
            id,
            ids,
            selected: None,
            line,
            tab_step,
            current,
            n,
            dir,
            start,
        };
        match self.axis {
            Axis::Horizontal => self.horizontal(ui, content, &mut pass),
            Axis::Vertical => self.vertical(ui, content, &mut pass),
        }
        let StepPass { ids, selected, .. } = pass;
        let enabled: Vec<bool> = (0..ids.len()).map(|i| stops.contains(&i)).collect();
        RovingFocus::new(self.axis)
            .wrap(false)
            .update_group(rt.core(), id, &ids, |i| enabled[i]);
        if let Some(i) = selected
            && self.steps.select(i)
        {
            out = Some(i);
        }
        let rect = Rect::from_min_max(start, pos2(start.x + ui.available_width().max(0.0), ui.cursor().min.y));
        kit::semantic::remember_rect(ctx, id, rect);
        out
    }
}

struct StepPass {
    id: egui::Id,
    ids: Vec<egui::Id>,
    selected: Option<usize>,
    line: Stroke,
    tab_step: Option<usize>,
    current: usize,
    n: usize,
    dir: Direction,
    start: egui::Pos2,
}

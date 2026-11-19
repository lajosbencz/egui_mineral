use egui::{
    Align2, CornerRadius, Frame, Id, KeyboardShortcut, Margin, Rangef, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2,
    pos2, text::LayoutJob, vec2,
};
use egui_mineral::{
    i18n::Direction,
    kit::{
        self,
        palette::{self, Entry, Labels, Row, State},
        state::TempState,
    },
    overlay,
    semantic::{self, KeyedSalt, Role, SemanticNode, StableId},
    shortcut,
    widget::{Availability, MineralUi},
};

use crate::ColorToken::{
    ColorBackgroundOverlay, ColorNeutralBackground1, ColorNeutralBackground1Hover, ColorNeutralBackground1Pressed,
    ColorNeutralForeground1, ColorNeutralForeground3, ColorNeutralForegroundDisabled, ColorStrokeFocus2,
};
use crate::{
    Fluent, FluentExt, Rt,
    inputs::SearchBox,
    spec::{
        defaults::{combobox as cb, dialog},
        tokens::{geometry as g, typography as ty},
    },
    surfaces::overlay::{Look, ModalFocus, framed},
    theme::{Elevation, Tokens},
    widget::paint,
};

// Not in Fluent UI React v9 (shadcn `Command` pattern): Dialog surface, SearchBox, Option row styling.
const WINDOW_MARGIN: f32 = g::SPACING_HORIZONTAL_L;
const PAD: f32 = g::SPACING_HORIZONTAL_M;
const ROW_PAD_V: f32 = g::SPACING_HORIZONTAL_S_NUDGE;
const ROW_PAD_H: f32 = g::SPACING_HORIZONTAL_S;
const ROW_H: f32 = ty::LINE_HEIGHT_BASE_300 + 2.0 * ROW_PAD_V;
const GROUP_H: f32 = ty::LINE_HEIGHT_BASE_200 + 2.0 * g::SPACING_HORIZONTAL_S;
const ICON: f32 = 20.0;
const GAP: f32 = g::SPACING_HORIZONTAL_S;

pub use egui_mineral::palette::{CommandPaletteResponse, PaletteCommand};

/// Modal fuzzy-filtered command list; not in Fluent UI React v9 (shadcn `Command` pattern).
#[derive(Debug)]
#[must_use = "call .show_modal(..)"]
pub struct CommandPalette<'a> {
    salt: KeyedSalt,
    query: &'a mut String,
    commands: Vec<PaletteCommand>,
    placeholder: Option<String>,
    shortcut: Option<KeyboardShortcut>,
}

impl<'a> CommandPalette<'a> {
    /// `query` is the bound search text. Agent key prefix `<id_salt>`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        commands: impl IntoIterator<Item = impl Into<PaletteCommand>>,
        query: &'a mut String,
    ) -> Self {
        Self {
            salt: id_salt.into(),
            query,
            commands: commands.into_iter().map(Into::into).collect(),
            placeholder: None,
            shortcut: None,
        }
    }

    /// Replaces the `<id_salt>` key prefix.
    pub fn agent_key(mut self, salt: impl Into<KeyedSalt>) -> Self {
        self.salt = salt.into();
        self
    }

    /// Hint shown while the query is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Toggles the palette; also closes it while open.
    pub fn shortcut(mut self, shortcut: KeyboardShortcut) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// Modal palette while `open`; running an entry or a dismissal clears `open`.
    pub fn show_modal(self, rt: &Rt, open: &mut bool) -> CommandPaletteResponse {
        let id = kit::scoped_id(rt.core(), Id::unique(("mineral::command_palette", self.salt.id_salt())));
        ModalFocus::scope(rt.ctx(), id.with("focus"), open, |_, open| self.run(rt, id, open))
    }

    fn run(self, rt: &Rt, id: Id, open: &mut bool) -> CommandPaletteResponse {
        let ctx = rt.ctx();
        let area = kit::overlay::modal_area(rt.core(), id.with("area"));
        let scope = shortcut::scope_of(ctx, area.layer());
        palette::bind_shortcuts(
            ctx,
            &self.salt,
            self.shortcut,
            &self.commands,
            scope,
            id.with("area"),
            open,
        );
        let mut st = State::load(ctx, id);
        if !*open {
            if st.was_open {
                State::remove(ctx, id);
            }
            return CommandPaletteResponse::default();
        }
        let opened = !st.was_open;
        if opened {
            self.query.clear();
            st.active = 0;
        }
        let rows = st.rows.get_or_compute(&(&**self.query, Labels(&self.commands)), || {
            palette::rows(&self.commands, self.query)
        });
        let items = palette::selectable(&rows, &self.commands);
        st.active = st.active.min(items.len().saturating_sub(1));
        // The key press that opened the palette must not run an entry.
        let mut run = if opened { None } else { st.navigate(ctx, &rows, &items) };
        let screen = ctx.content_rect();
        let width = dialog::MAX_WIDTH.min(screen.width() - 2.0 * WINDOW_MARGIN);
        let max_h = screen.height() * cb::LISTBOX_MAX_HEIGHT;
        let list_h = rows
            .iter()
            .map(|r| match r {
                Row::Header(_) => GROUP_H,
                Row::Item { .. } => ROW_H,
                _ => 0.0,
            })
            .sum::<f32>()
            .clamp(ROW_H, (max_h - 2.0 * PAD - ROW_H).max(ROW_H));
        let t = rt.tokens();
        let area = area.anchor(Align2::CENTER_TOP, vec2(0.0, (screen.height() - max_h) / 2.0));
        let mut search = None;
        let modal = kit::overlay::modal_on(rt.core(), id, area)
            .backdrop_color(t.color(ColorBackgroundOverlay))
            .frame(Frame::NONE)
            .show(ctx, |ui| {
                let look = Look {
                    fill: t.color(ColorNeutralBackground1),
                    radius: g::BORDER_RADIUS_X_LARGE,
                    elevation: Elevation::E64,
                };
                framed(ui, &t, &look, Margin::same(PAD as i8), |ui| {
                    ui.set_width(width - 2.0 * PAD);
                    let ui = &mut rt.ui(ui);
                    search = Some(Self::search(
                        &self.salt,
                        self.placeholder.as_deref(),
                        ui,
                        &mut *self.query,
                        &mut st,
                    ));
                    ui.add_space(GAP);
                    Self::list(
                        ui,
                        &View {
                            salt: &self.salt,
                            commands: &self.commands,
                            id,
                            dir: rt.i18n().direction(),
                            t: &t,
                            rows: &rows,
                            items: &items,
                            list_h,
                            width: width - 2.0 * PAD,
                        },
                        &mut st,
                        &mut run,
                    );
                });
            });
        st.reveal = false;
        if let (true, Some(search)) = (opened, search) {
            ctx.memory_mut(|m| m.request_focus(search));
        }
        let dismissed = overlay::modal(&modal);
        palette::finish(ctx, &self.commands, &mut run, dismissed, open);
        st.was_open = *open;
        st.store(ctx, id);
        CommandPaletteResponse::new(modal.response, run, dismissed)
    }

    fn search(
        salt: &KeyedSalt,
        placeholder: Option<&str>,
        ui: &mut MineralUi<'_, Fluent>,
        query: &mut String,
        st: &mut State,
    ) -> Id {
        let rt = ui.rt();
        let placeholder = crate::tr_or(rt.i18n(), placeholder, "mineral-command-placeholder");
        let before = query.clone();
        let r = ui.mineral(
            SearchBox::new(salt.child("search"), &placeholder, &mut *query)
                .placeholder(&placeholder)
                .agent_key(StableId::of(salt, "search")),
        );
        let id = r.response().id;
        if *query != before {
            st.active = 0;
        }
        id
    }

    fn list(ui: &mut MineralUi<'_, Fluent>, f: &View<'_>, st: &mut State, run: &mut Option<usize>) {
        let rt = ui.rt();
        let dir = rt.i18n().direction();
        let t = f.t;
        let (rect, _) = ui.allocate_exact_size(vec2(f.width, f.list_h), Sense::hover());
        semantic::emit(ui.ctx(), None, rect, || {
            SemanticNode::new(
                StableId::of(f.salt, "commands"),
                Role::ListBox,
                crate::tr(rt.i18n(), "mineral-command-results"),
            )
        });
        let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        let ui = &mut rt.ui(&mut inner);
        if f.rows.is_empty() {
            let format = crate::typography::format(
                ui.ctx(),
                crate::typography::TypeSpec {
                    size: ty::FONT_SIZE_BASE_300,
                    line_height: ty::LINE_HEIGHT_BASE_300,
                    weight: ty::FONT_WEIGHT_REGULAR,
                },
                t.color(ColorNeutralForeground3),
            );
            let text = crate::tr(rt.i18n(), "mineral-command-empty");
            let galley = ui.painter().layout_job(LayoutJob::single_section(text, format.clone()));
            ui.painter().galley(
                rect.center_top() + vec2(-galley.size().x / 2.0, ROW_PAD_V),
                galley,
                format.color,
            );
            return;
        }
        let active_row = f.items.get(st.active).copied();
        ScrollArea::vertical()
            .max_height(f.list_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ui.set_width(f.width);
                for (r, row) in f.rows.iter().enumerate() {
                    match row {
                        Row::Header(name) => group_header(ui, name, f.width, dir, t),
                        Row::Item { index, hits, .. } => {
                            let cmd = &f.commands[*index];
                            let active = active_row == Some(r);
                            let resp = Self::command_row(ui, f, *index, hits, active);
                            if active && st.reveal {
                                resp.scroll_to_me(None);
                            }
                            if !cmd.is_disabled() {
                                if resp.hovered()
                                    && ui.input(|i| i.pointer.delta() != Vec2::ZERO)
                                    && let Some(a) = f.items.iter().position(|x| *x == r)
                                {
                                    st.active = a;
                                }
                                if resp.clicked() {
                                    *run = Some(*index);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            });
    }

    // react-combobox useOptionStyles.styles.ts, with a leading icon and a trailing shortcut hint.
    fn command_row(ui: &mut Ui, f: &View<'_>, index: usize, hits: &[usize], active: bool) -> egui::Response {
        let cmd = &f.commands[index];
        let t = f.t;
        let dir = f.dir;
        let own = cmd.item_salt().cloned().unwrap_or_else(|| f.salt.child(index));
        let (rect, _) = ui.allocate_exact_size(vec2(f.width, ROW_H), Sense::hover());
        let sense = if cmd.is_disabled() {
            Sense::hover()
        } else {
            Sense::click() - Sense::FOCUSABLE
        };
        let resp = ui.interact(rect, f.id.with(own.id_salt()), sense);
        let radius = CornerRadius::from(g::BORDER_RADIUS_MEDIUM);
        let (fill, fore) = if cmd.is_disabled() {
            (None, ColorNeutralForegroundDisabled)
        } else if resp.is_pointer_button_down_on() {
            (Some(ColorNeutralBackground1Pressed), ColorNeutralForeground1)
        } else if resp.hovered() {
            (Some(ColorNeutralBackground1Hover), ColorNeutralForeground1)
        } else {
            (None, ColorNeutralForeground1)
        };
        if let Some(fill) = fill {
            ui.painter().rect_filled(rect, radius, t.color(fill));
        }
        let mut at = ROW_PAD_H;
        if let Some(icon) = cmd.leading_icon() {
            let x = dir.start_x(rect, Rangef::point(at + ICON / 2.0));
            paint::icon(ui.painter(), pos2(x, rect.center().y), icon, false, ICON, t.color(fore));
            at += ICON + GAP;
        }
        let hint = cmd.accelerator().map(|s| shortcut::label(ui.ctx(), &s));
        let hint_galley = hint.map(|h| {
            let format = crate::typography::format(
                ui.ctx(),
                crate::typography::TypeSpec {
                    size: ty::FONT_SIZE_BASE_200,
                    line_height: ty::LINE_HEIGHT_BASE_200,
                    weight: ty::FONT_WEIGHT_REGULAR,
                },
                t.color(ColorNeutralForeground3),
            );
            ui.painter().layout_job(LayoutJob::single_section(h, format))
        });
        let hint_w = hint_galley.as_ref().map_or(0.0, |g| g.size().x + GAP);
        let mut job = LayoutJob::default();
        for (i, c) in cmd.label().chars().enumerate() {
            let weight = if hits.contains(&i) {
                ty::FONT_WEIGHT_SEMIBOLD
            } else {
                ty::FONT_WEIGHT_REGULAR
            };
            let format = crate::typography::format(
                ui.ctx(),
                crate::typography::TypeSpec {
                    size: ty::FONT_SIZE_BASE_300,
                    line_height: ty::LINE_HEIGHT_BASE_300,
                    weight,
                },
                t.color(fore),
            );
            job.append(&c.to_string(), 0.0, format);
        }
        job.wrap = egui::text::TextWrapping::truncate_at_width((rect.width() - at - ROW_PAD_H - hint_w).max(1.0));
        let galley = ui.painter().layout_job(job);
        let x = dir.start_x(rect, Rangef::new(at, at + galley.size().x));
        ui.painter()
            .galley(pos2(x, rect.center().y - galley.size().y / 2.0), galley, t.color(fore));
        if let Some(g) = hint_galley {
            let x = dir.start_x(
                rect,
                Rangef::new(rect.width() - ROW_PAD_H - g.size().x, rect.width() - ROW_PAD_H),
            );
            ui.painter().galley(
                pos2(x, rect.center().y - g.size().y / 2.0),
                g,
                t.color(ColorNeutralForeground3),
            );
        }
        if active {
            ui.painter().rect_stroke(
                rect,
                radius,
                Stroke::new(cb::OPTION_FOCUS_RING, t.color(ColorStrokeFocus2)),
                StrokeKind::Outside,
            );
        }
        let shortcut_label = cmd.accelerator().map(|s| shortcut::label(ui.ctx(), &s));
        semantic::emit(ui.ctx(), Some(resp.id), rect, || {
            SemanticNode::new(StableId::of(&own, "command"), Role::ListBoxOption, cmd.label())
                .pressable(Availability::disabled_if(cmd.is_disabled()))
                .shortcut(shortcut_label.clone())
        });
        resp
    }
}

struct View<'a> {
    salt: &'a KeyedSalt,
    commands: &'a [PaletteCommand],
    id: Id,
    dir: Direction,
    t: &'a Tokens,
    rows: &'a [Row],
    items: &'a [usize],
    list_h: f32,
    width: f32,
}

fn group_header(ui: &mut Ui, name: &str, width: f32, dir: Direction, t: &Tokens) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, GROUP_H), Sense::hover());
    let format = crate::typography::format(
        ui.ctx(),
        crate::typography::TypeSpec {
            size: ty::FONT_SIZE_BASE_200,
            line_height: ty::LINE_HEIGHT_BASE_200,
            weight: ty::FONT_WEIGHT_SEMIBOLD,
        },
        t.color(ColorNeutralForeground3),
    );
    let galley = ui
        .painter()
        .layout_job(LayoutJob::single_section(name.to_owned(), format.clone()));
    let x = dir.start_x(rect, Rangef::new(ROW_PAD_H, ROW_PAD_H + galley.size().x));
    ui.painter()
        .galley(pos2(x, rect.top() + g::SPACING_HORIZONTAL_S), galley, format.color);
}

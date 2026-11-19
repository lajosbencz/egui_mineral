use egui_mineral::kit::state::{Memo, TempState};
use std::collections::HashMap;

use crate::Material;
use crate::MaterialExt;
use crate::Rt;
use crate::containment::dialog::surface_modal;
use crate::widget::modal_focus::ModalFocus;
use egui::{Align2, Id, KeyboardShortcut, Rangef, Rect, Sense, pos2, vec2};
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::{Availability, MineralUi};
use egui_mineral::{
    filter::Query,
    i18n::Direction,
    kit,
    layout::Edge,
    overlay::{self, Dismiss},
    perf::VirtualList,
    semantic::{self, Role, SemanticNode, StableId},
    shortcut::{self, ActionId, Scope},
};

use crate::navigation::search;
use crate::widget::{
    common, parts,
    row_parts::{self, OptionRow, RowMetrics},
};
use crate::{
    MaterialIcon,
    icons::ICON_SEARCH,
    spec::{
        ColorRole, defaults,
        tokens::{list, search_view},
    },
    tokens::Tokens,
};

// No MD3 counterpart

// MD3 compact window margin
const WINDOW_MARGIN: f32 = 16.0;

/// Entry of a [`CommandPalette`].
#[derive(Clone, Debug)]
#[must_use]
pub struct PaletteCommand {
    label: String,
    group: Option<String>,
    id: Option<ActionId>,
    shortcut: Option<KeyboardShortcut>,
    icon: Option<MaterialIcon>,
    disabled: bool,
    salt: Option<egui_mineral::semantic::KeyedSalt>,
}

impl PaletteCommand {
    crate::widget::setters! {
        /// Default the [`Self::command`] action, else the index.
        item_salt
    }

    /// `label` is the visible text and accessible name; also matched by the query.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            group: None,
            id: None,
            shortcut: None,
            icon: None,
            disabled: false,
            salt: None,
        }
    }
    /// Overline header the entry is listed under.
    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    /// Makes the entry a view of the app command `id`: running it fires the command
    /// ([`shortcut::fire`]) instead of reporting the entry as clicked. `shortcut` is bound to `id`
    /// while the palette is shown, open or not; other views of `id` may bind the same shortcut.
    pub fn command(mut self, id: impl Into<ActionId>, shortcut: impl Into<Option<KeyboardShortcut>>) -> Self {
        self.id = Some(id.into());
        self.shortcut = shortcut.into();
        self
    }
    /// Leading icon.
    pub fn icon(mut self, icon: MaterialIcon) -> Self {
        self.icon = Some(icon);
        self
    }
    crate::widget::setters! {
        /// Greys out the entry; it cannot run.
        disabled
    }
}

#[derive(Debug, PartialEq)]
enum Row {
    Header(String),
    Item { index: usize, hits: Vec<usize> },
}

#[derive(Clone, Default)]
struct State {
    active: usize,
    was_open: bool,
    reveal: bool,
    rows: Memo<Vec<Row>>,
}

impl TempState for State {}

/// Modal fuzzy-filtered command list; non-MD3 (shadcn `Command` pattern).
#[derive(Debug)]
#[must_use = "call .show_modal(..)"]
pub struct CommandPalette<'a> {
    salt: KeyedSalt,
    query: &'a mut String,
    commands: Vec<PaletteCommand>,
    placeholder: Option<String>,
    shortcut: Option<KeyboardShortcut>,
}

/// Result of [`CommandPalette::show_modal`].
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct CommandPaletteResponse {
    /// Palette surface; `None` while closed.
    pub response: Option<egui::Response>,
    /// Run entry without a [`PaletteCommand::command`]; command entries fire their command instead.
    pub clicked: Option<usize>,
    /// Why it closed without running an entry.
    pub dismissed: Option<Dismiss>,
}

impl<'a> CommandPalette<'a> {
    /// `query` is the bound search text. Agent key prefix `<id_salt>`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        commands: impl IntoIterator<Item = impl Into<PaletteCommand>>,
        query: &'a mut String,
    ) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            query,
            commands: commands.into_iter().map(Into::into).collect(),
            placeholder: None,
            shortcut: None,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
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

    fn rows(commands: &[PaletteCommand], query: &str) -> Vec<Row> {
        let mut matched: Vec<(usize, Vec<usize>)> = if query.trim().is_empty() {
            (0..commands.len()).map(|i| (i, Vec::new())).collect()
        } else {
            Query::new(query)
                .rank(commands.iter().enumerate().map(|(i, c)| (i, c.label.as_str())))
                .into_iter()
                .map(|(i, m)| (i, m.indices))
                .collect()
        };
        // Groups keep their first-appearance order; the stable sort keeps match order within one.
        let mut rank: HashMap<Option<&str>, usize> = HashMap::new();
        for c in commands {
            let next = rank.len();
            rank.entry(c.group.as_deref()).or_insert(next);
        }
        matched.sort_by_key(|(i, _)| rank[&commands[*i].group.as_deref()]);
        let mut rows = Vec::with_capacity(matched.len() + rank.len());
        let mut current = None;
        for (index, hits) in matched {
            let g = commands[index].group.as_deref();
            if current != Some(g) {
                current = Some(g);
                rows.extend(g.map(|g| Row::Header(g.to_owned())));
            }
            rows.push(Row::Item { index, hits });
        }
        rows
    }

    fn bind_shortcuts(&self, ctx: &egui::Context, id: Id, scope: Scope, open: &mut bool) {
        if let Some(k) = self.shortcut {
            let toggle = ActionId::from(String::from(StableId::of(&self.salt, "shortcut")));
            shortcut::bind(ctx, Scope::Global, k, toggle.clone());
            shortcut::bind(ctx, Scope::Modal(id.with("area")), k, toggle.clone());
            if shortcut::take_action(ctx, &toggle) {
                *open = !*open;
            }
        }
        let mut fired = false;
        for cmd in self.commands.iter().filter(|c| !c.disabled) {
            if let Some(command) = &cmd.id {
                if let Some(k) = cmd.shortcut {
                    shortcut::bind(ctx, scope, k, command);
                }
                fired |= shortcut::fired(ctx, command);
            }
        }
        if fired {
            *open = false;
        }
    }

    fn selectable(rows: &[Row], commands: &[PaletteCommand]) -> Vec<usize> {
        rows.iter()
            .enumerate()
            .filter_map(|(r, row)| match row {
                Row::Item { index, .. } if commands.get(*index).is_some_and(|c| !c.disabled) => Some(r),
                _ => None,
            })
            .collect()
    }

    fn navigate(rt: &Rt, st: &mut State, rows: &[Row], items: &[usize]) -> Option<usize> {
        let mut active = Some(st.active);
        let step = row_parts::drive_embedded(rt.ctx(), &mut active, items.len());
        if step.moved
            && let Some(a) = active
        {
            st.active = a;
            st.reveal = true;
        }
        match step.commit.and_then(|a| rows.get(*items.get(a)?)) {
            Some(&Row::Item { index, .. }) => Some(index),
            _ => None,
        }
    }

    fn body(
        query: &mut String,
        ui: &mut MineralUi<'_, Material>,
        v: &View<'_>,
        st: &mut State,
        run: &mut Option<usize>,
    ) {
        ui.set_width(v.width);
        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
        Self::header(query, ui, v, st);
        Self::list(ui, v, st, run);
    }

    fn header(query: &mut String, ui: &mut MineralUi<'_, Material>, v: &View<'_>, st: &mut State) {
        let rt = ui.rt();
        let View {
            salt,
            edit_id,
            t,
            width,
            header_h,
            placeholder,
            ..
        } = *v;
        let col = |role: ColorRole| v.col(role);
        let (head, _) = ui.allocate_exact_size(vec2(width, header_h), Sense::hover());
        common::icon(
            ui.painter(),
            search::icon_center(ui.rt(), head, Edge::Start),
            ICON_SEARCH,
            list::ITEM_LEADING_ICON_SIZE,
            col(search_view::HEADER_LEADING_ICON_COLOR),
        );
        let input = search::input_rect(rt, head, false);
        let te = search::SearchInput {
            text: &mut *query,
            id: edit_id,
            interactive: true,
            keep_escape: false,
            placeholder: Some(placeholder),
            input: col(search_view::HEADER_INPUT_TEXT_COLOR),
            hint: col(search_view::HEADER_SUPPORTING_TEXT_COLOR),
        }
        .show(ui, input);
        if te.changed() {
            st.active = 0;
        }
        semantic::emit(ui.ctx(), Some(te.id), head, || {
            SemanticNode::new(StableId::of(salt, "search"), Role::SearchInput, placeholder)
                .value(query.clone())
                .editable(Availability::Enabled)
        });
        search::view_divider(ui, width, t);
    }

    fn list(ui: &mut MineralUi<'_, Material>, v: &View<'_>, st: &mut State, run: &mut Option<usize>) {
        let rt = ui.rt();
        let View {
            salt,
            commands,
            id,
            t,
            dir,
            width,
            row_h,
            list_h,
            rows,
            items,
            ..
        } = *v;
        let col = |role: ColorRole| v.col(role);
        if rows.is_empty() {
            let (r, _) = ui.allocate_exact_size(vec2(width, row_h), Sense::hover());
            let g = parts::type_galley(
                rt.core(),
                ui.painter(),
                &crate::tr(rt.i18n(), "mineral-command-empty"),
                list::ITEM_SUPPORTING_TEXT_FONT,
                col(list::ITEM_SUPPORTING_TEXT_COLOR),
                None,
            );
            ui.painter()
                .galley(r.center() - g.size() / 2.0, g, egui::Color32::PLACEHOLDER);
            return;
        }
        let active_row = items.get(st.active).copied();
        let cx = Commands {
            id,
            salt,
            width,
            metrics: RowMetrics::list(rt.kit().density()),
            t,
        };
        let list_top = ui.cursor().min;
        ui.allocate_ui(vec2(width, list_h), |ui| {
            VirtualList::variable(salt.child("list"), row_h).show(ui, rows.len(), |ui, r| match &rows[r] {
                Row::Header(name) => group_header(&mut rt.ui(ui), name, width, dir, t),
                Row::Item { index, hits } => {
                    let cmd = &commands[*index];
                    let active = active_row == Some(r);
                    let resp = command_row(&mut rt.ui(ui), &cx, *index, cmd, hits, active);
                    if active && st.reveal {
                        resp.scroll_to_me(None);
                    }
                    if !cmd.disabled {
                        if resp.hovered()
                            && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO)
                            && let Some(a) = items.iter().position(|x| *x == r)
                        {
                            st.active = a;
                        }
                        if resp.clicked() {
                            *run = Some(*index);
                        }
                    }
                }
            });
        });
        semantic::emit(
            ui.ctx(),
            None,
            Rect::from_min_size(list_top, vec2(width, list_h)),
            || {
                SemanticNode::new(
                    StableId::of(salt, "commands"),
                    Role::ListBox,
                    crate::tr(rt.i18n(), "mineral-command-results"),
                )
            },
        );
    }

    /// Modal palette while `open`; running an entry or a dismissal clears `open`.
    pub fn show_modal(self, rt: &Rt, open: &mut bool) -> CommandPaletteResponse {
        let id = egui_mineral::kit::scoped_id(rt.core(), Id::unique(("mineral::command_palette", self.salt.id_salt())));
        ModalFocus::scope(rt.ctx(), id.with("focus"), open, None, |_, open| self.run(rt, id, open))
    }

    fn run(self, rt: &Rt, id: Id, open: &mut bool) -> CommandPaletteResponse {
        let ctx = rt.ctx();
        let area = kit::overlay::modal_area(rt.core(), id.with("area"));
        let scope = shortcut::scope_of(ctx, area.layer());
        self.bind_shortcuts(ctx, id, scope, open);
        let mut st = State::load(ctx, id);
        if !*open {
            if st.was_open {
                State::remove(ctx, id);
            }
            return CommandPaletteResponse::default();
        }
        let t = rt.tokens();
        let edit_id = id.with("edit");
        let opened = !st.was_open;
        if opened {
            self.query.clear();
            st.active = 0;
        }
        let rows = st.rows.get_or_compute(&(&**self.query, Labels(&self.commands)), || {
            Self::rows(&self.commands, self.query)
        });
        let items = Self::selectable(&rows, &self.commands);
        st.active = st.active.min(items.len().saturating_sub(1));

        let dir = rt.i18n().direction();
        // The key press that opened the palette must not run an entry.
        let mut run = if opened {
            None
        } else {
            Self::navigate(rt, &mut st, &rows, &items)
        };

        let screen = ctx.content_rect();
        let width = defaults::search_bar::MAX_WIDTH.min(screen.width() - 2.0 * WINDOW_MARGIN);
        let placeholder = crate::tr_or(rt.i18n(), self.placeholder.as_deref(), "mineral-command-placeholder");
        let header_h = search_view::DOCKED_HEADER_CONTAINER_HEIGHT;
        // A popup root: the app density.
        let row_h = row_parts::row_height(rt.kit().density());
        // Docked search view height limit; the view's top centers the tallest view.
        let max_h = screen.height() * defaults::search_bar::DOCKED_MAX_HEIGHT_RATIO;
        let list_h = rows
            .iter()
            .map(|r| match r {
                Row::Header(_) => GROUP_HEIGHT,
                Row::Item { .. } => row_h,
            })
            .sum::<f32>()
            .clamp(row_h, (max_h - header_h).max(row_h));
        let top = (screen.height() - max_h) / 2.0;
        let area = area.anchor(Align2::CENTER_TOP, vec2(0.0, top));
        let view = View {
            salt: &self.salt,
            commands: &self.commands,
            id,
            edit_id,
            t: &t,
            dir,
            width,
            header_h,
            row_h,
            list_h,
            rows: &rows,
            items: &items,
            placeholder: &placeholder,
        };
        let col = |role: ColorRole| view.col(role);
        let modal = surface_modal(
            area,
            &t,
            col(search_view::CONTAINER_COLOR),
            search_view::DOCKED_CONTAINER_SHAPE.corner_radius(&t.shape, dir),
            0.0,
        )
        .show(ctx, |ui| {
            Self::body(self.query, &mut rt.ui(ui), &view, &mut st, &mut run);
        });
        st.reveal = false;
        if opened {
            ctx.memory_mut(|m| m.request_focus(edit_id));
        }
        let dismissed = overlay::modal(&modal);
        if let Some(command) = run.and_then(|i| self.commands[i].id.as_ref()) {
            shortcut::fire(ctx, command);
            run = None;
            *open = false;
        }
        if dismissed.is_some() || run.is_some() {
            *open = false;
        }
        st.was_open = *open;
        st.store(ctx, id);
        CommandPaletteResponse {
            response: Some(modal.response),
            clicked: run,
            dismissed,
        }
    }
}

#[derive(Clone, Copy)]
struct View<'a> {
    salt: &'a KeyedSalt,
    commands: &'a [PaletteCommand],
    id: Id,
    edit_id: Id,
    t: &'a Tokens,
    dir: Direction,
    width: f32,
    header_h: f32,
    row_h: f32,
    list_h: f32,
    rows: &'a [Row],
    items: &'a [usize],
    placeholder: &'a str,
}

impl View<'_> {
    fn col(&self, role: ColorRole) -> egui::Color32 {
        self.t.role(role)
    }
}

const GROUP_HEIGHT: f32 = list::ITEM_TOP_SPACE + list::ITEM_OVERLINE_FONT.line_height() + list::ITEM_BOTTOM_SPACE;

fn group_header(ui: &mut MineralUi<'_, Material>, name: &str, width: f32, dir: Direction, t: &Tokens) {
    let rt = ui.rt();
    let (rect, _) = ui.allocate_exact_size(vec2(width, GROUP_HEIGHT), Sense::hover());
    let g = parts::type_galley(
        rt.core(),
        ui.painter(),
        name,
        list::ITEM_OVERLINE_FONT,
        t.role(list::ITEM_OVERLINE_COLOR),
        None,
    );
    let pad = defaults::list::START_PADDING;
    let x = dir.start_x(rect, Rangef::new(pad, pad + g.size().x));
    ui.painter().galley(
        pos2(x, rect.center().y - g.size().y / 2.0),
        g,
        egui::Color32::PLACEHOLDER,
    );
}

// Hashes what `CommandPalette::rows` reads.
struct Labels<'a>(&'a [PaletteCommand]);

impl std::hash::Hash for Labels<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for c in self.0 {
            (&c.label, &c.group).hash(state);
        }
        self.0.len().hash(state);
    }
}

struct Commands<'a> {
    id: Id,
    salt: &'a KeyedSalt,
    width: f32,
    metrics: RowMetrics,
    t: &'a Tokens,
}

fn command_row(
    ui: &mut MineralUi<'_, Material>,
    cx: &Commands<'_>,
    index: usize,
    cmd: &PaletteCommand,
    hits: &[usize],
    active: bool,
) -> egui::Response {
    let Commands { id, salt, width, t, .. } = *cx;
    let shortcut_label = cmd.shortcut.map(|s| shortcut::label(ui.ctx(), &s));
    let row = OptionRow {
        label: &cmd.label,
        hits,
        leading: cmd.icon.map(crate::Leading::Icon),
        trailing: shortcut_label.as_deref(),
        active,
        disabled: cmd.disabled,
        ..OptionRow::default()
    };
    let own = crate::widget::item_salt(salt, cmd.salt.as_ref(), cmd.id.as_ref(), index);
    let resp = row.show(ui, id.with(own.id_salt()), width, &cx.metrics, t);
    semantic::emit(ui.ctx(), Some(resp.id), resp.rect, || {
        row.pick_node(StableId::of(&own, "command"))
            .shortcut(shortcut_label.clone())
    });
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|r| match r {
                Row::Header(g) => format!("[{g}]"),
                Row::Item { index, .. } => index.to_string(),
            })
            .collect()
    }

    #[test]
    fn rows_group_in_first_appearance_order_and_keep_match_order() {
        let commands = [
            PaletteCommand::new("Open file").group("File"),
            PaletteCommand::new("Undo"),
            PaletteCommand::new("Save file").group("File"),
            PaletteCommand::new("Copy").group("Edit"),
            PaletteCommand::new("Redo"),
        ];
        let all = CommandPalette::rows(&commands, "");
        assert_eq!(shape(&all), ["[File]", "0", "2", "1", "4", "[Edit]", "3"]);
        let hits = CommandPalette::rows(&commands, "file");
        assert_eq!(
            shape(&hits),
            ["[File]", "0", "2"],
            "groups without matches drop their header"
        );
        let Some(Row::Item { hits, .. }) = hits.get(1) else {
            unreachable!("an item row")
        };
        assert_eq!(hits.len(), 4, "match indices move into the row");
    }
}

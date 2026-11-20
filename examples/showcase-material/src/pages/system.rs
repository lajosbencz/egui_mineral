use egui_mineral_material::MaterialExt;

use egui::{Frame, Key, KeyboardShortcut, Modifiers, Rect, RichText, Sense, Stroke, StrokeKind, vec2};
use egui_mineral::{
    bus::{BoxFuture, SpawnerExt},
    dnd::{self, RowView},
    input::{Gesture, PointerKind, UnifiedInputEvent},
    nav::{Route, Router},
    perf::{self, Eviction, ShapeCache, ShapeKey},
    persist::{self, Json, MemoryStorage, PersistError, PersistKey, Storage, Toml},
    platform::{
        MemoryPlatform, NativePlatform, Notification, PickedFile, PlatformError, PlatformService, SaveOutcome,
        SaveRequest,
    },
    undo::{UndoCommand, UndoShortcuts, UndoStack},
    window::{ModalStack, WindowSpec},
};
use egui_mineral_material::prelude::*;
use serde::{Deserialize, Serialize};

use super::{Cmd, Evt, Page, Shared, caption, section};

#[cfg(not(target_arch = "wasm32"))]
type Durable = persist::FsStorage;
#[cfg(target_arch = "wasm32")]
type Durable = persist::LocalStorage;

#[cfg(not(target_arch = "wasm32"))]
const DURABLE_LABEL: &str = "Files (temp dir)";
#[cfg(target_arch = "wasm32")]
const DURABLE_LABEL: &str = "localStorage";

#[cfg(not(target_arch = "wasm32"))]
const PLATFORM_NOTE: &str = "NativePlatform (OS clipboard, file dialogs, notifications); calls run on the app spawner and report back over the bus.";
#[cfg(target_arch = "wasm32")]
const PLATFORM_NOTE: &str = "NativePlatform (browser clipboard, file pickers, Notification API; each needs a user gesture and permission); calls run on the app spawner and report back over the bus.";

#[cfg(not(target_arch = "wasm32"))]
fn durable() -> Result<Durable, PersistError> {
    Durable::new(std::env::temp_dir().join("egui_mineral_showcase"))
}

#[cfg(target_arch = "wasm32")]
fn durable() -> Result<Durable, PersistError> {
    Durable::new("egui_mineral_showcase")
}

pub(crate) const TOPICS: &[&str] = &[
    "Command bus",
    "Undo and redo",
    "Drag and drop",
    "Deep links",
    "Persistence",
    "Platform services",
    "Unified input",
    "Windows and modals",
    "Render caches",
];

const JOB_BASE: u32 = 2_000_000;
const PERSIST_NAME: &str = "profile";
const LOG_LINES: usize = 8;

struct Doc {
    items: Vec<String>,
    count: i32,
}

struct AddItem(String);

impl UndoCommand<Doc> for AddItem {
    fn apply(&mut self, d: &mut Doc) {
        d.items.push(self.0.clone());
    }

    fn revert(&mut self, d: &mut Doc) {
        d.items.pop();
    }

    fn label(&self) -> &'static str {
        "Add item"
    }
}

struct Bump(i32);

impl UndoCommand<Doc> for Bump {
    fn apply(&mut self, d: &mut Doc) {
        d.count += self.0;
    }

    fn revert(&mut self, d: &mut Doc) {
        d.count -= self.0;
    }

    fn label(&self) -> &'static str {
        "Bump counter"
    }

    fn merge(&mut self, next: &Self) -> bool {
        self.0 += next.0;
        true
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Backend {
    Memory,
    Durable,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Format {
    Json,
    Toml,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
struct Profile {
    name: String,
    visits: usize,
}

pub(crate) struct State {
    next_job: u32,
    jobs: Vec<u32>,
    doc: Doc,
    undo: UndoStack<Doc>,
    undo_hint: String,
    left: Vec<String>,
    right: Vec<String>,
    order: Vec<String>,
    router: Router<Page>,
    link: String,
    link_note: String,
    profile: Profile,
    loaded: Option<Profile>,
    backend: Backend,
    format: Format,
    mem: MemoryStorage,
    durable: Option<Durable>,
    persist_note: String,
    persist_error: Option<String>,
    platform: Option<Result<NativePlatform, String>>,
    use_fake: bool,
    fake: MemoryPlatform,
    clip_text: String,
    platform_log: Vec<String>,
    platform_error: Option<String>,
    input_log: Vec<String>,
    input_kind: Option<PointerKind>,
    win_open: bool,
    modals: ModalStack<Modal>,
    shapes: ShapeCache,
    shape_builds: u32,
}

impl Default for State {
    fn default() -> Self {
        let words = |w: &[&str]| w.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        Self {
            next_job: JOB_BASE,
            jobs: Vec::new(),
            doc: Doc {
                items: Vec::new(),
                count: 0,
            },
            undo: UndoStack::new(64),
            undo_hint: String::new(),
            left: words(&["Apples", "Pears", "Plums", "Quinces"]),
            right: words(&["Carrots", "Leeks"]),
            order: words(&["First", "Second", "Third", "Fourth", "Fifth"]),
            router: Router::new(Page::Foundations),
            link: "/data".to_owned(),
            link_note: String::new(),
            profile: Profile {
                name: "Ada".to_owned(),
                visits: 1,
            },
            loaded: None,
            backend: Backend::Memory,
            format: Format::Json,
            mem: MemoryStorage::default(),
            durable: None,
            persist_note: String::new(),
            persist_error: None,
            platform: None,
            use_fake: false,
            fake: MemoryPlatform::default(),
            clip_text: "Hello from egui_mineral".to_owned(),
            platform_log: Vec::new(),
            platform_error: None,
            input_log: Vec::new(),
            input_kind: None,
            win_open: false,
            modals: ModalStack::new("showcase-modals"),
            shapes: ShapeCache::default(),
            shape_builds: 0,
        }
    }
}

fn button(ui: &mut MaterialUi<'_>, id: &str, label: &str) -> bool {
    button_if(ui, id, label, true)
}

fn button_if(ui: &mut MaterialUi<'_>, id: &str, label: &str, enabled: bool) -> bool {
    ui.mineral(
        PushButton::new(id.to_owned(), label)
            .variant(ButtonVariant::Tonal)
            .disabled(!enabled),
    )
    .clicked()
}

fn error_alert(ui: &mut MaterialUi<'_>, id: &str, msg: &mut Option<String>) {
    if let Some(m) = msg.as_deref() {
        let r = Alert::new(id.to_owned(), m)
            .severity(Severity::Error)
            .dismissible(true)
            .show(ui);
        if r.dismissed.is_some() {
            *msg = None;
        }
    }
}

pub(crate) fn show(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    section(ui, TOPICS[0], None, |ui| bus(ui, s, shared));
    section(ui, TOPICS[1], None, |ui| undo(ui, s));
    section(ui, TOPICS[2], None, |ui| drag_drop(ui, s));
    section(ui, TOPICS[3], None, |ui| deep_links(ui, s));
    section(ui, TOPICS[4], None, |ui| persistence(ui, s, shared));
    section(ui, TOPICS[5], None, |ui| platform(ui, s, shared));
    section(ui, TOPICS[6], None, |ui| input(ui, s));
    section(ui, TOPICS[7], None, |ui| windows(ui, s));
    section(ui, TOPICS[8], None, |ui| caches(ui, s));
}

fn bus(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    super::row(ui, |row| {
        for (label, count, millis) in [("Queue 1 job (1.5 s)", 1, 1500), ("Queue 5 jobs (0.8 s each)", 5, 800)] {
            if row.add(|ui| button(ui, label, label)) {
                for _ in 0..count {
                    let id = s.next_job;
                    if shared.bus.send(Cmd::Work { id, millis }).is_err() {
                        shared.toast("Worker is gone");
                        break;
                    }
                    s.next_job += 1;
                    s.jobs.push(id);
                }
            }
        }
        if row.add(|ui| button_if(ui, "bus-clear", "Clear list", !s.jobs.is_empty())) {
            s.jobs.clear();
        }
    });
    let done = s.jobs.iter().filter(|id| shared.done.contains(id)).count();
    ui.label(format!("{} pending, {done} done", s.jobs.len() - done));
    for id in &s.jobs {
        let state = if shared.done.contains(id) { "done" } else { "pending" };
        ui.label(format!("Job {id}: {state}"));
    }
    caption(
        ui,
        TypeScale::BodySmall,
        "One worker runs jobs in order; replies arrive as events and wake the UI.",
    );
}

fn undo(ui: &mut MaterialUi<'_>, s: &mut State) {
    let ctx = ui.ctx().clone();
    let undo_keys = [KeyboardShortcut::new(Modifiers::COMMAND, Key::Z)];
    let redo_keys = [
        KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z),
        KeyboardShortcut::new(Modifiers::COMMAND, Key::Y),
    ];
    if s.undo.handle_shortcuts(
        &ctx,
        &mut s.doc,
        UndoShortcuts {
            undo: &undo_keys,
            redo: &redo_keys,
        },
    ) {
        "Shortcut applied".clone_into(&mut s.undo_hint);
    }
    super::row(ui, |row| {
        if row.add(|ui| button(ui, "undo-add", "Add item")) {
            let n = s.doc.items.len() + 1;
            s.undo.execute(AddItem(format!("Item {n}")), &mut s.doc);
        }
        if row.add(|ui| button(ui, "undo-bump", "Bump (+1, merges)")) {
            s.undo.execute(Bump(1), &mut s.doc);
        }
        if row.add(|ui| button(ui, "undo-seal", "Seal step")) {
            s.undo.seal();
        }
        if row.add(|ui| button(ui, "undo-batch", "Batch of 3 (one step)")) {
            s.undo.begin_group("Batch");
            for _ in 0..3 {
                let n = s.doc.items.len() + 1;
                s.undo.execute(AddItem(format!("Item {n}")), &mut s.doc);
                s.undo.seal();
            }
            s.undo.end_group();
        }
    });
    super::row(ui, |row| {
        let label = s
            .undo
            .undo_label()
            .map_or_else(|| "Undo".to_owned(), |l| format!("Undo {l}"));
        if row.add(|ui| button_if(ui, "undo-undo", &label, s.undo.can_undo())) {
            s.undo.undo(&mut s.doc);
        }
        let label = s
            .undo
            .redo_label()
            .map_or_else(|| "Redo".to_owned(), |l| format!("Redo {l}"));
        if row.add(|ui| button_if(ui, "undo-redo", &label, s.undo.can_redo())) {
            s.undo.redo(&mut s.doc);
        }
    });
    ui.label(format!("Counter {}; items: {}", s.doc.count, s.doc.items.join(", ")));
    caption(
        ui,
        TypeScale::BodySmall,
        "Ctrl+Z undoes, Ctrl+Shift+Z or Ctrl+Y redoes (Cmd on macOS). Consecutive bumps merge until sealed.",
    );
    if !s.undo_hint.is_empty() {
        caption(ui, TypeScale::BodySmall, &s.undo_hint);
    }
}

struct Carry {
    from_left: bool,
    index: usize,
}

fn drag_drop(ui: &mut MaterialUi<'_>, s: &mut State) {
    let rt = ui.rt();
    let t = rt.tokens();
    let mut moved: Option<(bool, usize)> = None;
    super::top_row(ui, |row| {
        for left in [true, false] {
            let items = if left { &s.left } else { &s.right };
            row.add(|ui| {
                ui.set_width(200.0);
                ui.label(RichText::new(if left { "Fruit" } else { "Vegetables" }).strong());
                let frame = Frame::new()
                    .inner_margin(8)
                    .stroke(Stroke::new(1.0, t.color.outline_variant))
                    .corner_radius(8);
                let (_, dropped) = ui.dnd_drop_zone::<Carry, _>(frame, |ui| {
                    ui.set_min_size(vec2(180.0, 160.0));
                    let ui = &mut rt.ui(ui);
                    for (i, name) in items.iter().enumerate() {
                        dnd::drag_source(
                            ui,
                            ("dnd-item", left, name),
                            || Carry {
                                from_left: left,
                                index: i,
                            },
                            |ui| {
                                ui.label(RichText::new(name).background_color(t.color.primary_container));
                            },
                            |ui, dragging| {
                                ui.add_enabled(!dragging, egui::Label::new(name).selectable(false));
                            },
                        );
                    }
                });
                if let Some(c) = dropped
                    && c.from_left != left
                {
                    moved = Some((c.from_left, c.index));
                }
            });
        }
    });
    if let Some((from_left, index)) = moved {
        let (src, dst) = if from_left {
            (&mut s.left, &mut s.right)
        } else {
            (&mut s.right, &mut s.left)
        };
        if index < src.len() {
            dst.push(src.remove(index));
        }
    }
    caption(ui, TypeScale::BodySmall, "Drag an item into the other list.");
    ui.add_space(8.0);
    ui.label(RichText::new("Reorder").strong());
    let r = dnd::reorder_list(ui, "dnd-reorder", &s.order, |ui, _, name, view| {
        let bg = match view {
            RowView::Idle => t.color.surface_container,
            RowView::Dragged => t.color.surface_container_low,
            RowView::Preview => t.color.secondary_container,
        };
        Frame::new().fill(bg).corner_radius(8).inner_margin(8).show(ui, |ui| {
            ui.set_width(200.0);
            ui.label(name);
        });
    });
    if let Some(line) = r.indicator {
        ui.painter().hline(line.x, line.y, Stroke::new(2.0, t.color.primary));
    }
    if let Some(m) = r.moved {
        m.apply(&mut s.order);
    }
}

fn deep_links(ui: &mut MaterialUi<'_>, s: &mut State) {
    super::row(ui, |row| {
        row.mineral(TextField::new("link-url", "URL", &mut s.link).preferred_width(240.0));
        if row.add(|ui| button(ui, "link-go", "Deep link")) {
            s.link_note = match s.router.deep_link(&s.link) {
                Ok(()) => "Stack replaced".to_owned(),
                Err(e) => e.to_string(),
            };
        }
        if row.add(|ui| button_if(ui, "link-back", "Back", s.router.can_go_back())) {
            s.router.back();
        }
    });
    let parsed = Page::parse(&s.link).map_or_else(|e| e.to_string(), |p| format!("{} ({:?})", p.title(), p));
    ui.label(format!("Parse: {parsed}"));
    ui.label(format!("Stack: {}", s.router.to_paths().join("  >  ")));
    ui.label(format!("Current: {}", s.router.current().title()));
    if !s.link_note.is_empty() {
        caption(ui, TypeScale::BodySmall, &s.link_note);
    }
    caption(
        ui,
        TypeScale::BodySmall,
        "Own router instance; the shell's router drives the navigation rail. Query and fragment are ignored.",
    );
}

// One stored name in the format picked at run time; apps keep one `PersistKey` constant per value.
fn save_with<S: Storage>(st: &mut S, fmt: Format, name: &'static str, p: &Profile) -> Result<(), PersistError> {
    match fmt {
        Format::Json => persist::save(st, PersistKey::<_, Json>::new(name), p),
        Format::Toml => persist::save(st, PersistKey::<_, Toml>::new(name), p),
    }
}

fn load_with<S: Storage>(st: &S, fmt: Format) -> Result<Option<Profile>, PersistError> {
    match fmt {
        Format::Json => persist::load(st, PersistKey::<_, Json>::new(PERSIST_NAME)),
        Format::Toml => persist::load(st, PersistKey::<_, Toml>::new(PERSIST_NAME)),
    }
}

enum PersistOp {
    Save,
    Load,
    Remove,
    Corrupt,
    BadKey,
}

fn run_persist(s: &mut State, op: &PersistOp) -> Result<String, PersistError> {
    let (backend, fmt) = (s.backend, s.format);
    let profile = s.profile.clone();
    macro_rules! with_store {
        ($st:expr) => {{
            let st = $st;
            match op {
                PersistOp::Save => save_with(st, fmt, PERSIST_NAME, &profile).map(|()| "Saved".to_owned()),
                PersistOp::Load => load_with(st, fmt).map(|p| {
                    let note = if p.is_some() { "Loaded" } else { "Nothing stored" };
                    s.loaded = p;
                    note.to_owned()
                }),
                PersistOp::Remove => st.remove(PERSIST_NAME).map(|()| "Removed".to_owned()),
                PersistOp::Corrupt => st
                    .set(PERSIST_NAME, "{ not valid ::".to_owned())
                    .map(|()| "Wrote garbage".to_owned()),
                PersistOp::BadKey => save_with(st, fmt, "", &profile).map(|()| "Saved".to_owned()),
            }
        }};
    }
    match backend {
        Backend::Memory => {
            let mut mem = std::mem::take(&mut s.mem);
            let r = with_store!(&mut mem);
            s.mem = mem;
            r
        }
        Backend::Durable => {
            let mut st = match s.durable.take() {
                Some(st) => st,
                None => durable()?,
            };
            let r = with_store!(&mut st);
            s.durable = Some(st);
            r
        }
    }
}

fn persistence(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    super::row(ui, |row| {
        row.mineral(TextField::new("p-name", "Name", &mut s.profile.name).preferred_width(160.0));
        row.add(|ui| super::count(ui, "p-visits", "Visits", &mut s.profile.visits, 9_999));
    });
    super::row(ui, |row| {
        let b = row.add(|ui| {
            super::choice(
                ui,
                "p-backend",
                "Backend",
                &["Memory", DURABLE_LABEL],
                s.backend as usize,
            )
        });
        s.backend = [Backend::Memory, Backend::Durable][b];
        let f = row.add(|ui| super::choice(ui, "p-format", "Format", &["JSON", "TOML"], s.format as usize));
        s.format = [Format::Json, Format::Toml][f];
    });
    let mut op = None;
    super::row(ui, |row| {
        for (id, label, o) in [
            ("p-save", "Save", PersistOp::Save),
            ("p-load", "Load", PersistOp::Load),
            ("p-remove", "Remove", PersistOp::Remove),
            ("p-corrupt", "Write garbage", PersistOp::Corrupt),
            ("p-badkey", "Save with empty key", PersistOp::BadKey),
        ] {
            if row.add(|ui| button(ui, id, label)) {
                op = Some(o);
            }
        }
    });
    if let Some(o) = op {
        match run_persist(s, &o) {
            Ok(note) => {
                s.persist_note = note;
                s.persist_error = None;
            }
            Err(e) => {
                tracing::warn!(error = %e, "persistence demo failed");
                s.persist_error = Some(e.to_string());
                shared.toast("Persistence error");
            }
        }
    }
    error_alert(ui, "persist-error", &mut s.persist_error);
    if !s.persist_note.is_empty() {
        caption(ui, TypeScale::BodySmall, &s.persist_note);
    }
    if let Some(p) = &s.loaded {
        ui.label(format!("Loaded: {p:?}"));
    }
    caption(
        ui,
        TypeScale::BodySmall,
        "Garbage then Load surfaces a codec error; an empty key is rejected by the file backend (native only; the memory and localStorage backends accept it).",
    );
}

fn run<T: 'static>(
    shared: &Shared,
    fut: BoxFuture<Result<T, PlatformError>>,
    show: impl FnOnce(T) -> String + Send + 'static,
) {
    shared.spawner.spawn_event(&shared.bus.sender(), async move {
        Evt::Platform(fut.await.map(show).map_err(|e| e.to_string()))
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn path_buttons(ui: &mut MaterialUi<'_>, fake: Option<&MemoryPlatform>, p: &dyn PlatformService, shared: &Shared) {
    if button(ui, "plat-folder", "Pick folder") {
        if let Some(fake) = fake {
            fake.queue_path(std::path::PathBuf::from("/fake/folder"));
        }
        run(shared, p.pick_folder(FileFilter::default()), |d| {
            d.map_or_else(
                || "Folder pick cancelled".to_owned(),
                |d| format!("Folder {}", d.display()),
            )
        });
    }
    if button(ui, "plat-save-path", "Save target") {
        if let Some(fake) = fake {
            fake.queue_path(std::path::PathBuf::from("/fake/export.txt"));
        }
        run(shared, p.pick_save_path(text_save("export.txt")), |d| {
            d.map_or_else(
                || "Save target cancelled".to_owned(),
                |d| format!("Write to {}", d.display()),
            )
        });
    }
}

fn text_save(name: &str) -> SaveRequest {
    SaveRequest::new(name).filter(FileFilter::new(FileType::new("Text", ["txt"])))
}

fn save_buttons(ui: &mut MaterialUi<'_>, p: &dyn PlatformService, shared: &Shared, text: &str) {
    if button(ui, "plat-save", "Save file") {
        let bytes = text.as_bytes().to_vec();
        run(shared, p.save_file(text_save("export.txt"), bytes), |outcome| {
            match outcome {
                SaveOutcome::Saved => "Saved",
                _ => "Save cancelled",
            }
            .to_owned()
        });
    }
    if button(ui, "plat-stream", "Stream save") {
        stream_save(p, shared, text);
    }
}

fn stream_save(p: &dyn PlatformService, shared: &Shared, text: &str) {
    let open = p.save_file_writer(text_save("stream.txt"));
    // 64 KiB in 4 KiB chunks
    let chunk: Vec<u8> = format!("{text}\n").bytes().cycle().take(4096).collect();
    run(
        shared,
        Box::pin(async move {
            let Some(mut w) = open.await? else { return Ok(None) };
            for _ in 0..16 {
                w.write(&chunk).await?;
            }
            let n = w.written();
            w.close().await?;
            Ok(Some(n))
        }),
        |n| n.map_or_else(|| "Stream save cancelled".to_owned(), |n| format!("Streamed {n} bytes")),
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn native_platform(shared: &Shared) -> Result<NativePlatform, String> {
    NativePlatform::new(std::sync::Arc::clone(&shared.spawner)).map_err(|e| e.to_string())
}

#[cfg(target_arch = "wasm32")]
#[expect(clippy::unnecessary_wraps, reason = "same signature as native")]
fn native_platform(_: &Shared) -> Result<NativePlatform, String> {
    Ok(NativePlatform::new())
}

fn platform(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    for r in shared.platform.drain(..) {
        match r {
            Ok(line) => s.platform_log.push(line),
            Err(e) => s.platform_error = Some(e),
        }
    }
    let excess = s.platform_log.len().saturating_sub(32);
    s.platform_log.drain(..excess);
    ui.mineral(Switch::new(
        "plat-fake",
        "Fake platform (MemoryPlatform)",
        &mut s.use_fake,
    ));
    let p: &dyn PlatformService = if s.use_fake {
        &s.fake
    } else {
        match s.platform.get_or_insert_with(|| native_platform(shared)) {
            Ok(p) => &*p,
            Err(e) => {
                Alert::new("platform-init", format!("Platform unavailable: {e}"))
                    .severity(Severity::Error)
                    .show(ui);
                return;
            }
        }
    };
    super::row(ui, |row| {
        row.mineral(TextField::new("plat-clip", "Clipboard", &mut s.clip_text).preferred_width(220.0));
        if row.add(|ui| button(ui, "plat-copy", "Copy")) {
            run(shared, p.set_clipboard_text(s.clip_text.clone()), |()| {
                "Copied".to_owned()
            });
        }
        if row.add(|ui| button(ui, "plat-paste", "Paste")) {
            run(shared, p.clipboard_text(), |t| format!("Pasted: {t}"));
        }
    });
    super::row(ui, |row| {
        if row.add(|ui| button(ui, "plat-pick", "Pick file")) {
            if s.use_fake {
                s.fake.queue_pick(vec![
                    PickedFile::new("sample.txt", *b"fake contents").with_path("/fake/sample.txt"),
                ]);
            }
            run(
                shared,
                p.pick_file(FileFilter::new(FileType::new("Text", ["txt", "md"]))),
                |f| {
                    f.map_or_else(
                        || "Pick cancelled".to_owned(),
                        |f| match f.path() {
                            Some(p) => format!("Picked {} ({} bytes)", p.display(), f.bytes().len()),
                            None => format!("Picked {} ({} bytes)", f.name(), f.bytes().len()),
                        },
                    )
                },
            );
        }
        #[cfg(not(target_arch = "wasm32"))]
        row.add(|ui| path_buttons(ui, s.use_fake.then_some(&s.fake), p, shared));
        row.add(|ui| save_buttons(ui, p, shared, &s.clip_text));
        if row.add(|ui| button(ui, "plat-notify", "Notify")) {
            let notification = Notification::new("egui_mineral", s.clip_text.clone());
            run(shared, p.notify(notification), |()| "Notification sent".to_owned());
        }
    });
    error_alert(ui, "platform-error", &mut s.platform_error);
    for l in s.platform_log.iter().rev().take(4) {
        caption(ui, TypeScale::BodySmall, l);
    }
    if s.use_fake {
        caption(
            ui,
            TypeScale::BodySmall,
            &format!(
                "MemoryPlatform: {} saved file(s), {} notification(s); Pick returns a queued sample.",
                s.fake.saved().len(),
                s.fake.notifications().len()
            ),
        );
    }
    caption(ui, TypeScale::BodySmall, PLATFORM_NOTE);
}

fn describe(e: &UnifiedInputEvent, area: Rect) -> Option<String> {
    let at = |p: egui::Pos2| {
        area.contains(p)
            .then(|| format!("({:.0}, {:.0})", p.x - area.min.x, p.y - area.min.y))
    };
    match e {
        UnifiedInputEvent::PointerDown(p) => at(p.pos).map(|a| format!("down {:?} {a}", p.kind)),
        UnifiedInputEvent::PointerUp(p) => at(p.pos).map(|a| format!("up {:?} {a}", p.kind)),
        UnifiedInputEvent::Gesture(g) => match g {
            Gesture::Tap { pos } => at(*pos).map(|a| format!("tap {a}")),
            Gesture::LongPress { pos } => at(*pos).map(|a| format!("long press {a}")),
            Gesture::Swipe {
                start,
                direction,
                velocity,
                ..
            } => at(*start).map(|a| format!("swipe {direction:?} from {a} v={:.0}", velocity.length())),
            Gesture::Pinch { center, zoom_delta } => at(*center).map(|a| format!("pinch x{zoom_delta:.2} {a}")),
            Gesture::Pan { pos, delta } => at(*pos).map(|a| format!("pan {a} d=({:.0}, {:.0})", delta.x, delta.y)),
            _ => None,
        },
        _ => None,
    }
}

fn input(ui: &mut MaterialUi<'_>, s: &mut State) {
    let rt = ui.rt();
    let t = rt.tokens();
    let width = ui.available_width().min(480.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, 140.0), Sense::click_and_drag());
    ui.painter().rect_filled(rect, 12, t.color.surface_container);
    ui.painter().rect_stroke(
        rect,
        12,
        Stroke::new(1.0, t.color.outline_variant),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "Tap, long-press, drag, swipe, pinch here",
        TypeScale::BodyMedium.font(ui.ctx()),
        t.color.on_surface_variant,
    );
    let input = rt.core().input();
    for e in input.events.iter() {
        if let UnifiedInputEvent::PointerDown(p) | UnifiedInputEvent::PointerUp(p) = e
            && rect.contains(p.pos)
        {
            s.input_kind = Some(p.kind);
        }
        if let Some(line) = describe(e, rect) {
            if line.starts_with("pan") && s.input_log.last().is_some_and(|l| l.starts_with("pan")) {
                s.input_log.pop();
            }
            s.input_log.push(line);
        }
    }
    let excess = s.input_log.len().saturating_sub(LOG_LINES);
    s.input_log.drain(..excess);
    ui.label(format!(
        "Modality: {:?}; last pointer in area: {}",
        input.modality,
        s.input_kind.map_or("none".to_owned(), |k| format!("{k:?}"))
    ));
    for l in s.input_log.iter().rev() {
        caption(ui, TypeScale::BodySmall, l);
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, KeySalt)]
enum Modal {
    First,
    Nested,
}

fn windows(ui: &mut MaterialUi<'_>, s: &mut State) {
    let rt = ui.rt();
    super::row(ui, |row| {
        if row.add(|ui| button(ui, "win-open", "Open window")) {
            s.win_open = true;
        }
        if row.add(|ui| button(ui, "modal-open", "Open modal")) {
            s.modals.push(Modal::First);
        }
    });
    let ctx = ui.ctx().clone();
    WindowSpec::new("showcase-window", "Showcase window")
        .size(vec2(320.0, 140.0))
        .show(rt, &mut s.win_open, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.label("A separate viewport; embedded when the backend has no multi-viewport support.");
            });
        });
    s.modals.show(rt, |ui, m| {
        let first = *m.key() == Modal::First;
        ui.label(if first { "First modal" } else { "Nested modal" });
        if first && button(ui, "modal-nest", "Open nested modal") {
            m.push(Modal::Nested);
        }
        if button(ui, if first { "modal-close-a" } else { "modal-close-b" }, "Close") {
            m.close();
        }
    });
    caption(
        ui,
        TypeScale::BodySmall,
        &format!(
            "Modal stack top: {}",
            s.modals
                .top(&ctx)
                .map_or("none", |m| if *m == Modal::First { "first" } else { "nested" })
        ),
    );
}

fn caches(ui: &mut MaterialUi<'_>, s: &mut State) {
    let rt = ui.rt();
    let ctx = ui.ctx().clone();
    let t = rt.tokens();
    let width = ui.available_width().min(480.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, 72.0), Sense::hover());
    let key = ShapeKey {
        id: ui.make_persistent_id("shape-cache"),
        content_hash: u64::from(u32::from_le_bytes(t.color.primary.to_array())),
    };
    let color = t.color.primary;
    let mut built = false;
    let frame = ctx.cumulative_frame_nr();
    s.shapes.paint(
        &ui.painter().with_clip_rect(rect),
        key,
        rect.min.to_vec2(),
        frame,
        || {
            built = true;
            (0..12)
                .flat_map(|x| (0..3).map(move |y| (x, y)))
                .map(|(x, y)| {
                    egui::Shape::circle_filled(
                        egui::pos2(18.0 + x as f32 * 36.0, 12.0 + y as f32 * 24.0),
                        8.0 - y as f32 * 2.0,
                        color,
                    )
                })
                .collect()
        },
    );
    s.shapes.evict(Eviction { frame, max_age: 600 });
    if built {
        s.shape_builds += 1;
    }
    caption(
        ui,
        TypeScale::BodySmall,
        &format!(
            "ShapeCache: shapes built {} time(s); change the theme seed to rebuild.",
            s.shape_builds
        ),
    );

    let id = ui.make_persistent_id("size-memo");
    if let Some(size) = perf::size_or_discard(&ctx, id) {
        ui.painter().rect_stroke(
            Rect::from_min_size(ui.cursor().min, size),
            4,
            Stroke::new(1.0, t.color.outline),
            StrokeKind::Outside,
        );
    }
    let r = ui.scope(|ui| {
        ui.label("Outlined from the size measured on the previous pass.");
    });
    perf::store_size(&ctx, id, r.response.rect.size());
    let size = r.response.rect.size();
    caption(
        ui,
        TypeScale::BodySmall,
        &format!("size_or_discard: {:.0} x {:.0} dp", size.x, size.y),
    );
}

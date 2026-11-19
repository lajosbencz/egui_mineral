//! Shortcut and mnemonic registry. Bindings are registered every pass; keys are resolved and
//! consumed in `Runtime::pass` against the bindings registered last pass, so egui widgets
//! never see a consumed key. The app drains actions with [`take_action`]; components observe them
//! with the non-consuming [`fired`] and drain their own mnemonic items with [`take_item`].
//!
//! Actions are app-owned: an [`ActionId`] names one app action wherever it is shown (menu item,
//! palette entry), so showing it twice with the same shortcut rebinds, which is a no-op. An action
//! fires from its shortcut (resolved in `begin_pass`) or from [`fire`] (queued, delivered by the
//! next `begin_pass`); either way it is delivered once and stays readable for that whole pass,
//! whatever the read order. Its single consumer drains it with [`take_action`]; widgets that only
//! react to it (close a menu) use the non-consuming [`fired`]. An action fired by `Event::Paste`
//! carries the pasted text, drained with [`take_paste`].
//!
//! Resolution per key press: the most specific active scope wins (focus subtree by depth, then
//! topmost modal, then global); within a scope more modifiers win. While an `egui::Modal` is
//! open, global scopes, other modals and focus outside the modal are blocked. Auto-repeat events are
//! consumed without firing, so a held key fires once. While a `TextEdit` has focus, non-focus
//! bindings on keys it handles (typing, cursor movement, clipboard, undo) stay inert. Registry
//! state is per viewport. [`Policies::readline`](crate::policy::Policies::readline) gates egui's readline keys.
//! Mnemonics follow ARIA/APG menu conventions: <https://www.w3.org/WAI/ARIA/apg/patterns/menubar/>.

use std::{
    fmt,
    hash::{Hash, Hasher},
    ops::Range,
    sync::Arc,
};

use egui::{
    Context, Event, Id, InputState, Key, KeyboardShortcut, LayerId, Modifiers, Stroke, TextFormat, text::LayoutJob,
};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone)]
enum ActionRepr {
    Static(&'static str),
    Shared(Arc<str>),
    /// Accelerator of the widget with this id.
    Widget(Id),
}

/// Application-defined action identifier; cheap to clone. Static strings do not allocate, so
/// `const SAVE: ActionId = ActionId::from_static("file.save")` works; runtime actions
/// (palette entries, plugins) use `String` or `Arc<str>`. Compares equal to `&str`.
#[derive(Clone)]
pub struct ActionId(ActionRepr);

impl ActionId {
    /// Const-friendly; no allocation.
    pub const fn from_static(id: &'static str) -> Self {
        Self(ActionRepr::Static(id))
    }

    pub(crate) const fn widget(id: Id) -> Self {
        Self(ActionRepr::Widget(id))
    }

    /// Id text; equal texts compare equal regardless of representation. `None` for a widget accelerator.
    pub fn as_str(&self) -> Option<&str> {
        match &self.0 {
            ActionRepr::Static(s) => Some(s),
            ActionRepr::Shared(s) => Some(s),
            ActionRepr::Widget(_) => None,
        }
    }
}

impl From<&'static str> for ActionId {
    fn from(id: &'static str) -> Self {
        Self::from_static(id)
    }
}

impl From<String> for ActionId {
    fn from(id: String) -> Self {
        Self(ActionRepr::Shared(id.into()))
    }
}

impl From<Arc<str>> for ActionId {
    fn from(id: Arc<str>) -> Self {
        Self(ActionRepr::Shared(id))
    }
}

impl From<&ActionId> for ActionId {
    fn from(id: &ActionId) -> Self {
        id.clone()
    }
}

impl PartialEq for ActionId {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (ActionRepr::Widget(a), ActionRepr::Widget(b)) => a == b,
            _ => self.as_str().is_some_and(|a| other.as_str() == Some(a)),
        }
    }
}

impl Eq for ActionId {}

impl PartialEq<&str> for ActionId {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == Some(*other)
    }
}

impl PartialEq<str> for ActionId {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == Some(other)
    }
}

impl Hash for ActionId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match &self.0 {
            ActionRepr::Widget(id) => id.hash(state),
            _ => self.as_str().hash(state),
        }
    }
}

/// Key text is the id text (`file.save` keys `file-save`), so collection items commanding an action
/// keep their agent key when items are inserted before them.
impl crate::semantic::KeySalt for ActionId {
    fn write_key(&self, out: &mut String) {
        match &self.0 {
            ActionRepr::Widget(id) => {
                out.push_str("accelerator-");
                out.push_str(&id.value().to_string());
            }
            _ => out.push_str(self.as_str().unwrap_or_default()),
        }
    }

    fn key_parts(&self, parts: &mut crate::semantic::KeyParts) -> bool {
        match &self.0 {
            ActionRepr::Static(s) => parts.push_str(s),
            _ => false,
        }
    }
}

impl fmt::Debug for ActionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            ActionRepr::Widget(id) => write!(f, "accelerator-{:x}", id.value()),
            _ => fmt::Debug::fmt(self.as_str().unwrap_or_default(), f),
        }
    }
}

impl fmt::Display for ActionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            ActionRepr::Widget(id) => write!(f, "accelerator-{:x}", id.value()),
            _ => f.write_str(self.as_str().unwrap_or_default()),
        }
    }
}

const MAX_ANCESTORS: usize = 1000;

/// Where a shortcut is active.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    /// Active while no modal is on top ([`crate::overlay::top_modal`] is `None`).
    Global,
    /// Active while this `egui::Modal` area id is [`crate::overlay::top_modal`].
    Modal(Id),
    /// Active while the focused widget is this id or inside the `Ui` with this `unique_id`.
    Focus(Id),
}

/// Access key: an ASCII letter or digit, stored upper-case. Any other character gives an inert
/// mnemonic that never fires.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Mnemonic {
    c: char,
    key: Option<Key>,
}

/// Position of the mnemonic character in a label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MnemonicMark {
    /// Byte range of the grapheme cluster to underline in the label.
    At(Range<usize>),
    /// Translated labels without the letter (e.g. CJK) append "(F)" instead.
    Suffix,
}

/// Modifier a mnemonic needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Trigger {
    /// Alt+letter: menu bars, dialogs.
    Alt,
    /// Bare letter: open menus and popups.
    Bare,
    /// Alt+letter or bare letter.
    Either,
}

impl From<char> for Mnemonic {
    fn from(c: char) -> Self {
        Self::new(c)
    }
}

impl Mnemonic {
    /// Mnemonic for `c`, an ASCII letter or digit (the keys egui reports); anything else is inert.
    pub fn new(c: char) -> Self {
        let c = c.to_ascii_uppercase();
        let key = c
            .is_ascii_alphanumeric()
            .then(|| Key::from_name(c.encode_utf8(&mut [0; 4])))
            .flatten();
        if key.is_none() {
            crate::kit::sanitize::report(format_args!("mnemonic must be an ASCII letter or digit: {c:?}"));
        }
        Self { c, key }
    }

    /// Upper-case when ASCII.
    pub fn char(self) -> char {
        self.c
    }

    /// egui key the mnemonic listens for; `None` when inert.
    pub fn key(self) -> Option<Key> {
        self.key
    }

    /// First grapheme of `label` starting with the letter, case-insensitive; [`MnemonicMark::Suffix`] when none.
    pub fn underline(self, label: &str) -> MnemonicMark {
        label
            .grapheme_indices(true)
            .find(|(_, g)| {
                g.chars()
                    .next()
                    .is_some_and(|c| self.key.is_some() && c.eq_ignore_ascii_case(&self.c))
            })
            .map_or(MnemonicMark::Suffix, |(i, g)| MnemonicMark::At(i..i + g.len()))
    }
}

#[derive(Clone)]
struct ShortcutBinding {
    scope: Scope,
    shortcut: KeyboardShortcut,
    action: ActionId,
}

#[derive(Clone)]
struct MnemonicBinding {
    scope: Scope,
    mnemonic: Mnemonic,
    trigger: Trigger,
    item: Id,
}

#[derive(Clone, Default)]
struct Registry {
    shortcuts: Vec<ShortcutBinding>,
    mnemonics: Vec<MnemonicBinding>,
    last_shortcuts: Vec<ShortcutBinding>,
    last_mnemonics: Vec<MnemonicBinding>,
    fired: Vec<ActionId>,
    log: Vec<ActionId>,
    queued: Vec<ActionId>,
    pastes: Vec<(ActionId, String)>,
    activated: Vec<Id>,
    alt: AltTrack,
}

// Alt (not AltGr) hold since its press: `clean` while nothing else happened; `released` for the pass it ended clean.
#[derive(Clone, Copy, Default)]
struct AltTrack {
    held: bool,
    clean: bool,
    released: bool,
}

impl AltTrack {
    fn update(&mut self, ctx: &Context) {
        let (alt, other) = ctx.input(|i| {
            let m = i.modifiers;
            let other = i.events.iter().any(|e| {
                matches!(
                    e,
                    Event::Key { pressed: true, .. }
                        | Event::PointerButton { pressed: true, .. }
                        | Event::Text(_)
                        | Event::WindowFocused(_)
                )
            });
            (m.alt && !m.ctrl && !m.command && !m.mac_cmd, other)
        });
        self.released = self.held && !alt && self.clean && !other;
        self.clean = alt && (self.clean || !self.held) && !other;
        self.held = alt;
    }
}

fn registry_id(ctx: &Context) -> Id {
    crate::runtime::pass_scoped(ctx, "mineral::shortcut")
}

fn with_registry<R>(ctx: &Context, f: impl FnOnce(&mut Registry) -> R) -> R {
    let id = registry_id(ctx);
    ctx.data_mut(|d| f(d.get_temp_mut_or_default::<Registry>(id)))
}

/// Binds `shortcut` to `action` in `scope` for this pass. Rebinding the same action is a no-op;
/// another action on the same shortcut is reported and ignored, the first binding stays.
/// Command+X/C/V also match `Event::Cut/Copy/Paste` (what egui-winit emits instead of key events)
/// unless a text edit has focus. The paste text is kept for [`take_paste`]; a key press carries
/// none, so actions fall back to
/// [`PlatformClipboard::clipboard_text`](crate::platform::PlatformClipboard::clipboard_text).
pub fn bind(ctx: &Context, scope: Scope, shortcut: KeyboardShortcut, action: impl Into<ActionId>) {
    let action = action.into();
    with_registry(ctx, |r| {
        if let Some(other) = r.shortcuts.iter().find(|b| b.scope == scope && b.shortcut == shortcut) {
            if other.action != action {
                crate::kit::sanitize::report(format_args!(
                    "shortcut {shortcut:?} bound twice in {scope:?}: {:?} and {action:?}",
                    other.action
                ));
            }
            return;
        }
        r.shortcuts.push(ShortcutBinding {
            scope,
            shortcut,
            action,
        });
    });
}

/// Registers `mnemonic` for `item` in `scope` for this pass; activation is reported by [`take_item`].
/// Another item on the same mnemonic is reported and ignored, the first binding stays.
pub fn bind_mnemonic(ctx: &Context, scope: Scope, mnemonic: Mnemonic, trigger: Trigger, item: Id) {
    with_registry(ctx, |r| {
        if let Some(other) = r.mnemonics.iter().find(|b| b.scope == scope && b.mnemonic == mnemonic) {
            if other.item != item {
                crate::kit::sanitize::report(format_args!(
                    "mnemonic {:?} bound twice in {scope:?}: {:?} and {item:?}",
                    mnemonic.char(),
                    other.item
                ));
            }
            return;
        }
        r.mnemonics.push(MnemonicBinding {
            scope,
            mnemonic,
            trigger,
            item,
        });
    });
}

#[cfg(test)]
fn take_fired(ctx: &Context) -> Vec<ActionId> {
    with_registry(ctx, |r| std::mem::take(&mut r.fired))
}

/// Fires `action` like its shortcut would: queued and delivered by the next pass.
pub fn fire(ctx: &Context, action: impl Into<ActionId>) {
    let action = action.into();
    with_registry(ctx, |r| r.queued.push(action));
    #[expect(clippy::disallowed_methods, reason = "one pass to deliver the queued action")]
    ctx.request_repaint();
}

/// Non-consuming check: `action` was delivered this pass, taken or not.
pub fn fired(ctx: &Context, action: &ActionId) -> bool {
    let id = registry_id(ctx);
    ctx.data(|d| d.get_temp::<Registry>(id).is_some_and(|r| r.log.contains(action)))
}

/// Drains `action` only, leaving other fired actions for their owners.
pub fn take_action(ctx: &Context, action: &ActionId) -> bool {
    with_registry(ctx, |r| {
        let before = r.fired.len();
        r.fired.retain(|c| c != action);
        r.fired.len() != before
    })
}

#[cfg(test)]
fn take_activated(ctx: &Context) -> Vec<Id> {
    with_registry(ctx, |r| std::mem::take(&mut r.activated))
}

/// Drains the `Event::Paste` text that delivered `action` this pass. `None` when it was not
/// delivered by a paste event (a key press or [`fire`]), or was already taken.
pub fn take_paste(ctx: &Context, action: &ActionId) -> Option<String> {
    with_registry(ctx, |r| {
        let i = r.pastes.iter().position(|(c, _)| c == action)?;
        Some(r.pastes.swap_remove(i).1)
    })
}

/// Drains `item` only; see [`take_action`].
pub fn take_item(ctx: &Context, item: Id) -> bool {
    with_registry(ctx, |r| {
        let before = r.activated.len();
        r.activated.retain(|i| *i != item);
        r.activated.len() != before
    })
}

/// Scope for a widget on `layer`: its own [`Scope::Modal`] when `layer` is a modal
/// ([`crate::overlay::is_modal`], from the pass it opens in), else [`Scope::Global`]. A covered modal keeps its
/// scope, which is inactive until it is on top again.
pub fn scope_of(ctx: &Context, layer: LayerId) -> Scope {
    if crate::overlay::is_modal(ctx, layer) {
        Scope::Modal(layer.id)
    } else {
        Scope::Global
    }
}

/// Platform-aware text such as `Ctrl+S` or the macOS symbols.
pub fn label(ctx: &Context, shortcut: &KeyboardShortcut) -> String {
    ctx.format_shortcut(shortcut)
}

/// Alt pressed and released alone this pass, with no key, click or focus change while held: the Windows and Linux
/// menu bar toggle. `false` without [`Policies::mnemonics`](crate::policy::Policies::mnemonics).
pub fn lone_alt(core: &crate::runtime::Core) -> bool {
    let ctx = core.ctx();
    let id = registry_id(ctx);
    core.policies().mnemonics && ctx.data(|d| d.get_temp::<Registry>(id).is_some_and(|r| r.alt.released))
}

/// Keeps underlines visible without Alt, e.g. while a menu opened by keyboard is active.
pub fn set_mnemonics_revealed(ctx: &Context, revealed: bool) {
    crate::setting::set(ctx, "mineral::mnemonics_revealed", revealed);
}

/// Reads the flag set by [`set_mnemonics_revealed`].
pub fn mnemonics_revealed(ctx: &Context) -> bool {
    crate::setting::get::<bool>(ctx, "mineral::mnemonics_revealed")
}

/// True while enabled and (Alt held without Ctrl, which is `AltGr` on Windows, or revealed).
pub fn mnemonics_visible(core: &crate::runtime::Core) -> bool {
    let ctx = core.ctx();
    core.policies().mnemonics && (ctx.input(|i| i.modifiers.alt && !i.modifiers.ctrl) || mnemonics_revealed(ctx))
}

/// Label in `plain` with the access key underlined by `underline` while mnemonics are visible;
/// labels without the letter carry the `mineral-mnemonic-suffix` form (CJK "(F)") while
/// mnemonics are enabled.
pub fn mnemonic_job(
    core: &crate::runtime::Core,
    text: &str,
    plain: &TextFormat,
    mnemonic: Option<Mnemonic>,
    underline: Stroke,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut part = |s: &str, format: &TextFormat| {
        if !s.is_empty() {
            job.append(s, 0.0, format.clone());
        }
    };
    match mnemonic.filter(|_| core.policies().mnemonics) {
        Some(m) => match m.underline(text) {
            MnemonicMark::At(r) if mnemonics_visible(core) => {
                let marked = TextFormat {
                    underline,
                    ..plain.clone()
                };
                let (before, rest) = text.split_at(r.start);
                let (mark, after) = rest.split_at(r.len());
                part(before, plain);
                part(mark, &marked);
                part(after, plain);
            }
            MnemonicMark::At(_) => part(text, plain),
            MnemonicMark::Suffix => {
                let args = [("label", text.into()), ("key", m.char().to_string().into())];
                part(&core.i18n().tr("mineral-mnemonic-suffix", &args), plain);
            }
        },
        None => part(text, plain),
    }
    job
}

type Rank = (u8, usize);

struct Active {
    modal: Option<Id>,
    focus_allowed: bool,
    chain: Vec<Id>,
}

impl Active {
    fn of(ctx: &Context, wants_focus: bool) -> Self {
        let modal = crate::overlay::top_modal(ctx).map(|l| l.id);
        let mut chain = Vec::new();
        let mut focus_allowed = false;
        if wants_focus && let Some(focused) = ctx.memory(egui::Memory::focused) {
            chain.push(focused);
            if let Some(r) = ctx.read_response(focused) {
                focus_allowed = ctx.memory(|m| m.is_above_modal_layer(r.layer_id));
                let mut id = focused;
                for _ in 0..MAX_ANCESTORS {
                    let Some(parent) = ctx.read_response(id).map(|r| r.parent_id()) else {
                        break;
                    };
                    if parent == id {
                        break;
                    }
                    chain.push(parent);
                    id = parent;
                }
            } else {
                focus_allowed = modal.is_none();
            }
        }
        Self {
            modal,
            focus_allowed,
            chain,
        }
    }

    fn rank(&self, scope: Scope) -> Option<Rank> {
        match scope {
            Scope::Global => self.modal.is_none().then_some((2, 0)),
            Scope::Modal(id) => (self.modal == Some(id)).then_some((1, 0)),
            Scope::Focus(id) => {
                if !self.focus_allowed {
                    return None;
                }
                self.chain.iter().position(|c| *c == id).map(|depth| (0, depth))
            }
        }
    }
}

enum Candidate {
    Shortcut(usize),
    Mnemonic(usize),
}

fn modifier_count(m: Modifiers) -> usize {
    [m.alt, m.ctrl, m.shift, m.mac_cmd, m.command]
        .into_iter()
        .filter(|b| *b)
        .count()
}

fn plain(m: Modifiers) -> bool {
    !m.ctrl && !m.command && !m.mac_cmd
}

fn consume_presses(input: &mut InputState, mut matches: impl FnMut(Key, Modifiers) -> bool) -> (bool, bool) {
    let (mut consumed, mut fired) = (false, false);
    input.events.retain(|e| match e {
        Event::Key {
            key,
            pressed: true,
            modifiers,
            repeat,
            ..
        } if matches(*key, *modifiers) => {
            consumed = true;
            fired |= !repeat;
            false
        }
        _ => true,
    });
    (consumed, fired)
}

fn shortcut_matches(pressed: Modifiers, pattern: Modifiers) -> bool {
    pressed.matches_logically(pattern) && !(pressed.ctrl && pressed.alt && !pattern.alt)
}

fn consume_shortcut(input: &mut InputState, shortcut: KeyboardShortcut) -> bool {
    consume_presses(input, |key, mods| {
        key == shortcut.logical_key && shortcut_matches(mods, shortcut.modifiers)
    })
    .1
}

fn consume_mnemonic(input: &mut InputState, mnemonic: Mnemonic, trigger: Trigger) -> bool {
    let Some(key) = mnemonic.key() else {
        return false;
    };
    let (alt, bare) = (
        matches!(trigger, Trigger::Alt | Trigger::Either),
        matches!(trigger, Trigger::Bare | Trigger::Either),
    );
    let (consumed, fired) = consume_presses(input, |k, m| k == key && plain(m) && if m.alt { alt } else { bare });
    if consumed {
        input
            .events
            .retain(|e| !matches!(e, Event::Text(t) if t.eq_ignore_ascii_case(mnemonic.c.encode_utf8(&mut [0; 4]))));
    }
    fired
}

fn consume_clipboard(input: &mut egui::InputState, shortcut: KeyboardShortcut) -> (bool, Option<String>) {
    if shortcut.modifiers != Modifiers::COMMAND {
        return (false, None);
    }
    let matches: fn(&Event) -> bool = match shortcut.logical_key {
        Key::X => |e| matches!(e, Event::Cut),
        Key::C => |e| matches!(e, Event::Copy),
        Key::V => |e| matches!(e, Event::Paste(_)),
        _ => return (false, None),
    };
    let (mut hit, mut paste) = (false, None);
    input.events.retain_mut(|e| {
        if !matches(e) {
            return true;
        }
        hit = true;
        if let Event::Paste(text) = e {
            paste = Some(std::mem::take(text));
        }
        false
    });
    (hit, paste)
}

#[derive(Clone, Copy)]
enum Chord {
    Any,
    /// No Ctrl, Cmd or Alt.
    Typing,
    /// No Ctrl or Cmd; Alt allowed.
    Plain,
    /// Ctrl on Windows/Linux, Cmd on macOS.
    Command,
    Ctrl,
    CtrlNoShift,
}

impl Chord {
    fn matches(self, m: Modifiers) -> bool {
        match self {
            Self::Any => true,
            Self::Typing => !m.ctrl && !m.mac_cmd && !m.alt,
            Self::Plain => !m.ctrl && !m.mac_cmd,
            Self::Command => m.command,
            Self::Ctrl => m.ctrl,
            Self::CtrlNoShift => m.ctrl && !m.shift,
        }
    }
}

#[derive(Clone, Copy)]
enum Keys {
    In(&'static [Key]),
    /// Every key that can type a character: all but Escape and the function keys.
    Typing,
}

impl Keys {
    fn contains(self, key: Key) -> bool {
        match self {
            Self::In(keys) => keys.contains(&key),
            Self::Typing => key != Key::Escape && !(Key::F1 as u8..=Key::F35 as u8).contains(&(key as u8)),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Platform {
    Any,
    Mac,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Group {
    Core,
    /// Ctrl+H/K/U/W: egui applies them on every OS; `Policies::readline` gates them.
    Readline,
}

struct TextEditKey {
    keys: Keys,
    chord: Chord,
    platform: Platform,
    group: Group,
}

impl TextEditKey {
    fn applies(&self, os: egui::os::OperatingSystem, key: Key, m: Modifiers) -> bool {
        (self.platform == Platform::Any || os == egui::os::OperatingSystem::Mac)
            && self.keys.contains(key)
            && self.chord.matches(m)
    }
}

const NAVIGATION: &[Key] = &[
    Key::Backspace,
    Key::Delete,
    Key::ArrowLeft,
    Key::ArrowRight,
    Key::ArrowUp,
    Key::ArrowDown,
    Key::Home,
    Key::End,
];

const TEXT_EDIT_KEYS: &[TextEditKey] = &[
    TextEditKey {
        keys: Keys::In(NAVIGATION),
        chord: Chord::Any,
        platform: Platform::Any,
        group: Group::Core,
    },
    TextEditKey {
        keys: Keys::Typing,
        chord: Chord::Typing,
        platform: Platform::Any,
        group: Group::Core,
    },
    // Alt+Enter still inserts a newline.
    TextEditKey {
        keys: Keys::In(&[Key::Enter]),
        chord: Chord::Plain,
        platform: Platform::Any,
        group: Group::Core,
    },
    TextEditKey {
        keys: Keys::In(&[Key::A, Key::X, Key::C, Key::V, Key::Y, Key::Z]),
        chord: Chord::Command,
        platform: Platform::Any,
        group: Group::Core,
    },
    TextEditKey {
        keys: Keys::In(&[Key::A, Key::E, Key::P, Key::N, Key::B, Key::F]),
        chord: Chord::CtrlNoShift,
        platform: Platform::Mac,
        group: Group::Core,
    },
    TextEditKey {
        keys: Keys::In(&[Key::H, Key::K, Key::U, Key::W]),
        chord: Chord::Ctrl,
        platform: Platform::Any,
        group: Group::Readline,
    },
];

fn resolve(os: egui::os::OperatingSystem, m: Modifiers) -> Modifiers {
    let mac = os.is_mac();
    let (ctrl, mac_cmd) = (m.ctrl || (m.command && !mac), m.mac_cmd || (m.command && mac));
    Modifiers {
        ctrl,
        mac_cmd,
        command: mac_cmd || (ctrl && !mac),
        ..m
    }
}

fn edits_text(os: egui::os::OperatingSystem, readline: bool, shortcut: KeyboardShortcut) -> bool {
    let m = resolve(os, shortcut.modifiers);
    TEXT_EDIT_KEYS
        .iter()
        .any(|e| (e.group != Group::Readline || readline) && e.applies(os, shortcut.logical_key, m))
}

fn strip_readline(core: &crate::runtime::Core) {
    let ctx = core.ctx();
    if core.policies().readline || !ctx.text_edit_focused() {
        return;
    }
    let os = ctx.os();
    ctx.input_mut(|input| {
        input.events.retain(|e| match e {
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => !TEXT_EDIT_KEYS
                .iter()
                .any(|t| t.group == Group::Readline && t.applies(os, *key, *modifiers)),
            _ => true,
        });
    });
}

pub(crate) fn begin_pass(core: &crate::runtime::Core) {
    let ctx = core.ctx();
    let id = registry_id(ctx);
    let mut reg = ctx.data_mut(|d| d.remove_temp::<Registry>(id)).unwrap_or_default();
    reg.last_shortcuts = std::mem::take(&mut reg.shortcuts);
    reg.last_mnemonics = std::mem::take(&mut reg.mnemonics);
    if ctx.current_pass_index() == 0 {
        reg.alt.update(ctx);
        reg.fired.clear();
        reg.log.clear();
        reg.activated.clear();
        reg.pastes.clear();
    }
    let queued = std::mem::take(&mut reg.queued);
    reg.log.extend(queued.iter().cloned());
    reg.fired.extend(queued);
    dispatch(core, &mut reg);
    strip_readline(core);
    ctx.data_mut(|d| {
        // `fire` from another thread meanwhile wrote into a fresh registry.
        if let Some(stray) = d.remove_temp::<Registry>(id) {
            reg.queued.extend(stray.queued);
        }
        d.insert_temp(id, reg);
    });
}

fn dispatch(core: &crate::runtime::Core, reg: &mut Registry) {
    let ctx = core.ctx();
    let pressed = ctx.input(|i| {
        i.events.iter().any(|e| {
            matches!(
                e,
                Event::Key { pressed: true, .. } | Event::Cut | Event::Copy | Event::Paste(_)
            )
        })
    });
    if !pressed {
        return;
    }
    let mnemonics_on = core.policies().mnemonics;
    let wants_focus = reg.last_shortcuts.iter().any(|b| matches!(b.scope, Scope::Focus(_)))
        || (mnemonics_on && reg.last_mnemonics.iter().any(|b| matches!(b.scope, Scope::Focus(_))));
    let active = Active::of(ctx, wants_focus);

    let mut candidates: Vec<(Rank, usize, Candidate)> = Vec::new();
    for (i, b) in reg.last_shortcuts.iter().enumerate() {
        if let Some(rank) = active.rank(b.scope) {
            candidates.push((rank, modifier_count(b.shortcut.modifiers), Candidate::Shortcut(i)));
        }
    }
    if mnemonics_on {
        for (i, b) in reg.last_mnemonics.iter().enumerate() {
            if let Some(rank) = active.rank(b.scope) {
                candidates.push((rank, usize::from(b.trigger != Trigger::Bare), Candidate::Mnemonic(i)));
            }
        }
    }
    candidates.sort_by_key(|(rank, mods, _)| (*rank, std::cmp::Reverse(*mods)));

    let text_focused = ctx.text_edit_focused();
    let (os, readline) = (ctx.os(), core.policies().readline);
    let inert = |scope: Scope, edits: bool| text_focused && edits && !matches!(scope, Scope::Focus(_));
    ctx.input_mut(|input| {
        for (_, _, c) in &candidates {
            match c {
                Candidate::Shortcut(i) => {
                    let b = &reg.last_shortcuts[*i];
                    if inert(b.scope, edits_text(os, readline, b.shortcut)) {
                        continue;
                    }
                    let key = consume_shortcut(input, b.shortcut);
                    let (event, paste) = consume_clipboard(input, b.shortcut);
                    if let Some(text) = paste {
                        reg.pastes.push((b.action.clone(), text));
                    }
                    if key || event {
                        reg.fired.push(b.action.clone());
                        reg.log.push(b.action.clone());
                    }
                }
                Candidate::Mnemonic(i) => {
                    let b = &reg.last_mnemonics[*i];
                    let types = b.trigger != Trigger::Alt;
                    if inert(b.scope, types) {
                        continue;
                    }
                    if consume_mnemonic(input, b.mnemonic, b.trigger) {
                        reg.activated.push(b.item);
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use egui::{Modal, RawInput, ViewportId, os::OperatingSystem};

    use super::*;
    use crate::{
        input::synth::key_down,
        test_support::{at, input, run, tap},
    };

    const SAVE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::S);

    type Rt = Arc<crate::runtime::Runtime<crate::test_support::TestKit>>;

    fn set_mnemonics(ctx: &Context, mnemonics: bool) -> Rt {
        crate::test_support::runtime_with(
            ctx,
            crate::policy::Policies {
                mnemonics,
                ..crate::test_support::policies()
            },
        )
    }

    fn with_readline(ctx: &Context, readline: bool) -> Rt {
        crate::test_support::runtime_with(
            ctx,
            crate::policy::Policies {
                readline,
                ..crate::test_support::policies()
            },
        )
    }

    fn frame(ctx: &Context, events: Vec<Event>, add: impl FnMut(&mut egui::Ui)) {
        run(ctx, input(events), add);
    }

    fn step(rt: &Rt, events: Vec<Event>, mut add: impl FnMut(&mut egui::Ui)) -> (Vec<ActionId>, Vec<Id>) {
        let ctx = rt.ctx();
        let mut got = (Vec::new(), Vec::new());
        frame(ctx, events, |ui| {
            begin_pass(rt.core());
            got = (take_fired(ui.ctx()), take_activated(ui.ctx()));
            add(ui);
        });
        got
    }

    #[test]
    fn lone_alt_is_a_clean_alt_release() {
        let ctx = Context::default();
        let pass = |rt: &Rt, modifiers: Modifiers, mut events: Vec<Event>| {
            events.insert(0, Event::ModifiersChanged(modifiers));
            let mut lone = false;
            run(&ctx, input(events), |_| {
                begin_pass(rt.core());
                lone = lone_alt(rt.core());
            });
            lone
        };
        let rt = set_mnemonics(&ctx, true);
        assert!(!pass(&rt, Modifiers::ALT, vec![]));
        assert!(pass(&rt, Modifiers::NONE, vec![]), "Alt pressed and released alone");
        assert!(!pass(&rt, Modifiers::NONE, vec![]), "reported for one pass");
        assert!(!pass(&rt, Modifiers::ALT, vec![]));
        assert!(!pass(&rt, Modifiers::ALT, vec![key_down(Key::F, Modifiers::ALT)]));
        assert!(!pass(&rt, Modifiers::NONE, vec![]), "Alt+F");
        assert!(!pass(&rt, Modifiers::ALT | Modifiers::CTRL, vec![]));
        assert!(!pass(&rt, Modifiers::NONE, vec![]), "AltGr");
        let rt = set_mnemonics(&ctx, false);
        assert!(!pass(&rt, Modifiers::ALT, vec![]));
        assert!(!pass(&rt, Modifiers::NONE, vec![]), "off without the mnemonics policy");
    }

    #[test]
    fn mnemonic_from_char_matches_new() {
        let m = Mnemonic::from('s');
        assert_eq!(m, Mnemonic::new('s'));
        assert_eq!((m.char(), m.key()), ('S', Some(Key::S)));
    }

    #[test]
    fn clipboard_events_fire_command_shortcuts_unless_text_edit_focused() {
        let ctx = Context::default();
        let rt = crate::test_support::runtime(&ctx);
        let copy = KeyboardShortcut::new(Modifiers::COMMAND, Key::C);
        let mut text = String::new();
        let focus = std::cell::Cell::new(false);
        let mut register = |ui: &mut egui::Ui| {
            bind(ui.ctx(), Scope::Global, copy, "edit.copy");
            let r = ui.text_edit_singleline(&mut text);
            if focus.get() {
                r.request_focus();
            }
        };
        step(&rt, vec![], &mut register);
        assert_eq!(step(&rt, vec![Event::Copy], &mut register).0, ["edit.copy"]);
        let mut left = Vec::new();
        frame(&ctx, vec![Event::Copy], |ui| {
            begin_pass(rt.core());
            left = ui.input(|i| i.events.clone());
        });
        assert!(!left.contains(&Event::Copy), "consumed");

        focus.set(true);
        step(&rt, vec![], &mut register);
        step(&rt, vec![], &mut register);
        assert!(ctx.text_edit_focused());
        let mut left = Vec::new();
        let fired = step(&rt, vec![Event::Copy], |ui| {
            left = ui.input(|i| i.events.clone());
            register(ui);
        });
        assert_eq!(fired.0.len(), 0);
        assert!(left.contains(&Event::Copy), "passes through to the text edit");
    }

    #[test]
    fn paste_event_text_reaches_its_action() {
        let ctx = Context::default();
        let rt = crate::test_support::runtime(&ctx);
        let paste = KeyboardShortcut::new(Modifiers::COMMAND, Key::V);
        let cmd = ActionId::from("edit.paste");
        let register = |ui: &mut egui::Ui| bind(ui.ctx(), Scope::Global, paste, &cmd);
        step(&rt, vec![], register);
        let mut got = (false, None, None);
        frame(&ctx, vec![Event::Paste("clip".into())], |ui| {
            begin_pass(rt.core());
            let c = ui.ctx();
            got = (take_action(c, &cmd), take_paste(c, &cmd), take_paste(c, &cmd));
            register(ui);
        });
        assert_eq!(got, (true, Some("clip".into()), None), "delivered once with its text");
        let mut by_key = Some(String::new());
        frame(&ctx, tap(Key::V, Modifiers::COMMAND), |ui| {
            begin_pass(rt.core());
            assert!(fired(ui.ctx(), &cmd));
            by_key = take_paste(ui.ctx(), &cmd);
            register(ui);
        });
        assert_eq!(by_key, None, "key presses carry no text");
    }

    #[test]
    fn most_specific_scope_wins_and_consumes() {
        let ctx = Context::default();
        let rt = crate::test_support::runtime(&ctx);
        let field = Id::unique("field");
        let open = KeyboardShortcut::new(Modifiers::CTRL, Key::O);
        let register = |ui: &mut egui::Ui| {
            let group = ui
                .scope(|ui| {
                    ui.interact(
                        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(10.0, 10.0)),
                        field,
                        egui::Sense::click(),
                    );
                    ui.unique_id()
                })
                .inner;
            let c = ui.ctx();
            bind(c, Scope::Global, SAVE, "global.save");
            bind(c, Scope::Global, open, "global.open");
            bind(c, Scope::Focus(group), open, "group.open");
            bind(c, Scope::Focus(field), SAVE, "field.save");
        };
        let ctrl = |k| tap(k, Modifiers::CTRL);
        step(&rt, vec![], register);
        assert_eq!(step(&rt, ctrl(Key::S), register).0, ["global.save"]);

        ctx.memory_mut(|m| m.request_focus(field));
        step(&rt, vec![], register);
        step(&rt, vec![], register);
        assert_eq!(step(&rt, ctrl(Key::S), register).0, ["field.save"]);
        assert_eq!(step(&rt, ctrl(Key::O), register).0, ["group.open"]);
    }

    #[test]
    fn modal_blocks_lower_scopes() {
        let ctx = Context::default();
        let rt = crate::test_support::runtime(&ctx);
        let modal = Id::unique("dialog");
        let register = |ui: &mut egui::Ui| {
            bind(ui.ctx(), Scope::Global, SAVE, "global.save");
            bind(ui.ctx(), Scope::Modal(modal), SAVE, "modal.save");
            // ast-grep-ignore: raw-egui-area
            Modal::new(modal).show(ui.ctx(), |_| {});
        };
        step(&rt, vec![], register);
        step(&rt, vec![], register);
        assert_eq!(
            step(&rt, vec![key_down(Key::S, Modifiers::CTRL)], register).0,
            ["modal.save"]
        );

        let only_global = |ui: &mut egui::Ui| {
            bind(ui.ctx(), Scope::Global, SAVE, "global.save");
            // ast-grep-ignore: raw-egui-area
            Modal::new(modal).show(ui.ctx(), |_| {});
        };
        step(&rt, vec![], only_global);
        assert_eq!(
            step(&rt, vec![key_down(Key::S, Modifiers::CTRL)], only_global).0,
            Vec::<ActionId>::new()
        );
    }

    #[test]
    fn library_modal_binds_in_its_own_scope_from_its_opening_pass() {
        let ctx = Context::default();
        let rt = crate::test_support::runtime(&ctx);
        let modal = Id::unique("dialog");
        let scopes = std::cell::RefCell::new(Vec::new());
        let register = |ui: &mut egui::Ui| {
            let area = crate::kit::overlay::modal_area(rt.core(), modal);
            Modal {
                area,
                backdrop_color: egui::Color32::TRANSPARENT,
                frame: None,
            }
            .show(ui.ctx(), |ui| {
                scopes.borrow_mut().push(scope_of(ui.ctx(), ui.layer_id()));
            });
        };
        step(&rt, vec![], register);
        step(&rt, vec![], register);
        assert_eq!(*scopes.borrow(), [Scope::Modal(modal), Scope::Modal(modal)]);
    }

    #[test]
    fn shared_command_fires_once_from_key_or_fire() {
        const SAVE_CMD: ActionId = ActionId::from_static("file.save");
        let ctx = Context::default();
        let rt = crate::test_support::runtime(&ctx);
        let (mut peeked, mut taken, mut again) = (false, false, false);
        let mut run = |events: Vec<Event>, fire_now: bool| {
            frame(&ctx, events, |ui| {
                begin_pass(rt.core());
                // Menu and palette show the same action: the second bind is a no-op.
                bind(ui.ctx(), Scope::Global, SAVE, SAVE_CMD);
                taken = take_action(ui.ctx(), &SAVE_CMD);
                again = take_action(ui.ctx(), &SAVE_CMD);
                peeked = fired(ui.ctx(), &SAVE_CMD);
                bind(ui.ctx(), Scope::Global, SAVE, SAVE_CMD);
                if fire_now {
                    fire(ui.ctx(), SAVE_CMD);
                }
            });
            (taken, again, peeked)
        };
        run(vec![], false);
        assert_eq!(run(vec![key_down(Key::S, Modifiers::CTRL)], false), (true, false, true));
        assert_eq!(run(vec![], true), (false, false, false), "fire delivers next pass");
        assert_eq!(run(vec![], false), (true, false, true));
        assert_eq!(run(vec![], false), (false, false, false));
    }

    #[test]
    fn runtime_commands_and_targeted_take() {
        const OPEN: ActionId = ActionId::from_static("file.open");
        let ctx = Context::default();
        let rt = crate::test_support::runtime(&ctx);
        let dynamic = ActionId::from(format!("plugin.{}", 7));
        let register = |ui: &mut egui::Ui| {
            bind(ui.ctx(), Scope::Global, SAVE, dynamic.clone());
            bind(
                ui.ctx(),
                Scope::Global,
                KeyboardShortcut::new(Modifiers::CTRL, Key::O),
                OPEN,
            );
        };
        step(&rt, vec![], register);
        let mut rest = Vec::new();
        let mut taken = false;
        frame(
            &ctx,
            vec![key_down(Key::S, Modifiers::CTRL), key_down(Key::O, Modifiers::CTRL)],
            |ui| {
                begin_pass(rt.core());
                taken = take_action(ui.ctx(), &OPEN);
                rest = take_fired(ui.ctx());
            },
        );
        assert!(taken);
        assert_eq!(rest, ["plugin.7"]);
    }

    #[test]
    fn duplicate_shortcut_keeps_the_first_binding() {
        let ctx = Context::default();
        bind(&ctx, Scope::Global, SAVE, "a");
        bind(&ctx, Scope::Global, SAVE, "b");
        with_registry(&ctx, |r| {
            let actions: Vec<_> = r.shortcuts.iter().map(|b| b.action.clone()).collect();
            assert_eq!(actions, ["a"]);
        });
    }

    #[test]
    fn duplicate_mnemonic_keeps_the_first_binding() {
        let ctx = Context::default();
        let (a, b) = (Id::unique("a"), Id::unique("b"));
        bind_mnemonic(&ctx, Scope::Global, Mnemonic::new('f'), Trigger::Alt, a);
        bind_mnemonic(&ctx, Scope::Global, Mnemonic::new('F'), Trigger::Alt, b);
        with_registry(&ctx, |r| {
            let items: Vec<_> = r.mnemonics.iter().map(|m| m.item).collect();
            assert_eq!(items, [a]);
        });
    }

    #[test]
    fn invalid_mnemonic_is_inert() {
        let m = Mnemonic::new('\u{e9}');
        assert_eq!(m.key(), None);
        assert_eq!(m.underline("\u{e9}t\u{e9}"), MnemonicMark::Suffix);
        assert_eq!(Mnemonic::new('f').key(), Some(Key::F));
    }

    #[test]
    fn underline_marks() {
        let f = Mnemonic::new('f');
        assert_eq!(f.underline("Open file"), MnemonicMark::At(5..6));
        assert_eq!(f.underline("File"), MnemonicMark::At(0..1));
        assert_eq!(f.underline("\u{30d5}\u{30a1}\u{30a4}\u{30eb}"), MnemonicMark::Suffix);
        // multi-byte prefix; the cluster is `e` + combining acute
        assert_eq!(Mnemonic::new('e').underline("\u{dc}be\u{301}r"), MnemonicMark::At(3..6));
        assert_eq!(Mnemonic::new('Q').underline("\u{dc}ber"), MnemonicMark::Suffix);
    }

    #[test]
    fn alt_letter_consumed_when_matched_else_passes_through() {
        let ctx = Context::default();
        let rt = set_mnemonics(&ctx, true);
        let item = Id::unique("file-menu");
        let register = |ui: &mut egui::Ui| {
            bind_mnemonic(ui.ctx(), Scope::Global, Mnemonic::new('f'), Trigger::Alt, item);
        };
        step(&rt, vec![], register);
        let events = vec![
            key_down(Key::F, Modifiers::ALT),
            Event::Text("f".into()),
            key_down(Key::G, Modifiers::ALT),
            Event::Text("g".into()),
        ];
        let mut left = Vec::new();
        let mut activated = Vec::new();
        frame(&ctx, events, |ui| {
            begin_pass(rt.core());
            activated = take_activated(ui.ctx());
            left = ui.ctx().input(|i| i.events.clone());
            register(ui);
        });
        assert_eq!(activated, [item]);
        assert!(left.iter().all(|e| !matches!(e, Event::Key { key: Key::F, .. })));
        assert!(left.iter().any(|e| matches!(e, Event::Key { key: Key::G, .. })));
        assert!(left.iter().any(|e| matches!(e, Event::Text(t) if t == "g")));
        assert!(!left.iter().any(|e| matches!(e, Event::Text(t) if t == "f")));
    }

    #[test]
    fn mnemonics_off_by_default() {
        let ctx = Context::default();
        let rt = crate::test_support::runtime(&ctx);
        let item = Id::unique("file-menu");
        let register = |ui: &mut egui::Ui| {
            bind_mnemonic(ui.ctx(), Scope::Global, Mnemonic::new('f'), Trigger::Alt, item);
        };
        step(&rt, vec![], register);
        assert_eq!(step(&rt, tap(Key::F, Modifiers::ALT), register).1, Vec::<Id>::new());

        let rt = set_mnemonics(&ctx, true);
        step(&rt, vec![], register);
        assert_eq!(step(&rt, tap(Key::F, Modifiers::ALT), register).1, [item]);
    }

    #[test]
    fn held_key_fires_once_and_is_consumed() {
        let ctx = Context::default();
        let rt = set_mnemonics(&ctx, true);
        let item = Id::unique("file-menu");
        let register = |ui: &mut egui::Ui| {
            bind(ui.ctx(), Scope::Global, SAVE, "global.save");
            bind_mnemonic(ui.ctx(), Scope::Global, Mnemonic::new('f'), Trigger::Bare, item);
        };
        // The keys stay down between frames, so egui flags later presses as repeats.
        let held = |events: Vec<Event>| {
            let mut got = (Vec::new(), Vec::new(), Vec::new());
            frame(&ctx, events, |ui| {
                begin_pass(rt.core());
                got = (
                    take_fired(ui.ctx()),
                    take_activated(ui.ctx()),
                    ui.input(|i| i.events.clone()),
                );
                register(ui);
            });
            got
        };
        held(vec![]);
        let down = vec![key_down(Key::S, Modifiers::CTRL), key_down(Key::F, Modifiers::NONE)];
        let first = held(down.clone());
        assert_eq!((first.0, first.1), (vec!["global.save".into()], vec![item]));

        let mut again = down;
        again.push(Event::Text("f".into()));
        let (fired, activated, left) = held(again);
        assert!(fired.is_empty() && activated.is_empty(), "{fired:?} {activated:?}");
        assert!(left.is_empty(), "repeats and their text are consumed: {left:?}");
    }

    #[test]
    fn altgr_does_not_fire_ctrl_binding_or_mnemonic() {
        let ctx = Context::default();
        let rt = set_mnemonics(&ctx, true);
        let item = Id::unique("quit-button");
        let register = |ui: &mut egui::Ui| {
            bind(
                ui.ctx(),
                Scope::Global,
                KeyboardShortcut::new(Modifiers::CTRL, Key::Q),
                "app.quit",
            );
            bind_mnemonic(ui.ctx(), Scope::Global, Mnemonic::new('q'), Trigger::Alt, item);
        };
        let altgr = Modifiers {
            alt: true,
            ctrl: true,
            command: true,
            ..Modifiers::NONE
        };
        step(&rt, vec![], register);
        assert_eq!(step(&rt, tap(Key::Q, altgr), register), (vec![], vec![]));
        assert_eq!(step(&rt, tap(Key::Q, Modifiers::CTRL), register).0, ["app.quit"]);
        assert_eq!(step(&rt, tap(Key::Q, Modifiers::ALT), register).1, [item]);
        for (modifiers, visible) in [(altgr, false), (Modifiers::ALT, true)] {
            frame(&ctx, vec![Event::ModifiersChanged(modifiers)], |_| {
                assert_eq!(mnemonics_visible(rt.core()), visible);
            });
        }
    }

    #[test]
    fn text_edit_keeps_typing_and_editing_keys() {
        let ctx = Context::default();
        let rt = crate::test_support::runtime(&ctx);
        let cmd = |k| KeyboardShortcut::new(Modifiers::COMMAND, k);
        let mut text = String::new();
        let register = |ui: &mut egui::Ui| {
            let c = ui.ctx();
            bind(
                c,
                Scope::Global,
                KeyboardShortcut::new(Modifiers::NONE, Key::J),
                "bare.j",
            );
            bind(
                c,
                Scope::Global,
                KeyboardShortcut::new(Modifiers::NONE, Key::F12),
                "bare.f12",
            );
            bind(c, Scope::Global, cmd(Key::A), "edit.all");
            bind(c, Scope::Global, cmd(Key::N), "file.new");
        };
        let mut add = |ui: &mut egui::Ui| {
            register(ui);
            let r = ui.text_edit_singleline(&mut text);
            r.request_focus();
        };
        step(&rt, vec![], &mut add);
        step(&rt, vec![], &mut add);
        assert!(ctx.text_edit_focused());
        let events = vec![
            key_down(Key::J, Modifiers::NONE),
            key_down(Key::A, Modifiers::COMMAND),
            key_down(Key::N, Modifiers::COMMAND),
            key_down(Key::F12, Modifiers::NONE),
        ];
        assert_eq!(step(&rt, events, &mut add).0, ["file.new", "bare.f12"]);
    }

    #[test]
    fn viewports_keep_separate_bindings() {
        let ctx = Context::default();
        let rt = crate::test_support::runtime(&ctx);
        let (a, b) = (ViewportId::from_hash_of("a"), ViewportId::from_hash_of("b"));
        let run = |viewport_id, events: Vec<Event>, bound: bool| {
            let mut fired = Vec::new();
            let raw = RawInput {
                viewport_id,
                viewports: [(viewport_id, egui::ViewportInfo::default())].into_iter().collect(),
                ..input(events)
            };
            run(&ctx, raw, |ui| {
                begin_pass(rt.core());
                fired = take_fired(ui.ctx());
                if bound {
                    bind(ui.ctx(), Scope::Global, SAVE, "save");
                }
            });
            fired
        };
        run(a, vec![], true);
        run(b, vec![], false);
        assert_eq!(run(b, tap(Key::S, Modifiers::CTRL), false), Vec::<ActionId>::new());
        assert_eq!(run(a, tap(Key::S, Modifiers::CTRL), true), ["save"]);
    }

    #[derive(PartialEq, Debug)]
    struct Probe {
        text: String,
        cursor: Option<egui::text::CCursorRange>,
        copied: Option<String>,
    }

    struct EditHarness {
        ctx: Context,
        text: String,
        time: f64,
    }

    impl EditHarness {
        fn id() -> Id {
            Id::unique("drift-edit")
        }

        fn new(os: OperatingSystem) -> Self {
            let ctx = Context::default();
            ctx.set_os(os);
            let mut h = Self {
                ctx,
                text: "hello\nworld foo".into(),
                time: 0.0,
            };
            // Typing then idling commits undo points, so Cmd+Z and Cmd+Y have history.
            h.frame(vec![]);
            h.frame(vec![Event::Text("!".into())]);
            h.time += 2.0;
            h.frame(vec![]);
            let id = Self::id();
            let mut state = egui::text_edit::TextEditState::load(&h.ctx, id).expect("text edit state");
            state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(2),
                egui::text::CCursor::new(4),
            )));
            state.store(&h.ctx, id);
            h
        }

        fn frame(&mut self, events: Vec<Event>) -> Probe {
            self.time += 0.1;
            let mut text = std::mem::take(&mut self.text);
            let out = run(&self.ctx, at(self.time, events), |ui| {
                ui.add(egui::TextEdit::multiline(&mut text).id(Self::id()))
                    .request_focus();
            });
            self.text = text;
            let state = egui::text_edit::TextEditState::load(&self.ctx, Self::id()).expect("text edit state");
            Probe {
                text: self.text.clone(),
                cursor: state.cursor.char_range(),
                copied: out.platform_output.commands.iter().find_map(|c| match c {
                    egui::OutputCommand::CopyText(t) => Some(t.clone()),
                    _ => None,
                }),
            }
        }
    }

    fn press_events(key: Key, modifiers: Modifiers) -> Vec<Event> {
        match (key, modifiers.command) {
            (Key::X, true) => vec![Event::Cut],
            (Key::C, true) => vec![Event::Copy],
            (Key::V, true) => vec![Event::Paste("p".into())],
            _ => vec![key_down(key, modifiers)],
        }
    }

    fn sample_modifiers(os: OperatingSystem, chord: Chord) -> Modifiers {
        resolve(
            os,
            match chord {
                Chord::Command => Modifiers::COMMAND,
                Chord::Ctrl | Chord::CtrlNoShift => Modifiers::CTRL,
                Chord::Any | Chord::Typing | Chord::Plain => Modifiers::NONE,
            },
        )
    }

    #[test]
    fn text_edit_keys_edit_text() {
        let os = Context::default().os();
        for entry in TEXT_EDIT_KEYS
            .iter()
            .filter(|e| e.platform == Platform::Any || os == OperatingSystem::Mac)
        {
            let modifiers = sample_modifiers(os, entry.chord);
            let keys: &[Key] = match entry.keys {
                Keys::In(keys) => keys,
                Keys::Typing => &[Key::A],
            };
            for &key in keys {
                let mut h = EditHarness::new(os);
                if key == Key::Y {
                    // Redo needs an undo first.
                    let undo = h.frame(press_events(Key::Z, modifiers));
                    assert_ne!(undo.text, "hello\nworld foo", "undo before redo");
                }
                let before = h.frame(vec![]);
                let mut events = press_events(key, modifiers);
                if matches!(entry.keys, Keys::Typing) {
                    events.push(Event::Text("a".into()));
                }
                let after = h.frame(events);
                assert_ne!(before, after, "egui no longer edits on {key:?} {modifiers:?}");
            }
        }
    }

    #[test]
    fn readline_off_frees_ctrl_k_for_bindings() {
        let kill = KeyboardShortcut::new(Modifiers::CTRL, Key::K);
        let os = Context::default().os();
        for readline in [false, true] {
            let mut h = EditHarness::new(os);
            let rt = with_readline(&h.ctx, readline);
            let mut fired = Vec::new();
            let mut frame = |h: &mut EditHarness, events: Vec<Event>| {
                h.time += 0.1;
                let mut text = std::mem::take(&mut h.text);
                run(&h.ctx, at(h.time, events), |ui| {
                    begin_pass(rt.core());
                    fired.extend(take_fired(ui.ctx()));
                    bind(ui.ctx(), Scope::Global, kill, "line.kill");
                    ui.add(egui::TextEdit::multiline(&mut text).id(EditHarness::id()))
                        .request_focus();
                });
                h.text = text;
            };
            frame(&mut h, vec![]);
            let before = h.text.clone();
            frame(&mut h, vec![key_down(Key::K, resolve(os, Modifiers::CTRL))]);
            assert_eq!(fired.is_empty(), readline, "binding fires only with readline off");
            assert_eq!(h.text != before, readline, "text edits only with readline on");
        }
    }

    #[test]
    fn readline_off_strips_unbound_keys() {
        let os = Context::default().os();
        let mut h = EditHarness::new(os);
        let rt = with_readline(&h.ctx, false);
        let (kill_word, delete_word) = (
            key_down(Key::W, resolve(os, Modifiers::CTRL)),
            key_down(Key::Backspace, resolve(os, Modifiers::CTRL)),
        );
        let mut left = Vec::new();
        let mut text = std::mem::take(&mut h.text);
        run(&h.ctx, at(h.time, vec![kill_word.clone(), delete_word.clone()]), |ui| {
            ui.add(egui::TextEdit::multiline(&mut text).id(EditHarness::id()))
                .request_focus();
            begin_pass(rt.core());
            left = ui.input(|i| i.events.clone());
        });
        assert!(left.contains(&delete_word), "Ctrl+Backspace stays: {left:?}");
        assert!(!left.contains(&kill_word), "readline Ctrl+W is stripped: {left:?}");
    }
}

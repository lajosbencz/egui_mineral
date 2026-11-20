use egui::{Event, Key, KeyboardShortcut, Modifiers, PointerButton, TouchPhase, vec2};
use egui_mineral::{
    agent::{AgentAction, AgentError},
    input::synth,
    layout::{Measure, MinTarget, Zones},
    select::Selection,
    semantic::Role,
};
use egui_mineral_material::{
    actions::{ButtonGroup, ButtonGroupItem, ButtonGroupVariant, IconButton, PushButton, Segment, SegmentedButton},
    containment::{ListItem, ListSlot},
    icons::ICON_SAVE,
    selection::{CheckButton, Chip, Radio},
};

use crate::support::{Harness, MineralHarness, Mui, SCREEN, harness};

fn tap_button(disabled: bool) -> usize {
    let mut h = harness(SCREEN, 0, |ui, clicks| {
        if ui.mineral(PushButton::new("ok", "OK").disabled(disabled)).clicked() {
            *clicks += 1;
        }
    });
    let slop = h.rt().core().policies().gestures.slop;
    let click_dist = h.ctx.options(|o| o.input_options.max_click_dist);
    assert!(click_dist < slop, "the drift must tell the two apart");
    let at = h.rect("ok-button").center();
    let end = at + vec2(f32::midpoint(click_dist, slop), 0.0);
    h.input_mut().events.extend([
        synth::touch(at, TouchPhase::Start),
        Event::PointerMoved(at),
        synth::pointer_down(at, PointerButton::Primary),
    ]);
    h.step();
    h.input_mut().events.extend([
        Event::PointerMoved(end),
        synth::touch(end, TouchPhase::End),
        synth::pointer_up(end, PointerButton::Primary),
        Event::PointerGone,
    ]);
    h.run();
    h.state().value
}

#[test]
fn touch_tap_within_slop_clicks_button() {
    assert_eq!(tap_button(false), 1);
}

#[test]
fn touch_tap_ignored_by_disabled_button() {
    assert_eq!(tap_button(true), 0);
}

#[test]
fn shortcut_activates_enabled_icon_button_only() {
    const SAVE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
    for disabled in [false, true] {
        let mut h = harness(SCREEN, 0, |ui, clicks| {
            let button = IconButton::new("save", ICON_SAVE, "Save")
                .shortcut(SAVE)
                .disabled(disabled);
            if ui.mineral(button).clicked() {
                *clicks += 1;
            }
        });
        h.key_press_modifiers(SAVE.modifiers, SAVE.logical_key);
        h.run();
        assert_eq!(h.state().value, usize::from(!disabled), "disabled {disabled}");
    }
}

fn assert_gated(key: &'static str, mut show: impl FnMut(&mut Mui<'_>, &mut bool, &mut u32) + 'static) {
    let mut h = harness(SCREEN, (false, 0_u32), move |ui, (on, fired)| show(ui, on, fired));
    assert!(h.node(key).state.disabled, "{key} is disabled");
    assert!(
        !h.node(key).actions.contains(&egui_mineral::semantic::Action::Click),
        "{key} offers no Click"
    );
    h.click(key);
    let id = h.widget(key);
    h.ctx.memory_mut(|m| m.request_focus(id));
    for key in [Key::Space, Key::Enter, Key::Tab, Key::Space] {
        h.press(key);
    }
    assert_eq!(h.state().value, (false, 0), "{key} ignores click and keys");
    let got = h.drive_agent(move |agent| futures_executor::block_on(agent.act(key, AgentAction::Click)));
    assert!(
        matches!(got, Err(AgentError::ActionNotAvailable { .. })),
        "{key}: {got:?}"
    );
    h.run();
    assert_eq!(h.state().value, (false, 0), "{key} ignores the agent");
}

#[test]
fn disabled_widgets_ignore_click_keys_and_agent() {
    assert_gated("b-button", |ui, _, fired| {
        *fired += u32::from(
            ui.mineral(IconButton::new("b", ICON_SAVE, "Save").disabled(true))
                .clicked(),
        );
    });
    assert_gated("c-chip", |ui, on, fired| {
        *fired += u32::from(ui.mineral(Chip::filter("c", "Chip", on).disabled(true)).changed());
    });
    assert_gated("k-checkbox", |ui, on, fired| {
        *fired += u32::from(ui.mineral(CheckButton::new("k", "Check", on).disabled(true)).changed());
    });
    assert_gated("l-list-item", |ui, _, fired| {
        let r = ui.mineral(
            ListItem::new("l", "Row")
                .leading(ListSlot::new().checkbox(false))
                .clickable(true)
                .disabled(true),
        );
        let out = r.output();
        *fired += u32::from(out.activated || out.leading_toggled);
    });
}

fn button_group<'a>(
    variant: ButtonGroupVariant,
    selection: Option<Selection<usize>>,
) -> Harness<'a, Option<Selection<usize>>> {
    harness(SCREEN, selection, move |ui, selection| {
        let items = [
            ButtonGroupItem::new("A"),
            ButtonGroupItem::new("B").disabled(true),
            ButtonGroupItem::new("C"),
        ];
        if let Some(s) = selection {
            ButtonGroup::multi("grp", "Letters", s)
                .variant(variant)
                .toggles(items.into_iter().enumerate())
                .show(ui);
        } else {
            ButtonGroup::new("grp", "Letters")
                .variant(variant)
                .items(items)
                .show(ui);
        }
        ui.mineral(PushButton::new("after", "After"));
    })
}

#[test]
fn selection_button_groups_are_one_tab_stop_with_arrows_skipping_disabled() {
    for variant in [ButtonGroupVariant::Connected, ButtonGroupVariant::Standard] {
        for selection in [Selection::single().with([0]), Selection::multiple().with([0])] {
            let single = selection.mode() == egui_mineral::select::SelectionMode::Single;
            let why = format!("{variant:?} {:?}", selection.mode());
            let mut h = button_group(variant, Some(selection.clone()));
            h.press(Key::Tab);
            assert!(h.focused("grp-0-button"), "{why}: Tab enters at the selected item");
            h.press(Key::Tab);
            assert!(h.focused("after-button"), "{why}: the next Tab leaves the group");

            let mut h = button_group(variant, Some(selection));
            h.press(Key::Tab);
            h.press(Key::ArrowRight);
            assert!(h.focused("grp-2-button"), "{why}: arrows skip the disabled item");
            let selected = h.state().value.as_ref().is_some_and(|s| s.is_selected(&2));
            assert_eq!(selected, single, "{why}: arrows select only in a radio group");
            h.press(Key::Tab);
            assert!(h.focused("after-button"), "{why}: Tab leaves from the roving item");
        }
    }
}

#[test]
fn plain_button_group_keeps_a_stop_per_button() {
    let mut h = button_group(ButtonGroupVariant::Standard, None);
    for key in ["grp-0-button", "grp-2-button", "after-button"] {
        h.press(Key::Tab);
        assert!(h.focused(key), "Tab reaches {key}");
    }
}

#[test]
fn input_chip_reports_removed_apart_from_selection() {
    // (removed, changed, clicked)
    let mut h = harness(SCREEN, (false, false, false), |ui, (removed, changed, clicked)| {
        let r = ui.mineral(Chip::input("tag", "A long tag label"));
        *removed |= r.output().removed;
        *changed |= r.changed();
        *clicked |= r.clicked();
    });
    h.click("tag-chip-remove");
    assert_eq!(h.state().value, (true, false, false), "remove icon");
    h.state_mut().value = (false, false, false);
    h.click("tag-chip");
    assert_eq!(
        h.state().value,
        (false, false, true),
        "the body clicks without removing"
    );
}

// (selected, clicked, changed)
fn radio() -> Harness<'static, (bool, u32, u32)> {
    harness(SCREEN, (false, 0, 0), |ui, (selected, clicked, changed)| {
        let r = ui.mineral(Radio::new("r", "Radio").selected(*selected));
        *clicked += u32::from(r.clicked());
        *changed += u32::from(r.changed());
        *selected |= r.changed();
    })
}

#[test]
fn radio_click_selects_once() {
    let mut h = radio();
    h.click("r-radio");
    assert_eq!(h.state().value, (true, 1, 1), "click selects");
    assert!(h.node("r-radio").state.checked.is_some_and(|c| c == true.into()));
    h.click("r-radio");
    assert_eq!(
        h.state().value,
        (true, 2, 1),
        "a selected radio clicks without changing"
    );
}

#[test]
fn radio_space_selects_the_focused_radio() {
    let mut h = radio();
    h.press(Key::Tab);
    assert!(h.focused("r-radio"));
    h.press(Key::Space);
    assert_eq!(h.state().value, (true, 1, 1));
}

#[test]
fn standard_button_group_wraps_when_narrow() {
    let h = harness(vec2(200.0, 400.0), (), |ui, ()| {
        ButtonGroup::new("grp", "Words")
            .items(["Alpha", "Bravo", "Charlie", "Delta"].map(ButtonGroupItem::new))
            .show(ui);
    });
    let (first, last) = (h.rect("grp-0-button"), h.rect("grp-3-button"));
    assert!(last.top() > first.bottom(), "the last button wraps below the first");
    assert!(last.right() <= 200.0, "nothing overflows the width");
}

// (single selected, multi selection, single changes)
type Segments = (usize, Selection<usize>, u32);

fn segmented() -> Harness<'static, Segments> {
    harness(SCREEN, (0, Selection::multiple(), 0), |ui, (single, multi, changes)| {
        let r = SegmentedButton::new("one", "Period", single)
            .items(
                ([
                    Segment::new("Day"),
                    Segment::new("Week").disabled(true),
                    Segment::new("Month"),
                ])
                .into_iter()
                .enumerate(),
            )
            .show(ui);
        assert_eq!(r.changed, r.response.changed(), "response marks the change");
        *changes += u32::from(r.changed);
        SegmentedButton::multi("many", "Days", multi)
            .items(["Mon", "Tue", "Wed"].into_iter().enumerate())
            .show(ui);
        ui.mineral(PushButton::new("after", "After"));
    })
}

#[test]
fn single_segmented_button_click_selects_and_disabled_ignores() {
    let mut h = segmented();
    h.click("one-2-segment");
    assert_eq!((h.state().value.0, h.state().value.2), (2, 1), "click selects");
    h.click("one-2-segment");
    assert_eq!(h.state().value.2, 1, "the selected segment clicks without changing");
    h.click("one-1-segment");
    assert_eq!(h.state().value.0, 2, "a disabled segment ignores clicks");
    let node = h.node("one-2-segment");
    assert_eq!(node.role, Role::RadioButton);
    assert_eq!(node.state.selected, Some(true));
    assert_eq!(h.node("one-segmented-button").role, Role::RadioGroup);
}

#[test]
fn multi_segmented_button_toggles_apart_from_its_sibling() {
    let mut h = segmented();
    h.click("many-0-segment");
    h.click("many-2-segment");
    h.click("many-0-segment");
    let multi: Vec<usize> = h.state().value.1.selected().copied().collect();
    assert_eq!(multi, [2], "clicks toggle");
    assert_eq!(h.state().value.0, 0, "the sibling row keeps its selection");
    assert_eq!(h.node("many-2-segment").role, Role::CheckBox);
    assert!(h.node("many-2-segment").state.checked.is_some_and(|c| c == true.into()));
    assert_ne!(
        h.node("one-0-segment").widget_id,
        h.node("many-0-segment").widget_id,
        "sibling rows salt their segment ids"
    );
}

#[test]
fn segmented_button_is_one_tab_stop_and_arrows_select_in_single_mode() {
    let mut h = segmented();
    h.press(Key::Tab);
    assert!(h.focused("one-0-segment"), "Tab enters at the selected segment");
    h.press(Key::ArrowRight);
    assert!(h.focused("one-2-segment"), "arrows skip the disabled segment");
    assert_eq!(h.state().value.0, 2, "arrows select");
    h.press(Key::Tab);
    assert!(h.focused("many-0-segment"), "the next Tab enters the sibling row");
    h.press(Key::ArrowRight);
    assert!(h.focused("many-1-segment"));
    assert!(h.state().value.1.is_empty(), "arrows only move focus among checkboxes");
    h.press(Key::Tab);
    assert!(h.focused("after-button"), "Tab leaves the row");
}

#[test]
fn segmented_button_click_released_in_one_pass_hands_arrows_to_the_segment() {
    let mut h = segmented();
    let at = h.rect("one-2-segment").center();
    h.click_at_once(at);
    assert_eq!(h.state().value.0, 2, "the click selects");
    assert!(h.focused("one-2-segment"), "the click focuses");
    h.press(Key::ArrowLeft);
    assert_eq!(h.state().value.0, 0, "arrows select from the clicked segment");
}

#[test]
fn optional_single_segments_start_empty_and_content_width_hugs_labels() {
    let mut h = harness(SCREEN, None, |ui, picked: &mut Option<usize>| {
        let items = ["Field", "Both", "Intensity"];
        SegmentedButton::optional("opt", "View", picked)
            .equal_width(false)
            .items(items.into_iter().enumerate())
            .show(ui);
        let mut equal = 0;
        SegmentedButton::new("eq", "View", &mut equal)
            .items(items.into_iter().enumerate())
            .show(ui);
    });
    assert_eq!(h.state().value, None, "nothing selected");
    let (opt, eq) = (h.rect("opt-1-segment").width(), h.rect("eq-1-segment").width());
    assert!(opt < eq, "content width {opt} under the widest {eq}");
    h.click("opt-1-segment");
    assert_eq!(h.state().value, Some(1));
}

#[test]
fn auto_targets_keep_their_height_through_ewebs_touchless_pointerdown_pass() {
    let pos = egui::pos2(20.0, 20.0);
    let mut h = harness(SCREEN, Vec::<f32>::new(), |ui, heights| {
        let hit = ui.target(MinTarget::Auto);
        heights.push(Zones::new(Measure::new(vec2(20.0, 20.0)), hit).outer.height());
    });
    let touch = |phase| synth::touch(pos, phase);
    let end = |h: &mut Harness<'_, Vec<f32>>| {
        h.input_mut().events.extend([
            synth::pointer_up(pos, PointerButton::Primary),
            Event::PointerGone,
            touch(TouchPhase::End),
        ]);
        h.step();
    };
    h.input_mut().events.push(touch(TouchPhase::Start));
    h.step();
    end(&mut h);
    h.state_mut().value.clear();
    // pass 1: pointerdown alone, pass 2: touchstart
    h.input_mut()
        .events
        .push(synth::pointer_down(pos, PointerButton::Primary));
    h.step();
    h.input_mut()
        .events
        .extend([touch(TouchPhase::Start), Event::PointerMoved(pos)]);
    h.step();
    end(&mut h);
    let touch_target = h.rt().core().policies().target.touch;
    let heights = &h.state().value;
    assert_eq!(heights.len(), 3);
    assert!(
        heights.iter().all(|&v| (v - touch_target).abs() < f32::EPSILON),
        "{heights:?}"
    );
}

#[test]
fn segmented_button_value_binds_the_segment_value() {
    let mut h = harness(SCREEN, ('w', 0), |ui, (period, changes)| {
        let segments = [('d', "Day"), ('w', "Week"), ('m', "Month")];
        let r = SegmentedButton::new("period", "Period", period)
            .items(segments)
            .show(ui);
        *changes += u32::from(r.changed);
    });
    assert_eq!(
        h.node("period-1-segment").state.selected,
        Some(true),
        "the segment equal to the value is selected"
    );
    h.click("period-2-segment");
    h.click("period-2-segment");
    assert_eq!(h.state().value, ('m', 1));
}

#[test]
fn button_group_reports_the_item_key() {
    let mut h = harness(SCREEN, None, |ui, clicked| {
        let r = ButtonGroup::new("grp", "Actions")
            .items([ButtonGroupItem::new("Copy").key("copy"), ButtonGroupItem::new("Paste")])
            .show(ui);
        if r.clicked.is_some() {
            *clicked = r.clicked;
        }
    });
    h.click("grp-0-button");
    let copy = h.state().value.clone();
    assert_eq!(copy.as_ref().and_then(|c| c.key::<&str>().copied()), Some("copy"));
    h.click("grp-1-button");
    let paste = h.state().value.clone();
    assert_eq!(
        paste.map(|c| (c.index(), c.key::<&str>().is_none())),
        Some((1, true)),
        "an item without a key"
    );
}

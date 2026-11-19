use egui_mineral::{
    semantic::{Live, Role},
    toast::ToastQueue,
};
use egui_mineral_fluent::feedback::{MessageBar, ProgressBar, Toast, Toaster, toast_queue};

use crate::support::{FluentHarness, harness};

#[test]
fn message_bar_reports_its_dismiss_button() {
    let mut h = harness(false, |ui, dismissed| {
        let r = MessageBar::new("m", "Body").title("Title").dismissible(true).show(ui);
        *dismissed |= r.dismissed;
    });
    let node = h.node("m-message-bar");
    assert_eq!((node.name.as_str(), node.value.as_deref()), ("Title", Some("Body")));
    let at = h
        .tree()
        .nodes
        .iter()
        .find(|n| n.name == "Dismiss")
        .map(|n| n.rect.center())
        .expect("a dismiss button");
    h.click_at(at);
    assert!(h.state().value);
}

#[test]
fn message_bar_runs_its_actions_closure() {
    let h = harness(0, |ui, runs| {
        let r = MessageBar::new("m", "Body").show_with(ui, |_| 7);
        *runs = r.inner.unwrap_or_default();
    });
    assert_eq!(h.state().value, 7);
}

#[test]
fn progress_bar_exposes_its_value_as_a_range() {
    let h = harness((), |ui, ()| {
        ui.mineral(ProgressBar::determinate("p", 0.25).label("Upload"));
        ui.mineral(ProgressBar::indeterminate("q"));
    });
    let node = h.node("p-progress-bar");
    assert_eq!(node.name, "Upload");
    assert_eq!(node.value.as_deref(), Some("25%"));
    assert_eq!(node.state.range.map(|r| r.value), Some(0.25));
    assert!(h.node("q-progress-bar").state.range.is_none());
}

fn toasting() -> ToastQueue<Toast> {
    let mut queue = toast_queue();
    queue.push(Toast::new("Saved").body("Done").dismissible(true));
    queue
}

#[test]
fn toast_node_carries_title_body_and_politeness() {
    let h = harness(toasting(), |ui, queue| {
        Toaster::new(queue).show(ui.rt());
    });
    let node = h.node("toast-0");
    assert_eq!((node.name.as_str(), node.value.as_deref()), ("Saved", Some("Done")));
    assert_eq!(node.state.live, Some(Live::Polite));
}

#[test]
fn dismiss_link_removes_the_toast() {
    let mut h = harness(toasting(), |ui, queue| {
        Toaster::new(queue).show(ui.rt());
    });
    let at = h
        .tree()
        .nodes
        .iter()
        .find(|n| n.role == Role::Link)
        .map(|n| n.rect.center())
        .expect("a dismiss link");
    h.click_at(at);
    h.run();
    assert_eq!(h.state().value.visible().count(), 0);
    assert!(h.tree().find_key("toast-0").is_none());
}

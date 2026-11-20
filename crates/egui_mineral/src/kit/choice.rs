//! Selection groups whose items carry app values: the binding behind every [`SelectionResponse`].
//!
//! A widget pushes each item's value in display order, resolves the selected positions while painting and hands
//! the position picked this pass to [`Choice::pick`], which writes the item's value to the app's binding.

use crate::select::{Selection, SelectionResponse};

trait Many<K> {
    fn is_single_mode(&self) -> bool;
    fn holds(&self, value: &K) -> bool;
    fn flip(&mut self, value: K) -> bool;
}

impl<K: Ord + Clone> Many<K> for Selection<K> {
    fn is_single_mode(&self) -> bool {
        self.mode() == crate::select::SelectionMode::Single
    }

    fn holds(&self, value: &K) -> bool {
        self.is_selected(value)
    }

    fn flip(&mut self, value: K) -> bool {
        self.toggle(value)
    }
}

enum Bound<'a, K> {
    One(&'a mut K),
    Optional(&'a mut Option<K>),
    Many(&'a mut dyn Many<K>),
}

/// App binding of a selection group plus the value of each item.
pub struct Choice<'a, K> {
    bound: Bound<'a, K>,
    values: Vec<K>,
}

crate::kit::opaque_debug!(Choice<'_, K>);

impl<'a, K: PartialEq + Clone> Choice<'a, K> {
    /// Exactly one item selected: the one whose value equals `bound`.
    pub fn one(bound: &'a mut K) -> Self {
        Self::new(Bound::One(bound))
    }

    /// At most one item selected; `None` selects none.
    pub fn optional(bound: &'a mut Option<K>) -> Self {
        Self::new(Bound::Optional(bound))
    }

    /// Any number of items selected; a pick toggles the item.
    pub fn many(bound: &'a mut Selection<K>) -> Self
    where
        K: Ord,
    {
        Self::new(Bound::Many(bound))
    }

    const fn new(bound: Bound<'a, K>) -> Self {
        Self {
            bound,
            values: Vec::new(),
        }
    }

    /// Appends the value of the next item.
    pub fn push(&mut self, value: K) {
        self.values.push(value);
    }

    /// Item values in display order.
    pub fn values(&self) -> &[K] {
        &self.values
    }

    /// At most one item selected: [`Self::one`], [`Self::optional`] and [`Self::many`] of a
    /// [`Selection::single`].
    pub fn is_single(&self) -> bool {
        match &self.bound {
            Bound::One(_) | Bound::Optional(_) => true,
            Bound::Many(b) => b.is_single_mode(),
        }
    }

    /// `true` when item `index` carries a selected value.
    pub fn is_selected(&self, index: usize) -> bool {
        self.values.get(index).is_some_and(|v| match &self.bound {
            Bound::One(b) => v == &**b,
            Bound::Optional(b) => b.as_ref() == Some(v),
            Bound::Many(b) => b.holds(v),
        })
    }

    /// First selected item; `None` when no item carries a selected value.
    pub fn index(&self) -> Option<usize> {
        (0..self.values.len()).find(|&i| self.is_selected(i))
    }

    /// Writes the value of item `index` (toggles it for [`Self::many`]); whether the binding changed.
    pub fn select(&mut self, index: usize) -> bool {
        let Some(value) = self.values.get(index).cloned() else {
            return false;
        };
        match &mut self.bound {
            Bound::One(b) => {
                let changed = **b != value;
                **b = value;
                changed
            }
            Bound::Optional(b) => b.replace(value.clone()).as_ref() != Some(&value),
            Bound::Many(b) => b.flip(value),
        }
    }

    /// [`Self::select`]s item `picked` (a click or key this pass); `None` reports no pick.
    pub fn pick(&mut self, response: egui::Response, picked: Option<usize>) -> SelectionResponse<K> {
        let selected = picked.and_then(|i| self.values.get(i)).cloned();
        let changed = picked.is_some_and(|i| self.select(i));
        SelectionResponse::new(response, selected, changed)
    }
}

#[cfg(test)]
mod tests {
    use super::Choice;
    use crate::select::Selection;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum Tab {
        Home,
        Mail,
        Chat,
    }

    fn response() -> egui::Response {
        let ctx = egui::Context::default();
        let mut out = None;
        let mut full = ctx.run_ui(egui::RawInput::default(), |ui| out = Some(ui.label("")));
        full.textures_delta.clear();
        out.unwrap()
    }

    fn with_tabs<K>(mut c: Choice<'_, K>, values: impl IntoIterator<Item = K>) -> Choice<'_, K>
    where
        K: PartialEq + Clone,
    {
        for v in values {
            c.push(v);
        }
        c
    }

    #[test]
    fn one_follows_the_value_not_the_position() {
        let mut tab = Tab::Mail;
        let mut c = with_tabs(Choice::one(&mut tab), [Tab::Chat, Tab::Mail, Tab::Home]);
        assert_eq!(c.index(), Some(1));
        let r = c.pick(response(), Some(2));
        assert!(r.changed);
        assert_eq!(r.selected, Some(Tab::Home));
        let r = c.pick(response(), Some(2));
        assert!(!r.changed, "picking the selected item keeps it");
        assert_eq!(tab, Tab::Home);
    }

    #[test]
    fn unmatched_values_select_nothing() {
        let mut tab = Tab::Chat;
        let c = with_tabs(Choice::one(&mut tab), [Tab::Home, Tab::Mail]);
        assert_eq!(c.index(), None);
        let mut none = None;
        let c = with_tabs(Choice::optional(&mut none), [Tab::Home]);
        assert_eq!(c.index(), None);
    }

    #[test]
    fn many_toggles() {
        let mut sel: Selection<Tab> = [Tab::Home].into_iter().collect();
        let mut c = with_tabs(Choice::many(&mut sel), [Tab::Home, Tab::Mail]);
        assert!(!c.is_single());
        assert!(c.pick(response(), Some(1)).changed);
        assert!(c.pick(response(), Some(0)).changed);
        assert!(sel.is_selected(&Tab::Mail) && !sel.is_selected(&Tab::Home));
    }
}

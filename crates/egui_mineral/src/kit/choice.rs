//! Selection groups whose items carry app values: the binding behind every [`SelectionResponse`].
//!
//! A widget pushes each item's value in display order, resolves the selected positions while painting and hands
//! the position picked this pass to [`Choice::pick`], which writes the item's value to the app's binding.

use crate::select::{ItemKey, Selection, SelectionResponse};

trait Many<K> {
    fn is_single_mode(&self) -> bool;
    fn held(&self) -> Box<dyn Iterator<Item = &K> + '_>;
    fn holds(&self, value: &K) -> bool;
    fn flip(&mut self, value: K) -> bool;
}

impl<K: ItemKey> Many<K> for Selection<K> {
    fn is_single_mode(&self) -> bool {
        self.mode() == crate::select::SelectionMode::Single
    }

    fn held(&self) -> Box<dyn Iterator<Item = &K> + '_> {
        Box::new(Selection::selected(self))
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

impl<'a, K: ItemKey> Choice<'a, K> {
    /// Exactly one item selected: the one whose value equals `bound`.
    pub fn new(bound: &'a mut K) -> Self {
        Self::with(Bound::One(bound))
    }

    /// At most one item selected; `None` selects none.
    pub fn optional(bound: &'a mut Option<K>) -> Self {
        Self::with(Bound::Optional(bound))
    }

    /// Any number of items selected, at most one for a [`Selection::single`]; a pick toggles the item.
    pub fn multi(bound: &'a mut Selection<K>) -> Self {
        Self::with(Bound::Many(bound))
    }

    const fn with(bound: Bound<'a, K>) -> Self {
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

    /// Value of item `index`.
    pub fn value(&self, index: usize) -> Option<&K> {
        self.values.get(index)
    }

    /// At most one item selected: [`Self::new`], [`Self::optional`] and [`Self::multi`] of a
    /// [`Selection::single`].
    pub fn is_single(&self) -> bool {
        match &self.bound {
            Bound::One(_) | Bound::Optional(_) => true,
            Bound::Many(b) => b.is_single_mode(),
        }
    }

    /// `true` when item `index` carries a selected value.
    pub fn is_selected(&self, index: usize) -> bool {
        self.values.get(index).is_some_and(|v| self.contains(v))
    }

    /// Selected values, whether or not an item carries them.
    pub fn selected(&self) -> Box<dyn Iterator<Item = &K> + '_> {
        match &self.bound {
            Bound::One(b) => Box::new(std::iter::once(&**b)),
            Bound::Optional(b) => Box::new(b.iter()),
            Bound::Many(b) => b.held(),
        }
    }

    /// `true` when `value` is selected, whether or not an item carries it.
    pub fn contains(&self, value: &K) -> bool {
        match &self.bound {
            Bound::One(b) => value == &**b,
            Bound::Optional(b) => b.as_ref() == Some(value),
            Bound::Many(b) => b.holds(value),
        }
    }

    /// Writes `value` (toggles it for [`Self::multi`]); whether the binding changed. For groups that track item
    /// values themselves (nested navigation, grouped lists).
    pub fn set(&mut self, value: K) -> bool {
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

    /// First selected item; `None` when no item carries a selected value.
    pub fn index(&self) -> Option<usize> {
        (0..self.values.len()).find(|&i| self.is_selected(i))
    }

    /// Writes the value of item `index` (toggles it for [`Self::multi`]); whether the binding changed.
    pub fn select(&mut self, index: usize) -> bool {
        self.values.get(index).cloned().is_some_and(|value| self.set(value))
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

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, crate::semantic::KeySalt)]
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
        K: crate::select::ItemKey,
    {
        for v in values {
            c.push(v);
        }
        c
    }

    #[test]
    fn one_follows_the_value_not_the_position() {
        let mut tab = Tab::Mail;
        let mut c = with_tabs(Choice::new(&mut tab), [Tab::Chat, Tab::Mail, Tab::Home]);
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
        let c = with_tabs(Choice::new(&mut tab), [Tab::Home, Tab::Mail]);
        assert_eq!(c.index(), None);
        let mut none = None;
        let c = with_tabs(Choice::optional(&mut none), [Tab::Home]);
        assert_eq!(c.index(), None);
    }

    #[test]
    fn many_toggles() {
        let mut sel: Selection<Tab> = [Tab::Home].into_iter().collect();
        let mut c = with_tabs(Choice::multi(&mut sel), [Tab::Home, Tab::Mail]);
        assert!(!c.is_single());
        assert!(c.pick(response(), Some(1)).changed);
        assert!(c.pick(response(), Some(0)).changed);
        assert!(sel.is_selected(&Tab::Mail) && !sel.is_selected(&Tab::Home));
    }
}

use egui_mineral::kit;

trait Bound {
    fn selected(&self) -> Option<usize>;
    fn select(&mut self, index: usize) -> bool;
}

struct ByValue<'a, T> {
    value: &'a mut T,
    values: Vec<T>,
}

impl<T: PartialEq + Clone> Bound for ByValue<'_, T> {
    fn selected(&self) -> Option<usize> {
        self.values.iter().position(|v| v == &*self.value)
    }

    fn select(&mut self, index: usize) -> bool {
        match self.values.get(index) {
            Some(v) if v != &*self.value => {
                *self.value = v.clone();
                true
            }
            _ => false,
        }
    }
}

enum Binding<'a> {
    Index(&'a mut usize),
    Value(Box<dyn Bound + 'a>),
}

// Single choice bound by item index or by the value of each item.
pub(crate) struct Choice<'a>(Binding<'a>);

kit::opaque_debug!(Choice<'_>);

impl<'a> Choice<'a> {
    pub(crate) fn by_index(index: &'a mut usize) -> Self {
        Self(Binding::Index(index))
    }

    // `values[i]` is the value of item `i`.
    pub(crate) fn by_value<T: PartialEq + Clone + 'a>(value: &'a mut T, values: Vec<T>) -> Self {
        Self(Binding::Value(Box::new(ByValue { value, values })))
    }

    // A bound index past the last item moves onto it.
    #[track_caller]
    pub(crate) fn clamp(&mut self, len: usize, what: &str) {
        if let Binding::Index(i) = &mut self.0
            && let Some(valid) = kit::sanitize::index(**i, len, what)
        {
            **i = valid;
        }
    }

    pub(crate) fn selected(&self) -> Option<usize> {
        match &self.0 {
            Binding::Index(i) => Some(**i),
            Binding::Value(b) => b.selected(),
        }
    }

    // Whether the bound value changed.
    pub(crate) fn select(&mut self, index: usize) -> bool {
        match &mut self.0 {
            Binding::Index(i) => {
                let changed = **i != index;
                **i = index;
                changed
            }
            Binding::Value(b) => b.select(index),
        }
    }
}

// Items and their values, split for `Choice::by_value`.
pub(crate) fn split<T, Item>(pairs: impl IntoIterator<Item = (T, impl Into<Item>)>) -> (Vec<T>, Vec<Item>) {
    pairs.into_iter().map(|(v, i)| (v, i.into())).unzip()
}

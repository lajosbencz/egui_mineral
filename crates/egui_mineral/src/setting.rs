use std::marker::PhantomData;

use egui::{Context, Id};

// Per-viewport value of type `T` under a name; the handle ties the name to the type.
pub(crate) struct Setting<T> {
    name: &'static str,
    value: PhantomData<fn() -> T>,
}

impl<T: Clone + Default + Send + Sync + 'static> Setting<T> {
    pub(crate) const fn new(name: &'static str) -> Self {
        Self {
            name,
            value: PhantomData,
        }
    }

    pub(crate) fn get(&self, ctx: &Context) -> T {
        let id = self.key(ctx);
        ctx.data(|d| d.get_temp(id)).unwrap_or_default()
    }

    pub(crate) fn set(&self, ctx: &Context, value: T) {
        let id = self.key(ctx);
        ctx.data_mut(|d| d.insert_temp(id, value));
    }

    fn key(&self, ctx: &Context) -> Id {
        Id::unique((self.name, ctx.viewport_id()))
    }
}

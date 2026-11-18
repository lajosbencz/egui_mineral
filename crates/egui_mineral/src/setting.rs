use egui::{Context, Id};

pub(crate) fn get<T: Clone + Default + Send + Sync + 'static>(ctx: &Context, name: &'static str) -> T {
    let id = key(ctx, name);
    ctx.data(|d| d.get_temp(id)).unwrap_or_default()
}

pub(crate) fn set<T: Clone + Send + Sync + 'static>(ctx: &Context, name: &'static str, value: T) {
    let id = key(ctx, name);
    ctx.data_mut(|d| d.insert_temp(id, value));
}

fn key(ctx: &Context, name: &'static str) -> Id {
    Id::unique((name, ctx.viewport_id()))
}

use egui::{Context, ViewportId};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};

/// `Send` on native, local on wasm (browser futures are `!Send`).
#[cfg(not(target_arch = "wasm32"))]
pub type BoxFuture<T = ()> = futures_util::future::BoxFuture<'static, T>;
/// `Send` on native, local on wasm (browser futures are `!Send`).
#[cfg(target_arch = "wasm32")]
pub type BoxFuture<T = ()> = futures_util::future::LocalBoxFuture<'static, T>;

/// `Send` on native, no bound on wasm.
#[cfg(not(target_arch = "wasm32"))]
pub trait MaybeSend: Send {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send> MaybeSend for T {}
/// `Send` on native, no bound on wasm.
#[cfg(target_arch = "wasm32")]
pub trait MaybeSend {}
#[cfg(target_arch = "wasm32")]
impl<T> MaybeSend for T {}

/// Runs futures for the app; supplied by the app's executor.
pub trait Spawner {
    /// Must poll `fut` to completion; the core never spawns on its own.
    fn spawn(&self, fut: BoxFuture);
}

/// Spawn helpers over [`Spawner`].
pub trait SpawnerExt: Spawner {
    /// Boxes and spawns `fut`.
    fn spawn_future(&self, fut: impl Future<Output = ()> + MaybeSend + 'static) {
        self.spawn(Box::pin(fut));
    }

    /// Spawns `fut` and sends its output through `events`; dropped silently when the UI side is gone.
    fn spawn_event<E: MaybeSend + 'static>(
        &self,
        events: &EventSender<E>,
        fut: impl Future<Output = E> + MaybeSend + 'static,
    ) {
        let tx = events.clone();
        self.spawn(Box::pin(async move {
            if tx.send(fut.await).is_err() {
                tracing::debug!("spawn_event: UI side dropped, event discarded");
            }
        }));
    }
}

impl<S: Spawner + ?Sized> SpawnerExt for S {}

/// Send failure on a closed [`Bus`] channel; carries the undelivered value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Closed<T>(pub T);

impl<T> std::fmt::Display for Closed<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("channel closed")
    }
}

impl<T: std::fmt::Debug> std::error::Error for Closed<T> {}

/// Executor-agnostic command and event channels between UI and async handlers; the UI end.
#[derive(Debug)]
pub struct Bus<C, E> {
    commands: UnboundedSender<C>,
    events: UnboundedReceiver<E>,
    sender: EventSender<E>,
}

/// Handler end of a [`Bus`].
#[derive(Debug)]
#[non_exhaustive]
pub struct BusHandle<C, E> {
    /// Stream of UI commands; ends when the [`Bus`] drops.
    pub commands: CommandReceiver<C>,
    /// Wakes the UI with each sent event.
    pub events: EventSender<E>,
}

/// Commands sent through [`Bus::send`]; ends when the bus is dropped.
#[derive(Debug)]
pub struct CommandReceiver<C>(UnboundedReceiver<C>);

impl<C> futures_util::Stream for CommandReceiver<C> {
    type Item = C;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<C>> {
        std::pin::Pin::new(&mut self.0).poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

/// Wakes the UI on every sent event; the only repaint trigger of the bus.
#[derive(Debug)]
pub struct EventSender<E> {
    tx: UnboundedSender<E>,
    ctx: Context,
    viewport: ViewportId,
}

impl<E> Clone for EventSender<E> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            ctx: self.ctx.clone(),
            viewport: self.viewport,
        }
    }
}

impl<C, E> Bus<C, E> {
    /// Events sent through the returned [`BusHandle`] wake the root viewport of `ctx`; see [`EventSender::in_viewport`].
    pub fn new(ctx: &Context) -> (Self, BusHandle<C, E>) {
        let (ctx_tx, ctx_rx) = unbounded();
        let (evt_tx, evt_rx) = unbounded();
        let sender = EventSender {
            tx: evt_tx,
            ctx: ctx.clone(),
            viewport: ViewportId::ROOT,
        };
        (
            Self {
                commands: ctx_tx,
                events: evt_rx,
                sender: sender.clone(),
            },
            BusHandle {
                commands: CommandReceiver(ctx_rx),
                events: sender,
            },
        )
    }

    /// Queues `command` for the handler.
    ///
    /// # Errors
    /// [`Closed`] once the handle's receiver is dropped.
    pub fn send(&self, command: C) -> Result<(), Closed<C>> {
        self.commands
            .unbounded_send(command)
            .map_err(|e| Closed(e.into_inner()))
    }

    /// Extra sender for UI-side producers; wakes the UI like the handle's.
    pub fn sender(&self) -> EventSender<E> {
        self.sender.clone()
    }

    /// Events received since the last pass; never blocks. The bus holds a sender itself, so the
    /// channel never closes while it lives.
    pub fn drain(&mut self) -> impl Iterator<Item = E> + '_ {
        std::iter::from_fn(|| self.events.try_recv().ok())
    }
}

impl<E> EventSender<E> {
    /// Sender whose events wake `viewport` instead.
    #[must_use]
    pub fn in_viewport(mut self, viewport: ViewportId) -> Self {
        self.viewport = viewport;
        self
    }

    /// Queues `event` and requests a repaint of the sender's viewport.
    ///
    /// # Errors
    /// [`Closed`] once the [`Bus`] is dropped.
    pub fn send(&self, event: E) -> Result<(), Closed<E>> {
        self.tx.unbounded_send(event).map_err(|e| Closed(e.into_inner()))?;
        self.ctx.request_repaint_of(self.viewport);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use egui::{Context, ViewportId};
    use futures_util::{FutureExt as _, StreamExt as _};

    use super::{BoxFuture, Bus, Closed, Spawner, SpawnerExt as _};

    struct Inline;

    impl Spawner for Inline {
        fn spawn(&self, fut: BoxFuture) {
            assert!(fut.now_or_never().is_some(), "test futures complete on first poll");
        }
    }

    #[test]
    fn commands_and_spawned_events_are_delivered() {
        let ctx = Context::default();
        let (mut bus, mut handle) = Bus::<u8, u8>::new(&ctx);
        bus.send(1).unwrap();
        assert_eq!(handle.commands.next().now_or_never(), Some(Some(1)));

        Inline.spawn_event(&handle.events, async { 2 });
        assert!(ctx.has_requested_repaint(), "an event wakes the UI");
        assert_eq!(bus.drain().collect::<Vec<_>>(), [2]);

        drop(handle);
        assert_eq!(bus.send(3), Err(Closed(3)), "commands fail once the handle is gone");
    }

    #[test]
    fn events_wake_the_senders_viewport() {
        let ctx = Context::default();
        let other = ViewportId::from_hash_of("other");
        let (bus, _handle) = Bus::<u8, u8>::new(&ctx);
        bus.sender().in_viewport(other).send(1).unwrap();
        assert!(ctx.has_requested_repaint_for(&other));
    }
}

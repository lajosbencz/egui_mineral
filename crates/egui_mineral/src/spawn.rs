//! [`Spawner`](crate::bus::Spawner) adapters: plain threads, or opt-in over the app's executor.
//!
//! An adapter owns the tasks it spawned: dropping it cancels them (`docs/rules.md` rules 9 and 11).
//!
//! | Feature | Target | Adapter |
//! |---|---|---|
//! | `spawn-tokio` | native | `TokioSpawner` over a `tokio::runtime::Handle` |
//! | `spawn-wasm` | wasm32 | `WasmSpawner` over `wasm_bindgen_futures::spawn_local` |
//! | | native | `ThreadSpawner`, one OS thread per task |

#[cfg(not(target_arch = "wasm32"))]
pub use self::thread::ThreadSpawner;
#[cfg(all(feature = "spawn-tokio", not(target_arch = "wasm32")))]
pub use self::tokio::TokioSpawner;
#[cfg(all(feature = "spawn-wasm", target_arch = "wasm32"))]
pub use self::wasm::WasmSpawner;

#[cfg(all(feature = "spawn-tokio", not(target_arch = "wasm32")))]
mod tokio {
    use ::tokio::{
        runtime::Handle,
        task::{JoinError, JoinSet},
    };
    use parking_lot::Mutex;

    use crate::bus::{BlockingJob, BlockingSpawner, BoxFuture, Spawner};

    /// [`Spawner`] and [`BlockingSpawner`] over the app's Tokio runtime.
    ///
    /// Owns the tasks it spawned: dropping it aborts them, [`Self::shutdown`] also waits for them to end. A
    /// blocking job that already started cannot be aborted and runs to completion on Tokio's blocking pool,
    /// which the runtime joins when the app drops it. A task that panicked is reported to `tracing`.
    ///
    /// ```
    /// use std::sync::Arc;
    ///
    /// use egui_mineral::{bus::SpawnerExt, spawn::TokioSpawner};
    ///
    /// let runtime = tokio::runtime::Builder::new_current_thread().build()?;
    /// // Shared by the bus handlers and `NativePlatform::new(Arc::clone(&spawner))`.
    /// let spawner = Arc::new(TokioSpawner::new(runtime.handle().clone()));
    /// spawner.spawn_future(async {});
    /// runtime.block_on(spawner.shutdown());
    /// # Ok::<(), std::io::Error>(())
    /// ```
    #[derive(Debug)]
    pub struct TokioSpawner {
        handle: Handle,
        tasks: Mutex<JoinSet<()>>,
    }

    impl TokioSpawner {
        /// Spawns on the runtime of `handle`, which the app keeps alive.
        pub fn new(handle: Handle) -> Self {
            Self {
                handle,
                tasks: Mutex::new(JoinSet::new()),
            }
        }

        /// Runtime the tasks run on.
        pub fn handle(&self) -> &Handle {
            &self.handle
        }

        /// Tasks spawned and not yet finished.
        pub fn running(&self) -> usize {
            let mut tasks = self.tasks.lock();
            reap(&mut tasks);
            tasks.len()
        }

        /// Aborts every task and waits until all have ended; tasks spawned meanwhile are kept.
        pub async fn shutdown(&self) {
            let mut tasks = std::mem::take(&mut *self.tasks.lock());
            tasks.abort_all();
            while let Some(ended) = tasks.join_next().await {
                report(ended);
            }
        }
    }

    fn report(ended: Result<(), JoinError>) {
        if let Err(e) = ended
            && e.is_panic()
        {
            tracing::error!(target: "egui_mineral", error = %e, "spawned task panicked");
        }
    }

    fn reap(tasks: &mut JoinSet<()>) {
        while let Some(ended) = tasks.try_join_next() {
            report(ended);
        }
    }

    impl Spawner for TokioSpawner {
        fn spawn(&self, fut: BoxFuture) {
            let mut tasks = self.tasks.lock();
            reap(&mut tasks);
            #[expect(
                clippy::disallowed_methods,
                reason = "docs/rules.md rule 11: the adapter forwards to the app's runtime and owns the task"
            )]
            tasks.spawn_on(fut, &self.handle);
        }
    }

    impl BlockingSpawner for TokioSpawner {
        fn spawn_blocking(&self, job: BlockingJob) {
            let mut tasks = self.tasks.lock();
            reap(&mut tasks);
            #[expect(
                clippy::disallowed_methods,
                reason = "docs/rules.md rule 11: the adapter forwards to the app's runtime and owns the task"
            )]
            tasks.spawn_blocking_on(job, &self.handle);
        }
    }

    impl Drop for TokioSpawner {
        fn drop(&mut self) {
            reap(self.tasks.get_mut());
        }
    }

    #[cfg(test)]
    mod tests {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc,
        };

        use ::tokio::runtime::Builder;

        use super::TokioSpawner;
        use crate::bus::{BlockingSpawner as _, SpawnerExt as _};

        struct SetOnDrop(Arc<AtomicBool>);

        impl Drop for SetOnDrop {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }

        #[test]
        fn runs_futures_and_blocking_jobs_on_the_apps_runtime() {
            let runtime = Builder::new_current_thread().build().unwrap();
            let spawner = TokioSpawner::new(runtime.handle().clone());
            let (tx, rx) = mpsc::channel();
            let blocking = tx.clone();
            spawner.spawn_future(async move { tx.send(1).unwrap() });
            spawner.spawn_blocking(Box::new(move || blocking.send(2).unwrap()));
            runtime.block_on(async {
                while spawner.running() > 0 {
                    ::tokio::task::yield_now().await;
                }
            });
            let mut got: Vec<i32> = rx.try_iter().collect();
            got.sort_unstable();
            assert_eq!(got, [1, 2]);
        }

        #[test]
        fn drop_and_shutdown_cancel_pending_tasks() {
            let runtime = Builder::new_current_thread().build().unwrap();
            for shutdown in [false, true] {
                let spawner = TokioSpawner::new(runtime.handle().clone());
                let dropped = Arc::new(AtomicBool::new(false));
                let guard = SetOnDrop(Arc::clone(&dropped));
                spawner.spawn_future(async move {
                    let _guard = guard;
                    std::future::pending::<()>().await;
                });
                assert_eq!(spawner.running(), 1);
                if shutdown {
                    runtime.block_on(spawner.shutdown());
                    assert_eq!(spawner.running(), 0);
                } else {
                    drop(spawner);
                    runtime.block_on(::tokio::task::yield_now());
                }
                assert!(dropped.load(Ordering::SeqCst), "the pending future is dropped");
            }
        }
    }
}

#[cfg(all(feature = "spawn-wasm", target_arch = "wasm32"))]
mod wasm {
    use std::{
        collections::HashMap,
        sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
        },
    };

    use futures_util::future::{AbortHandle, Abortable};
    use parking_lot::Mutex;

    use crate::bus::{BoxFuture, Spawner};

    /// [`Spawner`] over the browser's microtask queue (`wasm_bindgen_futures::spawn_local`).
    ///
    /// Owns the tasks it spawned: dropping it, or [`Self::shutdown`], aborts them; the browser drops an
    /// aborted future on its next poll.
    #[derive(Debug, Default)]
    pub struct WasmSpawner {
        running: Arc<Mutex<HashMap<u64, AbortHandle>>>,
        next: AtomicU64,
    }

    impl WasmSpawner {
        /// Spawner without tasks.
        pub fn new() -> Self {
            Self::default()
        }

        /// Tasks spawned and not yet finished.
        pub fn running(&self) -> usize {
            self.running.lock().len()
        }

        /// Aborts every task.
        pub fn shutdown(&self) {
            for (_, task) in self.running.lock().drain() {
                task.abort();
            }
        }
    }

    impl Spawner for WasmSpawner {
        fn spawn(&self, fut: BoxFuture) {
            let id = self.next.fetch_add(1, Ordering::Relaxed);
            let (task, registration) = AbortHandle::new_pair();
            self.running.lock().insert(id, task);
            let running = Arc::clone(&self.running);
            #[expect(
                clippy::disallowed_methods,
                reason = "docs/rules.md rule 11: the adapter forwards to the browser's executor and owns the task"
            )]
            wasm_bindgen_futures::spawn_local(async move {
                if Abortable::new(fut, registration).await.is_err() {
                    tracing::debug!(target: "egui_mineral", "spawned task aborted");
                }
                running.lock().remove(&id);
            });
        }
    }

    impl Drop for WasmSpawner {
        fn drop(&mut self) {
            self.shutdown();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod thread {
    use std::{
        pin::pin,
        sync::Arc,
        task::{Context, Wake, Waker},
        thread::{Builder, JoinHandle, Thread},
    };

    use futures_util::future::{AbortHandle, Abortable};
    use parking_lot::Mutex;

    use crate::bus::{BlockingJob, BlockingSpawner, BoxFuture, Spawner};

    /// [`Spawner`] and [`BlockingSpawner`] running each task on its own OS thread, for apps without an async
    /// runtime.
    ///
    /// Owns the tasks it spawned: dropping it aborts the futures, which end at their next wake;
    /// [`Self::shutdown`] also joins every thread. A blocking job that already started runs to completion. A
    /// task that panicked is reported to `tracing`.
    ///
    /// ```
    /// use std::sync::Arc;
    ///
    /// use egui_mineral::{bus::SpawnerExt, spawn::ThreadSpawner};
    ///
    /// // Shared by the bus handlers and `NativePlatform::new(Arc::clone(&spawner))`.
    /// let spawner = Arc::new(ThreadSpawner::new());
    /// spawner.spawn_future(async {});
    /// spawner.shutdown();
    /// ```
    #[derive(Debug, Default)]
    pub struct ThreadSpawner {
        tasks: Mutex<Vec<Task>>,
    }

    #[derive(Debug)]
    struct Task {
        thread: JoinHandle<()>,
        abort: Option<AbortHandle>,
    }

    impl ThreadSpawner {
        /// Spawner without tasks.
        pub fn new() -> Self {
            Self::default()
        }

        /// Tasks spawned and not yet finished.
        pub fn running(&self) -> usize {
            let mut tasks = self.tasks.lock();
            reap(&mut tasks);
            tasks.len()
        }

        /// Aborts every future and joins every thread, waiting for started blocking jobs; tasks spawned
        /// meanwhile are kept.
        pub fn shutdown(&self) {
            let tasks = std::mem::take(&mut *self.tasks.lock());
            for task in &tasks {
                task.abort();
            }
            for task in tasks {
                report(task.thread);
            }
        }

        fn start(&self, abort: Option<AbortHandle>, run: impl FnOnce() + Send + 'static) {
            #[expect(
                clippy::disallowed_methods,
                reason = "docs/rules.md rule 11: the adapter owns the thread and joins it on shutdown"
            )]
            let spawned = Builder::new().name("egui_mineral-task".into()).spawn(run);
            match spawned {
                Ok(thread) => {
                    let mut tasks = self.tasks.lock();
                    reap(&mut tasks);
                    tasks.push(Task { thread, abort });
                }
                Err(e) => tracing::error!(target: "egui_mineral", error = %e, "spawning a task thread failed"),
            }
        }
    }

    impl Task {
        fn abort(&self) {
            if let Some(abort) = &self.abort {
                abort.abort();
            }
        }
    }

    fn report(thread: JoinHandle<()>) {
        if thread.join().is_err() {
            tracing::error!(target: "egui_mineral", "spawned task panicked");
        }
    }

    fn reap(tasks: &mut Vec<Task>) {
        let (ended, running) = std::mem::take(tasks)
            .into_iter()
            .partition(|t: &Task| t.thread.is_finished());
        *tasks = running;
        for task in ended {
            report(task.thread);
        }
    }

    struct Unpark(Thread);

    impl Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }

    fn block_on(fut: BoxFuture) {
        let mut fut = pin!(fut);
        let waker = Waker::from(Arc::new(Unpark(std::thread::current())));
        let mut cx = Context::from_waker(&waker);
        while fut.as_mut().poll(&mut cx).is_pending() {
            std::thread::park();
        }
    }

    impl Spawner for ThreadSpawner {
        fn spawn(&self, fut: BoxFuture) {
            let (abort, registration) = AbortHandle::new_pair();
            self.start(Some(abort), move || {
                block_on(Box::pin(async move {
                    if Abortable::new(fut, registration).await.is_err() {
                        tracing::debug!(target: "egui_mineral", "spawned task aborted");
                    }
                }));
            });
        }
    }

    impl BlockingSpawner for ThreadSpawner {
        fn spawn_blocking(&self, job: BlockingJob) {
            self.start(None, job);
        }
    }

    impl Drop for ThreadSpawner {
        fn drop(&mut self) {
            for task in self.tasks.get_mut().iter() {
                task.abort();
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc,
        };

        use super::ThreadSpawner;
        use crate::bus::{BlockingSpawner as _, SpawnerExt as _};

        struct SetOnDrop(Arc<AtomicBool>);

        impl Drop for SetOnDrop {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }

        #[test]
        fn runs_futures_and_blocking_jobs_on_threads() {
            let spawner = ThreadSpawner::new();
            let (tx, rx) = mpsc::channel();
            let blocking = tx.clone();
            let (wake_tx, wake_rx) = futures_channel::oneshot::channel::<i32>();
            spawner.spawn_future(async move { tx.send(wake_rx.await.unwrap()).unwrap() });
            spawner.spawn_blocking(Box::new(move || blocking.send(2).unwrap()));
            wake_tx.send(1).unwrap();
            let mut got = [rx.recv().unwrap(), rx.recv().unwrap()];
            spawner.shutdown();
            got.sort_unstable();
            assert_eq!(got, [1, 2]);
            assert_eq!(spawner.running(), 0);
        }

        #[test]
        fn shutdown_cancels_pending_futures() {
            let spawner = ThreadSpawner::new();
            let dropped = Arc::new(AtomicBool::new(false));
            let guard = SetOnDrop(Arc::clone(&dropped));
            spawner.spawn_future(async move {
                let _guard = guard;
                std::future::pending::<()>().await;
            });
            assert_eq!(spawner.running(), 1);
            spawner.shutdown();
            assert!(dropped.load(Ordering::SeqCst), "the pending future is dropped");
        }
    }
}

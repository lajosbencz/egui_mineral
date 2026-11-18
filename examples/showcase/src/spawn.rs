//! The showcase's `Spawner`: one OS thread per task on native, `spawn_local` tasks on the web.

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::{AppSpawner, sleep};
#[cfg(target_arch = "wasm32")]
pub(crate) use web::{AppSpawner, sleep};

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::{
        sync::{Mutex, MutexGuard, PoisonError},
        thread::JoinHandle,
        time::{Duration, Instant},
    };

    use egui_mineral::bus::{BoxFuture, Spawner};

    const SHUTDOWN_GRACE: Duration = Duration::from_secs(2);

    pub(crate) fn sleep(d: Duration) -> impl Future<Output = ()> {
        futures_util::future::lazy(move |_| std::thread::sleep(d))
    }

    #[derive(Default)]
    pub(crate) struct AppSpawner(Mutex<Vec<JoinHandle<()>>>);

    impl AppSpawner {
        fn handles(&self) -> MutexGuard<'_, Vec<JoinHandle<()>>> {
            self.0.lock().unwrap_or_else(PoisonError::into_inner)
        }

        pub(crate) fn reap(&self) {
            let (done, running): (Vec<_>, Vec<_>) = self.handles().drain(..).partition(JoinHandle::is_finished);
            self.handles().extend(running);
            for h in done {
                if let Err(panic) = h.join() {
                    std::panic::resume_unwind(panic);
                }
            }
        }
    }

    impl Spawner for AppSpawner {
        fn spawn(&self, fut: BoxFuture) {
            let handle = std::thread::spawn(move || futures_executor::block_on(fut));
            self.handles().push(handle);
        }
    }

    impl Drop for AppSpawner {
        fn drop(&mut self) {
            let deadline = Instant::now() + SHUTDOWN_GRACE;
            let handles = std::mem::take(self.0.get_mut().unwrap_or_else(PoisonError::into_inner));
            for h in handles {
                while !h.is_finished() && Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(5));
                }
                if !h.is_finished() {
                    tracing::warn!("task did not finish within {SHUTDOWN_GRACE:?}; detaching");
                } else if let Err(panic) = h.join() {
                    if std::thread::panicking() {
                        tracing::error!(?panic, "task panicked during shutdown");
                    } else {
                        std::panic::resume_unwind(panic);
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use std::{
        collections::HashMap,
        sync::{
            Arc, Mutex, MutexGuard, PoisonError,
            atomic::{AtomicU64, Ordering},
        },
        time::Duration,
    };

    use egui_mineral::bus::{BoxFuture, Spawner};
    use futures_util::future::{AbortHandle, Abortable};

    pub(crate) fn sleep(d: Duration) -> impl Future<Output = ()> {
        gloo_timers::future::sleep(d)
    }

    type Running = Arc<Mutex<HashMap<u64, AbortHandle>>>;

    fn lock(running: &Running) -> MutexGuard<'_, HashMap<u64, AbortHandle>> {
        running.lock().unwrap_or_else(PoisonError::into_inner)
    }

    #[derive(Default)]
    pub(crate) struct AppSpawner {
        running: Running,
        next: AtomicU64,
    }

    impl AppSpawner {
        #[expect(
            clippy::unused_self,
            reason = "same API as the native spawner; a panic ends the wasm instance"
        )]
        pub(crate) fn reap(&self) {}
    }

    impl Spawner for AppSpawner {
        fn spawn(&self, fut: BoxFuture) {
            let id = self.next.fetch_add(1, Ordering::Relaxed);
            let (handle, registration) = AbortHandle::new_pair();
            lock(&self.running).insert(id, handle);
            let running = self.running.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if Abortable::new(fut, registration).await.is_err() {
                    tracing::debug!("task aborted on shutdown");
                }
                lock(&running).remove(&id);
            });
        }
    }

    impl Drop for AppSpawner {
        fn drop(&mut self) {
            for (_, h) in lock(&self.running).drain() {
                h.abort();
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

#[cfg(all(feature = "platform-clipboard", not(target_arch = "wasm32")))]
use parking_lot::Mutex;

#[cfg(feature = "platform-clipboard")]
use super::PlatformClipboard;
use super::PlatformError;
#[cfg(all(feature = "platform-pickers", not(target_arch = "wasm32")))]
use super::PlatformPathPickers;
#[cfg(any(
    feature = "platform-pickers",
    all(feature = "platform-clipboard", not(target_arch = "wasm32")),
    all(feature = "platform-notify", not(target_arch = "wasm32"))
))]
use super::backend;
#[cfg(feature = "platform-pickers")]
use super::{FileFilter, FileReader, FileWriter, PickedFile, PlatformPickers, SaveOutcome, SaveRequest};
#[cfg(feature = "platform-notify")]
use super::{Notification, PlatformNotifier};
#[cfg(not(target_arch = "wasm32"))]
use crate::bus::BlockingSpawner;
use crate::bus::BoxFuture;
#[cfg(all(feature = "platform-pickers", not(target_arch = "wasm32")))]
use std::path::Path;

/// Desktop and browser platform services; implements [`PlatformPickers`] (`platform-pickers`),
/// [`PlatformClipboard`] (`platform-clipboard`) and [`PlatformNotifier`] (`platform-notify`)
/// only with their feature.
///
/// Native clipboard, file read and writer-creation work runs on the app's `bus::BlockingSpawner`.
#[derive(Clone)]
#[cfg_attr(target_arch = "wasm32", derive(Default))]
pub struct NativePlatform {
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg_attr(
        not(any(feature = "platform-pickers", feature = "platform-clipboard")),
        expect(dead_code, reason = "only pickers and clipboard block")
    )]
    blocking: Arc<dyn BlockingSpawner>,
    #[cfg(all(feature = "platform-clipboard", not(target_arch = "wasm32")))]
    clipboard: Arc<Mutex<arboard::Clipboard>>,
}

crate::kit::opaque_debug!(NativePlatform);

// The future owns the result channel; the app's `BlockingSpawner` owns the job.
#[cfg(all(
    any(feature = "platform-pickers", feature = "platform-clipboard"),
    not(target_arch = "wasm32")
))]
async fn offload<T: Send + 'static>(
    blocking: &dyn BlockingSpawner,
    f: impl FnOnce() -> Result<T, PlatformError> + Send + 'static,
) -> Result<T, PlatformError> {
    let (tx, rx) = futures_channel::oneshot::channel();
    blocking.spawn_blocking(Box::new(move || {
        // The receiver is gone when the future was dropped; the result has no consumer.
        drop(tx.send(f()));
    }));
    rx.await.map_err(|_| backend("blocking job ended without a result"))?
}

impl NativePlatform {
    /// Platform service over the OS clipboard (`platform-clipboard`); blocking OS calls run on `blocking`.
    ///
    /// # Errors
    /// [`PlatformError::Backend`] when the OS clipboard is unavailable.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(blocking: impl BlockingSpawner + 'static) -> Result<Self, PlatformError> {
        Ok(Self {
            blocking: Arc::new(blocking),
            #[cfg(feature = "platform-clipboard")]
            clipboard: Arc::new(Mutex::new(arboard::Clipboard::new().map_err(backend)?)),
        })
    }

    /// Platform service over the browser APIs.
    #[cfg(target_arch = "wasm32")]
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(feature = "platform-pickers")]
mod pickers {
    #[cfg(not(target_arch = "wasm32"))]
    use super::backend;
    use super::{FileFilter, PickedFile, PlatformError};

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) async fn read(
        blocking: &dyn super::BlockingSpawner,
        h: rfd::FileHandle,
    ) -> Result<PickedFile, PlatformError> {
        let path = h.path().to_owned();
        let bytes = super::offload(blocking, {
            let path = path.clone();
            move || std::fs::read(path).map_err(backend)
        })
        .await?;
        Ok(PickedFile::new(h.file_name(), bytes).with_path(path))
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) async fn read(h: rfd::FileHandle) -> Result<PickedFile, PlatformError> {
        Ok(PickedFile::new(h.file_name(), h.read().await))
    }

    pub(super) fn save_dialog(request: &super::SaveRequest) -> rfd::AsyncFileDialog {
        dialog(&request.filter).set_file_name(&request.suggested_name)
    }

    pub(super) fn dialog(filter: &FileFilter) -> rfd::AsyncFileDialog {
        let mut d = rfd::AsyncFileDialog::new();
        if let Some(t) = &filter.title {
            d = d.set_title(t);
        }
        for t in &filter.filters {
            d = d.add_filter(&t.name, &t.extensions);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let (dir, name) = super::resolve_directory(filter, std::path::Path::is_dir, std::path::Path::is_file);
        #[cfg(target_arch = "wasm32")]
        let name = filter.file_name.clone();
        if let Some(n) = name {
            d = d.set_file_name(n);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(dir) = dir {
            d = d.set_directory(dir);
        }
        d
    }
}

#[cfg(all(feature = "platform-pickers", not(target_arch = "wasm32")))]
impl PlatformPathPickers for NativePlatform {
    fn pick_file_path(&self, filter: FileFilter) -> BoxFuture<Result<Option<std::path::PathBuf>, PlatformError>> {
        let fut = pickers::dialog(&filter).pick_file();
        Box::pin(async move { Ok(fut.await.map(|h| h.path().to_owned())) })
    }

    fn pick_file_paths(&self, filter: FileFilter) -> BoxFuture<Result<Vec<std::path::PathBuf>, PlatformError>> {
        let fut = pickers::dialog(&filter).pick_files();
        Box::pin(async move {
            Ok(fut
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|h| h.path().to_owned())
                .collect())
        })
    }
}

#[cfg(feature = "platform-pickers")]
impl PlatformPickers for NativePlatform {
    fn pick_file(&self, filter: FileFilter) -> BoxFuture<Result<Option<PickedFile>, PlatformError>> {
        let fut = pickers::dialog(&filter).pick_file();
        #[cfg(not(target_arch = "wasm32"))]
        let blocking = Arc::clone(&self.blocking);
        Box::pin(async move {
            match fut.await {
                #[cfg(not(target_arch = "wasm32"))]
                Some(h) => pickers::read(&*blocking, h).await.map(Some),
                #[cfg(target_arch = "wasm32")]
                Some(h) => pickers::read(h).await.map(Some),
                None => Ok(None),
            }
        })
    }

    fn pick_file_reader(&self, filter: FileFilter) -> BoxFuture<Result<Option<FileReader>, PlatformError>> {
        let fut = pickers::dialog(&filter).pick_file();
        Box::pin(async move {
            let Some(h) = fut.await else { return Ok(None) };
            #[cfg(not(target_arch = "wasm32"))]
            let reader = FileReader::from_path(h.path())?;
            #[cfg(target_arch = "wasm32")]
            let reader = FileReader::from_web(h.inner().clone());
            Ok(Some(reader))
        })
    }

    fn pick_files(&self, filter: FileFilter) -> BoxFuture<Result<Vec<PickedFile>, PlatformError>> {
        let fut = pickers::dialog(&filter).pick_files();
        #[cfg(not(target_arch = "wasm32"))]
        let blocking = Arc::clone(&self.blocking);
        Box::pin(async move {
            let mut out = Vec::new();
            for h in fut.await.unwrap_or_default() {
                #[cfg(not(target_arch = "wasm32"))]
                out.push(pickers::read(&*blocking, h).await?);
                #[cfg(target_arch = "wasm32")]
                out.push(pickers::read(h).await?);
            }
            Ok(out)
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn pick_folder(&self, filter: FileFilter) -> BoxFuture<Result<Option<std::path::PathBuf>, PlatformError>> {
        let fut = pickers::dialog(&filter.without_file_types()).pick_folder();
        Box::pin(async move { Ok(fut.await.map(|h| h.path().to_owned())) })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn pick_save_path(&self, request: SaveRequest) -> BoxFuture<Result<Option<std::path::PathBuf>, PlatformError>> {
        let fut = pickers::save_dialog(&request).save_file();
        Box::pin(async move { Ok(fut.await.map(|h| h.path().to_owned())) })
    }

    fn save_file(&self, request: SaveRequest, bytes: Vec<u8>) -> BoxFuture<Result<SaveOutcome, PlatformError>> {
        let fut = pickers::save_dialog(&request).save_file();
        Box::pin(async move {
            let Some(h) = fut.await else {
                return Ok(SaveOutcome::Cancelled);
            };
            h.write(&bytes).await.map_err(backend)?;
            Ok(SaveOutcome::Saved)
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_file_writer(&self, request: SaveRequest) -> BoxFuture<Result<Option<FileWriter>, PlatformError>> {
        let fut = pickers::save_dialog(&request).save_file();
        let blocking = Arc::clone(&self.blocking);
        Box::pin(async move {
            match fut.await {
                Some(h) => {
                    let path = h.path().to_owned();
                    offload(&*blocking, move || FileWriter::create(path)).await.map(Some)
                }
                None => Ok(None),
            }
        })
    }

    #[cfg(target_arch = "wasm32")]
    fn save_file_writer(&self, request: SaveRequest) -> BoxFuture<Result<Option<FileWriter>, PlatformError>> {
        let this = self.clone();
        Box::pin(async move {
            #[cfg(feature = "web-save-stream")]
            if let Some(picked) = super::writer::web::open(&request.suggested_name, &request.filter).await? {
                return Ok(picked.map(|stream| FileWriter::web(request.suggested_name, stream)));
            }
            let SaveRequest { suggested_name, filter } = request;
            Ok(Some(FileWriter::buffered(suggested_name, move |name, bytes| {
                Box::pin(async move {
                    this.save_file(SaveRequest::new(name).filter(filter), bytes)
                        .await
                        .map(drop)
                })
            })))
        })
    }
}

#[cfg(feature = "platform-clipboard")]
impl PlatformClipboard for NativePlatform {
    #[cfg(not(target_arch = "wasm32"))]
    fn clipboard_text(&self) -> BoxFuture<Result<String, PlatformError>> {
        let (blocking, clipboard) = (Arc::clone(&self.blocking), Arc::clone(&self.clipboard));
        Box::pin(async move { offload(&*blocking, move || clipboard.lock().get_text().map_err(backend)).await })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn set_clipboard_text(&self, text: String) -> BoxFuture<Result<(), PlatformError>> {
        let (blocking, clipboard) = (Arc::clone(&self.blocking), Arc::clone(&self.clipboard));
        Box::pin(async move { offload(&*blocking, move || clipboard.lock().set_text(text).map_err(backend)).await })
    }

    #[cfg(target_arch = "wasm32")]
    fn clipboard_text(&self) -> BoxFuture<Result<String, PlatformError>> {
        Box::pin(async {
            let clip = web::clipboard()?;
            let v = wasm_bindgen_futures::JsFuture::from(clip.read_text())
                .await
                .map_err(web::js)?;
            v.as_string()
                .ok_or_else(|| PlatformError::Backend("clipboard did not return text".into()))
        })
    }

    #[cfg(target_arch = "wasm32")]
    fn set_clipboard_text(&self, text: String) -> BoxFuture<Result<(), PlatformError>> {
        Box::pin(async move {
            let clip = web::clipboard()?;
            wasm_bindgen_futures::JsFuture::from(clip.write_text(&text))
                .await
                .map_err(web::js)?;
            Ok(())
        })
    }
}

#[cfg(feature = "platform-notify")]
impl PlatformNotifier for NativePlatform {
    // XDG: D-Bus round trip, awaited on zbus' own runtime.
    #[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
    fn notify(&self, notification: Notification) -> BoxFuture<Result<(), PlatformError>> {
        Box::pin(async move {
            notify_rust::Notification::new()
                .summary(&notification.title)
                .body(&notification.body)
                .show_async()
                .await
                .map(|_| ())
                .map_err(backend)
        })
    }

    #[cfg(any(windows, target_os = "macos"))]
    fn notify(&self, notification: Notification) -> BoxFuture<Result<(), PlatformError>> {
        Box::pin(async move {
            notify_rust::Notification::new()
                .summary(&notification.title)
                .body(&notification.body)
                .show()
                .map(|_| ())
                .map_err(backend)
        })
    }

    #[cfg(target_arch = "wasm32")]
    fn notify(&self, notification: Notification) -> BoxFuture<Result<(), PlatformError>> {
        use web_sys::{NotificationOptions, NotificationPermission};
        Box::pin(async move {
            if web_sys::Notification::permission() == NotificationPermission::Default {
                let p = web_sys::Notification::request_permission().map_err(web::js)?;
                wasm_bindgen_futures::JsFuture::from(p).await.map_err(web::js)?;
            }
            if web_sys::Notification::permission() != NotificationPermission::Granted {
                return Err(PlatformError::PermissionDenied);
            }
            let opts = NotificationOptions::new();
            opts.set_body(&notification.body);
            web_sys::Notification::new_with_options(&notification.title, &opts).map_err(web::js)?;
            Ok(())
        })
    }
}

#[cfg(all(
    target_arch = "wasm32",
    any(feature = "platform-clipboard", feature = "platform-notify")
))]
mod web {
    use super::PlatformError;

    pub(super) fn js(e: wasm_bindgen::JsValue) -> PlatformError {
        PlatformError::Backend(Box::new(crate::js::JsError::from(e)))
    }

    #[cfg(feature = "platform-clipboard")]
    pub(super) fn clipboard() -> Result<web_sys::Clipboard, PlatformError> {
        let window = web_sys::window().ok_or_else(|| PlatformError::Backend("no window".into()))?;
        Ok(window.navigator().clipboard())
    }
}

// `FileFilter::start`: a folder as is, a file as its folder plus name, a missing path as its parent
#[cfg(all(feature = "platform-pickers", not(target_arch = "wasm32")))]
fn resolve_directory(
    filter: &FileFilter,
    is_dir: impl Fn(&Path) -> bool,
    is_file: impl Fn(&Path) -> bool,
) -> (Option<std::path::PathBuf>, Option<String>) {
    let name = filter.file_name.clone();
    let Some(path) = &filter.start else {
        return (None, name);
    };
    if is_dir(path) {
        return (Some(path.clone()), name);
    }
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).map(Path::to_owned);
    match path.file_name() {
        Some(file) if is_file(path) => (parent, Some(file.to_string_lossy().into_owned())),
        _ => (parent, name),
    }
}

#[cfg(all(test, feature = "platform-pickers", not(target_arch = "wasm32")))]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn filter() -> FileFilter {
        FileFilter::default().file_name("n.txt")
    }

    fn resolve(f: &FileFilter, dir: bool, file: bool) -> (Option<PathBuf>, Option<String>) {
        resolve_directory(f, |_| dir, |_| file)
    }

    #[test]
    fn no_directory_keeps_the_name() {
        assert_eq!(resolve(&filter(), false, false), (None, Some("n.txt".into())));
    }

    #[test]
    fn existing_folder_is_kept() {
        let (dir, name) = resolve(&filter().start("/d"), true, false);
        assert_eq!(dir, Some(PathBuf::from("/d")));
        assert_eq!(name.as_deref(), Some("n.txt"));
    }

    #[test]
    fn existing_file_opens_its_folder_and_proposes_it() {
        let (dir, name) = resolve(&filter().start("/a/b.png"), false, true);
        assert_eq!(dir, Some(PathBuf::from("/a")));
        assert_eq!(name.as_deref(), Some("b.png"));
    }

    #[test]
    fn missing_path_opens_its_parent_and_keeps_the_name() {
        let (dir, name) = resolve(&filter().start("/a/b.png"), false, false);
        assert_eq!(dir, Some(PathBuf::from("/a")));
        assert_eq!(name.as_deref(), Some("n.txt"));
    }

    #[test]
    fn real_files_and_folders_are_detected() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let f = FileFilter::default().start(manifest.join("Cargo.toml"));
        let (dir, name) = resolve_directory(&f, Path::is_dir, Path::is_file);
        assert_eq!(dir, Some(manifest.clone()));
        assert_eq!(name.as_deref(), Some("Cargo.toml"));
        let f = FileFilter::default().start(&manifest);
        assert_eq!(resolve_directory(&f, Path::is_dir, Path::is_file).0, Some(manifest));
    }
}

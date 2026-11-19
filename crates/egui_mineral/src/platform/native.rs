#[cfg(all(feature = "platform-clipboard", not(target_arch = "wasm32")))]
use std::sync::Arc;

#[cfg(all(feature = "platform-clipboard", not(target_arch = "wasm32")))]
use parking_lot::Mutex;

#[cfg(any(
    feature = "platform-pickers",
    all(feature = "platform-clipboard", not(target_arch = "wasm32")),
    all(feature = "platform-notify", not(target_arch = "wasm32"))
))]
use super::backend;
use super::{
    FileFilter, FileReader, FileWriter, Notification, PickedFile, PlatformError, PlatformService, SaveOutcome,
};
use crate::bus::BoxFuture;

// The service call of a disabled feature.
#[cfg(not(all(
    feature = "platform-pickers",
    feature = "platform-clipboard",
    feature = "platform-notify"
)))]
macro_rules! unsupported {
    ($feature:expr $(, $arg:ident)*) => {{
        $(drop($arg);)*
        Box::pin(async { Err(PlatformError::Unsupported($feature)) })
    }};
}

/// Desktop and browser [`PlatformService`]: file dialogs (`platform-pickers`), clipboard
/// (`platform-clipboard`) and notifications (`platform-notify`). A call of a disabled feature fails
/// with [`PlatformError::Unsupported`].
///
/// Clipboard futures block their thread on the OS clipboard while polled; spawn them where that is fine.
#[derive(Clone)]
pub struct NativePlatform {
    #[cfg(all(feature = "platform-clipboard", not(target_arch = "wasm32")))]
    clipboard: Arc<Mutex<arboard::Clipboard>>,
}

crate::kit::opaque_debug!(NativePlatform);

impl NativePlatform {
    /// Platform service over the OS clipboard (`platform-clipboard`).
    ///
    /// # Errors
    /// [`PlatformError::Backend`] when the OS clipboard is unavailable.
    pub fn new() -> Result<Self, PlatformError> {
        Ok(Self {
            #[cfg(all(feature = "platform-clipboard", not(target_arch = "wasm32")))]
            clipboard: Arc::new(Mutex::new(arboard::Clipboard::new().map_err(backend)?)),
        })
    }
}

#[cfg(feature = "platform-pickers")]
mod pickers {
    #[cfg(not(target_arch = "wasm32"))]
    use super::backend;
    use super::{FileFilter, PickedFile, PlatformError};

    #[expect(clippy::unused_async, reason = "same signature as the wasm variant")]
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) async fn read(h: rfd::FileHandle) -> Result<PickedFile, PlatformError> {
        let bytes = std::fs::read(h.path()).map_err(backend)?;
        Ok(PickedFile {
            name: h.file_name(),
            bytes,
            path: Some(h.path().to_owned()),
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) async fn read(h: rfd::FileHandle) -> Result<PickedFile, PlatformError> {
        Ok(PickedFile {
            name: h.file_name(),
            bytes: h.read().await,
        })
    }

    pub(super) fn dialog(filter: &FileFilter) -> rfd::AsyncFileDialog {
        let mut d = rfd::AsyncFileDialog::new();
        if let Some(t) = &filter.title {
            d = d.set_title(t);
        }
        for t in &filter.filters {
            d = d.add_filter(&t.name, &t.extensions);
        }
        if let Some(n) = &filter.file_name {
            d = d.set_file_name(n);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(dir) = &filter.directory {
            d = d.set_directory(dir);
        }
        d
    }
}

impl PlatformService for NativePlatform {
    fn pick_file(&self, filter: FileFilter) -> BoxFuture<Result<Option<PickedFile>, PlatformError>> {
        #[cfg(not(feature = "platform-pickers"))]
        {
            unsupported!("platform-pickers", filter)
        }
        #[cfg(feature = "platform-pickers")]
        {
            let fut = pickers::dialog(&filter).pick_file();
            Box::pin(async move {
                match fut.await {
                    Some(h) => pickers::read(h).await.map(Some),
                    None => Ok(None),
                }
            })
        }
    }

    fn pick_file_reader(&self, filter: FileFilter) -> BoxFuture<Result<Option<FileReader>, PlatformError>> {
        #[cfg(not(feature = "platform-pickers"))]
        {
            unsupported!("platform-pickers", filter)
        }
        #[cfg(feature = "platform-pickers")]
        {
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
    }

    fn pick_files(&self, filter: FileFilter) -> BoxFuture<Result<Vec<PickedFile>, PlatformError>> {
        #[cfg(not(feature = "platform-pickers"))]
        {
            unsupported!("platform-pickers", filter)
        }
        #[cfg(feature = "platform-pickers")]
        {
            let fut = pickers::dialog(&filter).pick_files();
            Box::pin(async move {
                let mut out = Vec::new();
                for h in fut.await.unwrap_or_default() {
                    out.push(pickers::read(h).await?);
                }
                Ok(out)
            })
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn pick_folder(&self, filter: FileFilter) -> BoxFuture<Result<Option<std::path::PathBuf>, PlatformError>> {
        #[cfg(not(feature = "platform-pickers"))]
        {
            unsupported!("platform-pickers", filter)
        }
        #[cfg(feature = "platform-pickers")]
        {
            let fut = pickers::dialog(&FileFilter {
                filters: Vec::new(),
                ..filter
            })
            .pick_folder();
            Box::pin(async move { Ok(fut.await.map(|h| h.path().to_owned())) })
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn pick_save_path(
        &self,
        suggested_name: String,
        filter: FileFilter,
    ) -> BoxFuture<Result<Option<std::path::PathBuf>, PlatformError>> {
        #[cfg(not(feature = "platform-pickers"))]
        {
            unsupported!("platform-pickers", suggested_name, filter)
        }
        #[cfg(feature = "platform-pickers")]
        {
            let fut = pickers::dialog(&filter).set_file_name(suggested_name).save_file();
            Box::pin(async move { Ok(fut.await.map(|h| h.path().to_owned())) })
        }
    }

    fn save_file(
        &self,
        suggested_name: String,
        filter: FileFilter,
        bytes: Vec<u8>,
    ) -> BoxFuture<Result<SaveOutcome, PlatformError>> {
        #[cfg(not(feature = "platform-pickers"))]
        {
            unsupported!("platform-pickers", suggested_name, filter, bytes)
        }
        #[cfg(feature = "platform-pickers")]
        {
            let fut = pickers::dialog(&filter).set_file_name(suggested_name).save_file();
            Box::pin(async move {
                let Some(h) = fut.await else {
                    return Ok(SaveOutcome::Cancelled);
                };
                h.write(&bytes).await.map_err(backend)?;
                Ok(SaveOutcome::Saved)
            })
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_file_writer(
        &self,
        suggested_name: String,
        filter: FileFilter,
    ) -> BoxFuture<Result<Option<FileWriter>, PlatformError>> {
        #[cfg(not(feature = "platform-pickers"))]
        {
            unsupported!("platform-pickers", suggested_name, filter)
        }
        #[cfg(feature = "platform-pickers")]
        {
            let fut = pickers::dialog(&filter).set_file_name(suggested_name).save_file();
            Box::pin(async move {
                match fut.await {
                    Some(h) => FileWriter::create(h.path()).map(Some),
                    None => Ok(None),
                }
            })
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn save_file_writer(
        &self,
        suggested_name: String,
        filter: FileFilter,
    ) -> BoxFuture<Result<Option<FileWriter>, PlatformError>> {
        #[cfg(not(feature = "platform-pickers"))]
        {
            unsupported!("platform-pickers", suggested_name, filter)
        }
        #[cfg(feature = "platform-pickers")]
        {
            let this = self.clone();
            Box::pin(async move {
                #[cfg(feature = "web-save-stream")]
                if let Some(picked) = super::writer::web::open(&suggested_name, &filter).await? {
                    return Ok(picked.map(|stream| FileWriter::web(suggested_name, stream)));
                }
                Ok(Some(FileWriter::buffered(suggested_name, move |name, bytes| {
                    Box::pin(async move { this.save_file(name, filter, bytes).await.map(drop) })
                })))
            })
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn clipboard_text(&self) -> BoxFuture<Result<String, PlatformError>> {
        #[cfg(not(feature = "platform-clipboard"))]
        {
            unsupported!("platform-clipboard")
        }
        #[cfg(feature = "platform-clipboard")]
        {
            let clipboard = Arc::clone(&self.clipboard);
            Box::pin(async move { clipboard.lock().get_text().map_err(backend) })
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn set_clipboard_text(&self, text: String) -> BoxFuture<Result<(), PlatformError>> {
        #[cfg(not(feature = "platform-clipboard"))]
        {
            unsupported!("platform-clipboard", text)
        }
        #[cfg(feature = "platform-clipboard")]
        {
            let clipboard = Arc::clone(&self.clipboard);
            Box::pin(async move { clipboard.lock().set_text(text).map_err(backend) })
        }
    }

    // XDG: D-Bus round trip, awaited on zbus' own runtime.
    #[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
    fn notify(&self, notification: Notification) -> BoxFuture<Result<(), PlatformError>> {
        #[cfg(not(feature = "platform-notify"))]
        {
            unsupported!("platform-notify", notification)
        }
        #[cfg(feature = "platform-notify")]
        {
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
    }

    #[cfg(any(windows, target_os = "macos"))]
    fn notify(&self, notification: Notification) -> BoxFuture<Result<(), PlatformError>> {
        #[cfg(not(feature = "platform-notify"))]
        {
            unsupported!("platform-notify", notification)
        }
        #[cfg(feature = "platform-notify")]
        {
            Box::pin(async move {
                notify_rust::Notification::new()
                    .summary(&notification.title)
                    .body(&notification.body)
                    .show()
                    .map(|_| ())
                    .map_err(backend)
            })
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn clipboard_text(&self) -> BoxFuture<Result<String, PlatformError>> {
        #[cfg(not(feature = "platform-clipboard"))]
        {
            unsupported!("platform-clipboard")
        }
        #[cfg(feature = "platform-clipboard")]
        {
            Box::pin(async {
                let clip = web::clipboard()?;
                let v = wasm_bindgen_futures::JsFuture::from(clip.read_text())
                    .await
                    .map_err(web::js)?;
                v.as_string()
                    .ok_or_else(|| PlatformError::Backend("clipboard did not return text".into()))
            })
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn set_clipboard_text(&self, text: String) -> BoxFuture<Result<(), PlatformError>> {
        #[cfg(not(feature = "platform-clipboard"))]
        {
            unsupported!("platform-clipboard", text)
        }
        #[cfg(feature = "platform-clipboard")]
        {
            Box::pin(async move {
                let clip = web::clipboard()?;
                wasm_bindgen_futures::JsFuture::from(clip.write_text(&text))
                    .await
                    .map_err(web::js)?;
                Ok(())
            })
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn notify(&self, notification: Notification) -> BoxFuture<Result<(), PlatformError>> {
        #[cfg(not(feature = "platform-notify"))]
        {
            unsupported!("platform-notify", notification)
        }
        #[cfg(feature = "platform-notify")]
        {
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use futures_util::FutureExt;

    use super::*;

    // Only built where a feature is off, so the constructor never reaches the OS clipboard.
    #[cfg(not(feature = "platform-clipboard"))]
    #[test]
    fn disabled_clipboard_is_unsupported() {
        let p = NativePlatform::new().unwrap();
        let err = p.clipboard_text().now_or_never().unwrap().unwrap_err();
        assert!(matches!(err, PlatformError::Unsupported("platform-clipboard")), "{err}");
        let err = p.set_clipboard_text("x".into()).now_or_never().unwrap().unwrap_err();
        assert!(matches!(err, PlatformError::Unsupported("platform-clipboard")), "{err}");
    }

    #[cfg(all(not(feature = "platform-clipboard"), not(feature = "platform-pickers")))]
    #[test]
    fn disabled_pickers_are_unsupported() {
        let p = NativePlatform::new().unwrap();
        let err = p.pick_file(FileFilter::default()).now_or_never().unwrap().unwrap_err();
        assert!(matches!(err, PlatformError::Unsupported("platform-pickers")), "{err}");
        let err = p
            .save_file("a".into(), FileFilter::default(), vec![])
            .now_or_never()
            .unwrap()
            .unwrap_err();
        assert!(matches!(err, PlatformError::Unsupported("platform-pickers")), "{err}");
    }

    #[cfg(all(not(feature = "platform-clipboard"), not(feature = "platform-notify")))]
    #[test]
    fn disabled_notifications_are_unsupported() {
        let n = Notification {
            title: "t".into(),
            body: "b".into(),
        };
        let err = NativePlatform::new()
            .unwrap()
            .notify(n)
            .now_or_never()
            .unwrap()
            .unwrap_err();
        assert!(matches!(err, PlatformError::Unsupported("platform-notify")), "{err}");
    }
}

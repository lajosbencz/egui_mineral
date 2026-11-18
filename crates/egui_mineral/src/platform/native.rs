#[cfg(not(target_arch = "wasm32"))]
use std::{path::PathBuf, sync::Arc};

#[cfg(not(target_arch = "wasm32"))]
use parking_lot::Mutex;

use super::{
    FileFilter, FileReader, FileWriter, Notification, PickedFile, PlatformError, PlatformService, SaveOutcome, backend,
};
use crate::bus::BoxFuture;

/// Desktop [`PlatformService`]: clipboard, file dialogs, notifications.
///
/// Clipboard futures block their thread on the OS clipboard while polled; spawn them where that is fine.
#[derive(Clone)]
pub struct NativePlatform {
    #[cfg(not(target_arch = "wasm32"))]
    clipboard: Arc<Mutex<arboard::Clipboard>>,
}

crate::kit::opaque_debug!(NativePlatform);

impl NativePlatform {
    /// Platform service over the OS clipboard.
    ///
    /// # Errors
    /// [`PlatformError::Backend`] when the OS clipboard is unavailable.
    pub fn new() -> Result<Self, PlatformError> {
        Ok(Self {
            #[cfg(not(target_arch = "wasm32"))]
            clipboard: Arc::new(Mutex::new(arboard::Clipboard::new().map_err(backend)?)),
        })
    }
}

#[expect(clippy::unused_async, reason = "same signature as the wasm variant")]
#[cfg(not(target_arch = "wasm32"))]
async fn read(h: rfd::FileHandle) -> Result<PickedFile, PlatformError> {
    let bytes = std::fs::read(h.path()).map_err(backend)?;
    Ok(PickedFile {
        name: h.file_name(),
        bytes,
        path: Some(h.path().to_owned()),
    })
}

#[cfg(target_arch = "wasm32")]
async fn read(h: rfd::FileHandle) -> Result<PickedFile, PlatformError> {
    Ok(PickedFile {
        name: h.file_name(),
        bytes: h.read().await,
    })
}

fn dialog(filter: &FileFilter) -> rfd::AsyncFileDialog {
    let mut d = rfd::AsyncFileDialog::new();
    if let Some(t) = &filter.title {
        d = d.set_title(t);
    }
    for t in &filter.filters {
        d = d.add_filter(&t.name, &t.extensions);
    }
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(dir) = &filter.directory {
        d = d.set_directory(dir);
    }
    d
}

impl PlatformService for NativePlatform {
    fn pick_file(&self, filter: FileFilter) -> BoxFuture<Result<Option<PickedFile>, PlatformError>> {
        let fut = dialog(&filter).pick_file();
        Box::pin(async move {
            match fut.await {
                Some(h) => read(h).await.map(Some),
                None => Ok(None),
            }
        })
    }

    fn pick_file_reader(&self, filter: FileFilter) -> BoxFuture<Result<Option<FileReader>, PlatformError>> {
        let fut = dialog(&filter).pick_file();
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
        let fut = dialog(&filter).pick_files();
        Box::pin(async move {
            let mut out = Vec::new();
            for h in fut.await.unwrap_or_default() {
                out.push(read(h).await?);
            }
            Ok(out)
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn pick_folder(&self, filter: FileFilter) -> BoxFuture<Result<Option<PathBuf>, PlatformError>> {
        let fut = dialog(&FileFilter {
            filters: Vec::new(),
            ..filter
        })
        .pick_folder();
        Box::pin(async move { Ok(fut.await.map(|h| h.path().to_owned())) })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn pick_save_path(
        &self,
        suggested_name: String,
        filter: FileFilter,
    ) -> BoxFuture<Result<Option<PathBuf>, PlatformError>> {
        let fut = dialog(&filter).set_file_name(suggested_name).save_file();
        Box::pin(async move { Ok(fut.await.map(|h| h.path().to_owned())) })
    }

    fn save_file(&self, suggested_name: String, bytes: Vec<u8>) -> BoxFuture<Result<SaveOutcome, PlatformError>> {
        let fut = rfd::AsyncFileDialog::new().set_file_name(suggested_name).save_file();
        Box::pin(async move {
            let Some(h) = fut.await else {
                return Ok(SaveOutcome::Cancelled);
            };
            h.write(&bytes).await.map_err(backend)?;
            Ok(SaveOutcome::Saved)
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_file_writer(
        &self,
        suggested_name: String,
        filter: FileFilter,
    ) -> BoxFuture<Result<Option<FileWriter>, PlatformError>> {
        let fut = dialog(&filter).set_file_name(suggested_name).save_file();
        Box::pin(async move {
            match fut.await {
                Some(h) => FileWriter::create(h.path()).map(Some),
                None => Ok(None),
            }
        })
    }

    #[cfg(target_arch = "wasm32")]
    fn save_file_writer(
        &self,
        suggested_name: String,
        filter: FileFilter,
    ) -> BoxFuture<Result<Option<FileWriter>, PlatformError>> {
        let this = self.clone();
        Box::pin(async move {
            #[cfg(feature = "web-save-stream")]
            if let Some(picked) = super::writer::web::open(&suggested_name, &filter).await? {
                return Ok(picked.map(|stream| FileWriter::web(suggested_name, stream)));
            }
            #[cfg(not(feature = "web-save-stream"))]
            drop(filter);
            Ok(Some(FileWriter::buffered(suggested_name, move |name, bytes| {
                Box::pin(async move { this.save_file(name, bytes).await.map(drop) })
            })))
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn clipboard_text(&self) -> BoxFuture<Result<String, PlatformError>> {
        let clipboard = Arc::clone(&self.clipboard);
        Box::pin(async move { clipboard.lock().get_text().map_err(backend) })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn set_clipboard_text(&self, text: String) -> BoxFuture<Result<(), PlatformError>> {
        let clipboard = Arc::clone(&self.clipboard);
        Box::pin(async move { clipboard.lock().set_text(text).map_err(backend) })
    }

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

#[cfg(target_arch = "wasm32")]
mod web {
    use super::PlatformError;

    pub(super) fn js(e: wasm_bindgen::JsValue) -> PlatformError {
        PlatformError::Backend(Box::new(crate::js::JsError::from(e)))
    }

    pub(super) fn clipboard() -> Result<web_sys::Clipboard, PlatformError> {
        let window = web_sys::window().ok_or_else(|| PlatformError::Backend("no window".into()))?;
        Ok(window.navigator().clipboard())
    }
}

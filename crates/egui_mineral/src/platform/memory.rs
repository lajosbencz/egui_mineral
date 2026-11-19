use std::{collections::VecDeque, sync::Arc};

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use parking_lot::{Mutex, MutexGuard};

use super::{
    FileFilter, FileReader, FileWriter, Notification, PickedFile, PlatformError, PlatformService, SaveOutcome,
};
use crate::bus::BoxFuture;

#[derive(Default)]
struct Inner {
    clipboard: String,
    picks: VecDeque<Vec<PickedFile>>,
    #[cfg(not(target_arch = "wasm32"))]
    paths: VecDeque<PathBuf>,
    saved: Vec<PickedFile>,
    notifications: Vec<Notification>,
}

/// In-memory [`PlatformService`] for tests; clones share state.
#[derive(Clone, Default)]
pub struct MemoryPlatform(Arc<Mutex<Inner>>);

crate::kit::opaque_debug!(MemoryPlatform);

impl MemoryPlatform {
    fn inner(&self) -> MutexGuard<'_, Inner> {
        self.0.lock()
    }

    /// Next result of `pick_file`/`pick_files`/`pick_file_reader`; an empty queue answers as cancelled.
    pub fn queue_pick(&self, files: Vec<PickedFile>) {
        self.inner().picks.push_back(files);
    }

    /// Next result of [`PlatformService::pick_folder`] or [`PlatformService::pick_save_path`];
    /// an empty queue answers as cancelled.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn queue_path(&self, path: PathBuf) {
        self.inner().paths.push_back(path);
    }

    /// Files written through `save_file` or a closed `save_file_writer`, oldest first.
    pub fn saved(&self) -> Vec<PickedFile> {
        self.inner().saved.clone()
    }

    /// Each `notify` call, oldest first.
    pub fn notifications(&self) -> Vec<Notification> {
        self.inner().notifications.clone()
    }
}

impl PlatformService for MemoryPlatform {
    fn pick_file(&self, _filter: FileFilter) -> BoxFuture<Result<Option<PickedFile>, PlatformError>> {
        let next = self.inner().picks.pop_front().and_then(|v| v.into_iter().next());
        Box::pin(async move { Ok(next) })
    }

    fn pick_file_reader(&self, _filter: FileFilter) -> BoxFuture<Result<Option<FileReader>, PlatformError>> {
        let next = self.inner().picks.pop_front().and_then(|v| v.into_iter().next());
        Box::pin(async move { Ok(next.map(|f| FileReader::from_bytes(f.name, f.bytes))) })
    }

    fn pick_files(&self, _filter: FileFilter) -> BoxFuture<Result<Vec<PickedFile>, PlatformError>> {
        let next = self.inner().picks.pop_front().unwrap_or_default();
        Box::pin(async move { Ok(next) })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn pick_folder(&self, _filter: FileFilter) -> BoxFuture<Result<Option<PathBuf>, PlatformError>> {
        let next = self.inner().paths.pop_front();
        Box::pin(async move { Ok(next) })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn pick_save_path(
        &self,
        _suggested_name: String,
        _filter: FileFilter,
    ) -> BoxFuture<Result<Option<PathBuf>, PlatformError>> {
        let next = self.inner().paths.pop_front();
        Box::pin(async move { Ok(next) })
    }

    fn save_file(
        &self,
        suggested_name: String,
        _filter: FileFilter,
        bytes: Vec<u8>,
    ) -> BoxFuture<Result<SaveOutcome, PlatformError>> {
        self.inner().saved.push(PickedFile {
            name: suggested_name,
            bytes,
            #[cfg(not(target_arch = "wasm32"))]
            path: None,
        });
        Box::pin(async { Ok(SaveOutcome::Saved) })
    }

    fn save_file_writer(
        &self,
        suggested_name: String,
        _filter: FileFilter,
    ) -> BoxFuture<Result<Option<FileWriter>, PlatformError>> {
        let this = self.clone();
        let w = FileWriter::buffered(suggested_name, move |name, bytes| {
            Box::pin(async move { this.save_file(name, FileFilter::default(), bytes).await.map(drop) })
        });
        Box::pin(async move { Ok(Some(w)) })
    }

    fn clipboard_text(&self) -> BoxFuture<Result<String, PlatformError>> {
        let text = self.inner().clipboard.clone();
        Box::pin(async move { Ok(text) })
    }

    fn set_clipboard_text(&self, text: String) -> BoxFuture<Result<(), PlatformError>> {
        self.inner().clipboard = text;
        Box::pin(async { Ok(()) })
    }

    fn notify(&self, notification: Notification) -> BoxFuture<Result<(), PlatformError>> {
        self.inner().notifications.push(notification);
        Box::pin(async { Ok(()) })
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use futures_util::FutureExt;

    use crate::platform::FileType;

    use super::*;

    #[test]
    fn save_and_notify_report_typed_results() {
        let p = MemoryPlatform::default();
        let outcome = p
            .save_file("a.txt".into(), FileFilter::default(), vec![1])
            .now_or_never()
            .unwrap()
            .unwrap();
        assert_eq!(outcome, SaveOutcome::Saved);
        let note = Notification {
            title: "t".into(),
            body: "b".into(),
        };
        p.notify(note.clone()).now_or_never().unwrap().unwrap();
        assert_eq!(p.notifications(), [note]);
        assert_eq!(p.saved().len(), 1);
    }

    #[test]
    fn backend_error_exposes_its_cause() {
        let io = std::io::Error::other("disk gone");
        let e = PlatformError::Backend(Box::new(io));
        assert_eq!(e.source().map(ToString::to_string).as_deref(), Some("disk gone"));
        assert_eq!(e.to_string(), "platform backend failed: disk gone");
        assert!(PlatformError::PermissionDenied.source().is_none());
    }

    #[test]
    fn file_filters_are_named_types() {
        let f = FileFilter::new(FileType::new("Images", ["png", "jpg"])).file_type(FileType::new("Text", ["txt"]));
        assert_eq!(f.filters[0].name, "Images");
        assert_eq!(f.filters[0].extensions, ["png", "jpg"]);
        assert_eq!(f.filters[1].extensions, ["txt"]);
        assert_eq!(
            f.clone().file_name("report.txt").file_name.as_deref(),
            Some("report.txt")
        );
        assert_eq!(f.file_name, None);
    }
}

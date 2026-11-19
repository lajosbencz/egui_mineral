//! Every call is async ([`BoxFuture`]) because browser clipboard and notification APIs are.
//! Blocking OS work runs when the future is polled, never in the call itself.

mod memory;
#[cfg(any(
    feature = "platform-pickers",
    feature = "platform-clipboard",
    feature = "platform-notify"
))]
mod native;
mod reader;
mod writer;

pub use memory::MemoryPlatform;
#[cfg(any(
    feature = "platform-pickers",
    feature = "platform-clipboard",
    feature = "platform-notify"
))]
pub use native::NativePlatform;
pub use reader::FileReader;
pub use writer::FileWriter;

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use crate::bus::BoxFuture;

/// Failure of a platform service call.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PlatformError {
    /// User or OS refused access (clipboard read, notifications).
    #[error("permission denied")]
    PermissionDenied,
    /// The cargo feature (named) behind the call is disabled.
    #[error("platform feature `{0}` is not enabled")]
    Unsupported(&'static str),
    /// Backend failure; the cause is the [`source`](std::error::Error::source).
    #[error("platform backend failed: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

fn backend(e: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> PlatformError {
    PlatformError::Backend(e.into())
}

/// Outcome of [`PlatformService::save_file`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SaveOutcome {
    /// Written to the chosen target.
    Saved,
    /// Dialog dismissed; nothing written.
    Cancelled,
}

/// OS notification content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notification {
    /// Summary line.
    pub title: String,
    /// Message text.
    pub body: String,
}

/// File chosen through the platform file picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickedFile {
    /// File name without directories.
    pub name: String,
    /// Full file contents, read when the future resolves.
    pub bytes: Vec<u8>,
    /// Native only; `None` for in-memory files.
    #[cfg(not(target_arch = "wasm32"))]
    pub path: Option<PathBuf>,
}

/// File picker title, extension filters and start folder.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[must_use]
pub struct FileFilter {
    /// Dialog title; the OS default when `None`.
    pub title: Option<String>,
    /// File types offered. Empty allows all files.
    pub filters: Vec<FileType>,
    /// Folder the dialog opens in; the OS default when `None`. Native only.
    #[cfg(not(target_arch = "wasm32"))]
    pub directory: Option<PathBuf>,
    /// File name the dialog starts with; a suggested save name overrides it.
    pub file_name: Option<String>,
}

/// One named group of file extensions in a file picker.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct FileType {
    /// Label shown in the dialog.
    pub name: String,
    /// Extensions without the dot.
    pub extensions: Vec<String>,
}

impl FileType {
    /// `name` is the label; `extensions` omit the dot.
    pub fn new(name: impl Into<String>, extensions: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            name: name.into(),
            extensions: extensions.into_iter().map(Into::into).collect(),
        }
    }
}

impl FileFilter {
    /// Offers `file_type` only.
    pub fn new(file_type: impl Into<FileType>) -> Self {
        Self::default().file_type(file_type)
    }

    /// Offers another file type.
    pub fn file_type(mut self, file_type: impl Into<FileType>) -> Self {
        self.filters.push(file_type.into());
        self
    }

    /// Offers more file types.
    pub fn file_types(mut self, file_types: impl IntoIterator<Item = impl Into<FileType>>) -> Self {
        self.filters.extend(file_types.into_iter().map(Into::into));
        self
    }

    /// Dialog title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// File name the dialog starts with.
    pub fn file_name(mut self, file_name: impl Into<String>) -> Self {
        self.file_name = Some(file_name.into());
        self
    }

    /// Folder the dialog opens in.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.directory = Some(directory.into());
        self
    }
}

/// OS services behind async calls: file pickers, clipboard, notifications.
pub trait PlatformService {
    /// `Ok(None)` when the user cancelled.
    fn pick_file(&self, filter: FileFilter) -> BoxFuture<Result<Option<PickedFile>, PlatformError>>;
    /// Empty when the user cancelled.
    fn pick_files(&self, filter: FileFilter) -> BoxFuture<Result<Vec<PickedFile>, PlatformError>>;
    /// Like [`Self::pick_file`], read on demand instead of up front. `Ok(None)` when the user cancelled.
    fn pick_file_reader(&self, filter: FileFilter) -> BoxFuture<Result<Option<FileReader>, PlatformError>>;
    /// `Ok(None)` when the user cancelled; `filter`'s file types are ignored. Native only: browsers
    /// expose no filesystem paths.
    #[cfg(not(target_arch = "wasm32"))]
    fn pick_folder(&self, filter: FileFilter) -> BoxFuture<Result<Option<PathBuf>, PlatformError>>;
    /// [`SaveOutcome::Cancelled`] when the user dismissed the dialog. Browsers expose no saved path, so none is reported.
    /// Writes `bytes` whole; stream large files through [`Self::save_file_writer`].
    fn save_file(
        &self,
        suggested_name: String,
        filter: FileFilter,
        bytes: Vec<u8>,
    ) -> BoxFuture<Result<SaveOutcome, PlatformError>>;
    /// Like [`Self::save_file`], written in chunks; `Ok(None)` when the user cancelled.
    fn save_file_writer(
        &self,
        suggested_name: String,
        filter: FileFilter,
    ) -> BoxFuture<Result<Option<FileWriter>, PlatformError>>;
    /// Save target chosen by the user, for apps that write the file themselves. `Ok(None)` when
    /// the user cancelled. Native only.
    #[cfg(not(target_arch = "wasm32"))]
    fn pick_save_path(
        &self,
        suggested_name: String,
        filter: FileFilter,
    ) -> BoxFuture<Result<Option<PathBuf>, PlatformError>>;
    /// Plain-text clipboard contents.
    fn clipboard_text(&self) -> BoxFuture<Result<String, PlatformError>>;
    /// Replaces the clipboard with plain text.
    fn set_clipboard_text(&self, text: String) -> BoxFuture<Result<(), PlatformError>>;
    /// OS notification; [`PlatformError::PermissionDenied`] when the user refuses.
    fn notify(&self, notification: Notification) -> BoxFuture<Result<(), PlatformError>>;
}

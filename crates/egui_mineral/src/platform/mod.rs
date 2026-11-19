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

use std::path::{Path, PathBuf};

use crate::bus::BoxFuture;

/// Failure of a platform service call.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PlatformError {
    /// User or OS refused access (clipboard read, notifications).
    #[error("permission denied")]
    PermissionDenied,
    /// Backend failure; the cause is the [`source`](std::error::Error::source).
    #[error("platform backend failed: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

fn backend(e: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> PlatformError {
    PlatformError::Backend(e.into())
}

/// Outcome of [`PlatformPickers::save_file`].
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
#[non_exhaustive]
pub struct Notification {
    /// Summary line.
    pub title: String,
    /// Message text.
    pub body: String,
}

impl Notification {
    /// Notification with summary line `title` and message `body`.
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
        }
    }
}

/// File chosen through the platform file picker.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct PickedFile {
    name: String,
    bytes: Vec<u8>,
    path: Option<PathBuf>,
}

impl PickedFile {
    /// In-memory file `name` holding `bytes`.
    pub fn new(name: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            name: name.into(),
            bytes: bytes.into(),
            path: None,
        }
    }

    /// Location the file was read from.
    #[must_use]
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// File name without directories.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Full file contents, read when the pick resolved.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Contents, without copying.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// `None` in browsers and for in-memory files.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

/// File picker title, extension filters and start folder.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
#[must_use]
pub struct FileFilter {
    title: Option<String>,
    filters: Vec<FileType>,
    start: Option<PathBuf>,
    file_name: Option<String>,
}

/// One named group of file extensions in a file picker.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
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

    /// Dialog title; the OS default when unset.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// File name the dialog starts with; a [`SaveRequest`] name overrides it.
    pub fn file_name(mut self, file_name: impl Into<String>) -> Self {
        self.file_name = Some(file_name.into());
        self
    }

    /// Where the dialog opens, resolved when it opens: an existing folder as is, an existing file in its folder
    /// with its name proposed (over [`Self::file_name`]), a missing path at its parent; the OS default when unset.
    /// Browsers ignore it.
    pub fn start(mut self, path: impl Into<PathBuf>) -> Self {
        self.start = Some(path.into());
        self
    }

    /// File types offered; empty allows all files.
    pub fn filters(&self) -> &[FileType] {
        &self.filters
    }

    /// Dialog title.
    pub fn title_text(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// File name the dialog starts with.
    pub fn file_name_text(&self) -> Option<&str> {
        self.file_name.as_deref()
    }

    /// Path set by [`Self::start`].
    pub fn start_path(&self) -> Option<&Path> {
        self.start.as_deref()
    }

    /// Same dialog without file types.
    pub fn without_file_types(mut self) -> Self {
        self.filters.clear();
        self
    }
}

/// Save dialog request: the proposed file name and the dialog options.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
#[must_use]
pub struct SaveRequest {
    suggested_name: String,
    filter: FileFilter,
}

impl SaveRequest {
    /// Proposes `suggested_name`, any file type.
    pub fn new(suggested_name: impl Into<String>) -> Self {
        Self {
            suggested_name: suggested_name.into(),
            filter: FileFilter::default(),
        }
    }

    /// Dialog title, file types and start folder.
    pub fn filter(mut self, filter: FileFilter) -> Self {
        self.filter = filter;
        self
    }

    /// File name the dialog proposes.
    pub fn suggested_name(&self) -> &str {
        &self.suggested_name
    }

    /// Dialog options set by [`Self::filter`].
    pub fn file_filter(&self) -> &FileFilter {
        &self.filter
    }
}

/// Async file pickers.
pub trait PlatformPickers {
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
    fn save_file(&self, request: SaveRequest, bytes: Vec<u8>) -> BoxFuture<Result<SaveOutcome, PlatformError>>;
    /// Like [`Self::save_file`], written in chunks; `Ok(None)` when the user cancelled.
    fn save_file_writer(&self, request: SaveRequest) -> BoxFuture<Result<Option<FileWriter>, PlatformError>>;
    /// Save target chosen by the user, for apps that write the file themselves. `Ok(None)` when
    /// the user cancelled. Native only.
    #[cfg(not(target_arch = "wasm32"))]
    fn pick_save_path(&self, request: SaveRequest) -> BoxFuture<Result<Option<PathBuf>, PlatformError>>;
}

/// Async file pickers that report paths instead of contents. Native only: browsers expose no
/// filesystem paths.
#[cfg(not(target_arch = "wasm32"))]
pub trait PlatformPathPickers {
    /// Like [`PlatformPickers::pick_file`], returning only the path. `Ok(None)` when the user cancelled.
    fn pick_file_path(&self, filter: FileFilter) -> BoxFuture<Result<Option<PathBuf>, PlatformError>>;
    /// Like [`PlatformPickers::pick_files`], returning only the paths. Empty when the user cancelled.
    fn pick_file_paths(&self, filter: FileFilter) -> BoxFuture<Result<Vec<PathBuf>, PlatformError>>;
}

/// Async plain-text clipboard.
pub trait PlatformClipboard {
    /// Plain-text clipboard contents.
    fn clipboard_text(&self) -> BoxFuture<Result<String, PlatformError>>;
    /// Replaces the clipboard with plain text.
    fn set_clipboard_text(&self, text: String) -> BoxFuture<Result<(), PlatformError>>;
}

/// Async OS notifications.
pub trait PlatformNotifier {
    /// OS notification; [`PlatformError::PermissionDenied`] when the user refuses.
    fn notify(&self, notification: Notification) -> BoxFuture<Result<(), PlatformError>>;
}

/// All three capabilities, for `&dyn PlatformService`; implemented for every type that has them.
/// The set is permanent: a later capability is its own trait, never a supertrait added here.
pub trait PlatformService: PlatformPickers + PlatformClipboard + PlatformNotifier {}

impl<T: PlatformPickers + PlatformClipboard + PlatformNotifier + ?Sized> PlatformService for T {}

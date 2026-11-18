#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::PlatformError;
use crate::bus::BoxFuture;

#[derive(Clone)]
enum Source {
    Bytes(Arc<[u8]>),
    #[cfg(not(target_arch = "wasm32"))]
    Path(PathBuf),
    #[cfg(all(target_arch = "wasm32", feature = "platform-native"))]
    Web(web_sys::File),
}

/// Picked file read in chunks on demand, for files too large to hold in memory
/// ([`PlatformService::pick_file_reader`](super::PlatformService::pick_file_reader)).
///
/// Native reads block their thread on the file system while polled.
#[derive(Clone)]
pub struct FileReader {
    name: String,
    len: u64,
    source: Source,
}

crate::kit::opaque_debug!(FileReader);

impl FileReader {
    /// Reader over in-memory `bytes`, e.g. for tests.
    pub fn from_bytes(name: impl Into<String>, bytes: impl Into<Arc<[u8]>>) -> Self {
        let bytes = bytes.into();
        Self {
            name: name.into(),
            len: bytes.len() as u64,
            source: Source::Bytes(bytes),
        }
    }

    /// Reader over the file at `path`; its length is read now.
    ///
    /// # Errors
    /// [`PlatformError::Backend`] when the file's metadata cannot be read.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_path(path: impl Into<PathBuf>) -> Result<Self, PlatformError> {
        let path = path.into();
        let len = std::fs::metadata(&path).map_err(super::backend)?.len();
        Ok(Self {
            name: path
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
            len,
            source: Source::Path(path),
        })
    }

    #[cfg(all(target_arch = "wasm32", feature = "platform-native"))]
    pub(crate) fn from_web(file: web_sys::File) -> Self {
        // Browser file sizes are whole numbers of bytes below 2^53.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "whole, non-negative byte count"
        )]
        let len = file.size() as u64;
        Self {
            name: file.name(),
            len,
            source: Source::Web(file),
        }
    }

    /// File name without directories.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Size in bytes when picked.
    pub fn len(&self) -> u64 {
        self.len
    }

    /// `true` for an empty file.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Native file path; `None` for in-memory readers.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn path(&self) -> Option<&Path> {
        match &self.source {
            Source::Path(p) => Some(p),
            Source::Bytes(_) => None,
        }
    }

    /// Up to `len` bytes from `offset`; shorter at the end of the file, empty past it.
    pub fn read_at(&self, offset: u64, len: usize) -> BoxFuture<Result<Vec<u8>, PlatformError>> {
        let end = offset.saturating_add(len as u64).min(self.len);
        let source = self.source.clone();
        Box::pin(async move {
            if offset >= end {
                return Ok(Vec::new());
            }
            match source {
                Source::Bytes(b) => {
                    let range = usize::try_from(offset).map_err(super::backend)?
                        ..usize::try_from(end).map_err(super::backend)?;
                    Ok(b.get(range).map_or_else(Vec::new, <[u8]>::to_vec))
                }
                #[cfg(not(target_arch = "wasm32"))]
                Source::Path(path) => read_path(&path, offset, end - offset),
                #[cfg(all(target_arch = "wasm32", feature = "platform-native"))]
                Source::Web(file) => read_web(&file, offset, end).await,
            }
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_path(path: &Path, offset: u64, len: u64) -> Result<Vec<u8>, PlatformError> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).map_err(super::backend)?;
    file.seek(SeekFrom::Start(offset)).map_err(super::backend)?;
    let mut out = Vec::new();
    file.take(len).read_to_end(&mut out).map_err(super::backend)?;
    Ok(out)
}

#[cfg(all(target_arch = "wasm32", feature = "platform-native"))]
async fn read_web(file: &web_sys::File, offset: u64, end: u64) -> Result<Vec<u8>, PlatformError> {
    let js = |e: wasm_bindgen::JsValue| PlatformError::Backend(format!("{e:?}").into());
    // Offsets stay below 2^53, exact in f64.
    #[expect(clippy::cast_precision_loss, reason = "byte offsets below 2^53")]
    let blob = file.slice_with_f64_and_f64(offset as f64, end as f64).map_err(js)?;
    let buf = wasm_bindgen_futures::JsFuture::from(blob.array_buffer())
        .await
        .map_err(js)?;
    Ok(js_sys::Uint8Array::new(&buf).to_vec())
}

#[cfg(test)]
mod tests {
    use futures_util::FutureExt as _;

    use super::FileReader;

    fn read(r: &FileReader, offset: u64, len: usize) -> Vec<u8> {
        r.read_at(offset, len).now_or_never().expect("ready").expect("read")
    }

    #[test]
    fn reads_chunks_clamped_to_the_end() {
        let bytes = FileReader::from_bytes("a.bin", b"0123456789".to_vec());
        let path = std::env::temp_dir().join(format!("mineral-reader-{}", std::process::id()));
        std::fs::write(&path, b"0123456789").expect("temp file");
        let file = FileReader::from_path(&path).expect("metadata");
        for r in [&bytes, &file] {
            assert_eq!(r.len(), 10);
            assert_eq!(read(r, 2, 3), b"234");
            assert_eq!(read(r, 8, 5), b"89", "short at the end");
            assert_eq!(read(r, 10, 4), b"", "empty past it");
            assert_eq!(read(r, u64::MAX, usize::MAX), b"");
        }
        assert_eq!(file.path(), Some(path.as_path()));
        std::fs::remove_file(&path).expect("temp file removed");
    }
}

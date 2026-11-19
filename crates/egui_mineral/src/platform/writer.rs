#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

use super::PlatformError;
use crate::bus::{BoxFuture, MaybeSend};

#[cfg(not(target_arch = "wasm32"))]
type Finish = Box<dyn FnOnce(String, Vec<u8>) -> BoxFuture<Result<(), PlatformError>> + Send>;
#[cfg(target_arch = "wasm32")]
type Finish = Box<dyn FnOnce(String, Vec<u8>) -> BoxFuture<Result<(), PlatformError>>>;

enum Sink {
    Buffer(Vec<u8>, Finish),
    #[cfg(not(target_arch = "wasm32"))]
    File(std::fs::File, PathBuf),
    #[cfg(all(target_arch = "wasm32", feature = "web-save-stream"))]
    Web(wasm_bindgen::JsValue),
    Closed,
}

/// Save target written in chunks, for files too large to build in memory
/// ([`PlatformPickers::save_file_writer`](super::PlatformPickers::save_file_writer)).
///
/// Native writes block their thread on the file system while polled. Browsers without a writable
/// file stream (all but Chromium with `web-save-stream`) buffer and download on [`close`](Self::close).
/// Dropping the writer before `close` aborts a browser stream; a native file keeps what was written.
#[must_use = "call .close() to finish the file"]
pub struct FileWriter {
    name: String,
    written: u64,
    sink: Sink,
}

crate::kit::opaque_debug!(FileWriter);

impl FileWriter {
    /// Buffers every write; `finish(name, bytes)` stores the whole file on [`close`](Self::close).
    pub fn buffered(
        name: impl Into<String>,
        finish: impl FnOnce(String, Vec<u8>) -> BoxFuture<Result<(), PlatformError>> + MaybeSend + 'static,
    ) -> Self {
        Self::new(name.into(), Sink::Buffer(Vec::new(), Box::new(finish)))
    }

    /// Writer over a new or truncated file at `path`.
    ///
    /// # Errors
    /// [`PlatformError::Backend`] when the file cannot be created.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn create(path: impl Into<PathBuf>) -> Result<Self, PlatformError> {
        let path = path.into();
        let file = std::fs::File::create(&path).map_err(super::backend)?;
        let name = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        Ok(Self::new(name, Sink::File(file, path)))
    }

    // `stream` is a `FileSystemWritableFileStream`.
    #[cfg(all(target_arch = "wasm32", feature = "web-save-stream"))]
    pub(crate) fn web(name: String, stream: wasm_bindgen::JsValue) -> Self {
        Self::new(name, Sink::Web(stream))
    }

    fn new(name: String, sink: Sink) -> Self {
        Self { name, written: 0, sink }
    }

    /// File name without directories.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Bytes written so far.
    pub fn written(&self) -> u64 {
        self.written
    }

    /// Native file path; `None` for buffered writers.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn path(&self) -> Option<&Path> {
        match &self.sink {
            Sink::File(_, p) => Some(p),
            _ => None,
        }
    }

    /// Appends `chunk`.
    ///
    /// # Errors
    /// [`PlatformError::Backend`] when the target rejects the write (disk full, download
    /// cancelled, writer already failed).
    #[cfg_attr(
        not(all(target_arch = "wasm32", feature = "web-save-stream")),
        expect(
            clippy::unused_async,
            clippy::unused_async_trait_impl,
            reason = "awaits only on the web stream"
        )
    )]
    pub async fn write(&mut self, chunk: &[u8]) -> Result<(), PlatformError> {
        match &mut self.sink {
            Sink::Buffer(buf, _) => buf.extend_from_slice(chunk),
            #[cfg(not(target_arch = "wasm32"))]
            Sink::File(file, _) => std::io::Write::write_all(file, chunk).map_err(super::backend)?,
            #[cfg(all(target_arch = "wasm32", feature = "web-save-stream"))]
            Sink::Web(stream) => {
                let data = js_sys::Uint8Array::from(chunk);
                web::call(stream, "write", &[data.into()]).await?;
            }
            Sink::Closed => return Err(super::backend("file writer is closed")),
        }
        self.written += chunk.len() as u64;
        Ok(())
    }

    /// Commits the file: syncs it natively, ends the browser stream or stores the buffer.
    ///
    /// # Errors
    /// [`PlatformError::Backend`] when committing fails.
    pub async fn close(mut self) -> Result<(), PlatformError> {
        match std::mem::replace(&mut self.sink, Sink::Closed) {
            Sink::Buffer(buf, finish) => finish(std::mem::take(&mut self.name), buf).await,
            #[cfg(not(target_arch = "wasm32"))]
            Sink::File(file, _) => file.sync_all().map_err(super::backend),
            #[cfg(all(target_arch = "wasm32", feature = "web-save-stream"))]
            Sink::Web(stream) => web::call(&stream, "close", &[]).await.map(drop),
            Sink::Closed => Ok(()),
        }
    }
}

#[cfg(all(target_arch = "wasm32", feature = "web-save-stream"))]
impl Drop for FileWriter {
    fn drop(&mut self) {
        if let Sink::Web(stream) = &self.sink {
            // Discards the temporary file; the target stays untouched.
            drop(web::call(stream, "abort", &[]));
        }
    }
}

// File System Access API through reflection: web-sys gates it behind `web_sys_unstable_apis`.
#[cfg(all(target_arch = "wasm32", feature = "web-save-stream"))]
pub(super) mod web {
    use js_sys::{Array, Function, Object, Promise, Reflect};
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;

    use crate::platform::{FileFilter, PlatformError};

    fn js(e: JsValue) -> PlatformError {
        PlatformError::Backend(Box::new(crate::js::JsError::from(e)))
    }

    // Calls `target[method](...args)`, awaiting a returned promise.
    async fn call_raw(target: &JsValue, method: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
        let f: Function = Reflect::get(target, &method.into())?.dyn_into()?;
        let v = f.apply(target, &args.iter().collect::<Array>())?;
        match v.dyn_into::<Promise>() {
            Ok(p) => JsFuture::from(p).await,
            Err(v) => Ok(v),
        }
    }

    pub(crate) async fn call(target: &JsValue, method: &str, args: &[JsValue]) -> Result<JsValue, PlatformError> {
        call_raw(target, method, args).await.map_err(js)
    }

    fn options(name: &str, filter: &FileFilter) -> Result<Object, PlatformError> {
        let set = |o: &Object, k: &str, v: &JsValue| Reflect::set(o, &k.into(), v).map(drop).map_err(js);
        let opts = Object::new();
        set(&opts, "suggestedName", &name.into())?;
        let types = Array::new();
        for t in &filter.filters {
            let exts: Array = t.extensions.iter().map(|e| JsValue::from(format!(".{e}"))).collect();
            let accept = Object::new();
            set(&accept, "application/octet-stream", &exts)?;
            let ty = Object::new();
            set(&ty, "description", &t.name.as_str().into())?;
            set(&ty, "accept", &accept)?;
            types.push(&ty);
        }
        if types.length() > 0 {
            set(&opts, "types", &types)?;
        }
        Ok(opts)
    }

    // `None`: no `showSaveFilePicker`; `Some(None)`: the user cancelled.
    pub(crate) async fn open(name: &str, filter: &FileFilter) -> Result<Option<Option<JsValue>>, PlatformError> {
        let Some(window) = web_sys::window() else {
            return Ok(None);
        };
        if !Reflect::has(&window, &"showSaveFilePicker".into()).unwrap_or(false) {
            return Ok(None);
        }
        let handle = match call_raw(&window, "showSaveFilePicker", &[options(name, filter)?.into()]).await {
            Ok(h) => h,
            Err(e)
                if Reflect::get(&e, &"name".into())
                    .ok()
                    .and_then(|n| n.as_string())
                    .as_deref()
                    == Some("AbortError") =>
            {
                return Ok(Some(None));
            }
            Err(e) => return Err(js(e)),
        };
        Ok(Some(Some(call(&handle, "createWritable", &[]).await?)))
    }
}

#[cfg(test)]
mod tests {
    use futures_util::FutureExt as _;
    use parking_lot::Mutex;
    use std::sync::Arc;

    use super::FileWriter;

    #[test]
    fn buffered_and_file_writers_commit_on_close() {
        let got = Arc::new(Mutex::new(None));
        let sink = Arc::clone(&got);
        let mut w = FileWriter::buffered("a.bin", move |name, bytes| {
            *sink.lock() = Some((name, bytes));
            Box::pin(async { Ok(()) })
        });
        let path = std::env::temp_dir().join(format!("mineral-writer-{}", std::process::id()));
        let mut f = FileWriter::create(&path).expect("temp file");
        for w in [&mut w, &mut f] {
            w.write(b"012").now_or_never().expect("ready").expect("write");
            w.write(b"34").now_or_never().expect("ready").expect("write");
            assert_eq!(w.written(), 5);
        }
        assert!(got.lock().is_none(), "buffered until close");
        w.close().now_or_never().expect("ready").expect("close");
        f.close().now_or_never().expect("ready").expect("close");
        assert_eq!(*got.lock(), Some(("a.bin".to_owned(), b"01234".to_vec())));
        assert_eq!(std::fs::read(&path).expect("written"), b"01234");
        std::fs::remove_file(&path).expect("temp file removed");
    }
}

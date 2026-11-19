use serde::{Serialize, de::DeserializeOwned};

/// Failure reading, writing or encoding persisted state.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PersistError {
    /// Filesystem failure.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Encode or decode failure of the codec.
    #[error("codec: {0}")]
    Codec(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// Key the backend cannot store under (e.g. empty, `.`, `..`).
    #[error("invalid key `{0}`")]
    InvalidKey(String),
    /// No config directory on this platform (`FsStorage::for_app`).
    #[error("no config directory")]
    NoConfigDir,
    /// Backend failure (e.g. browser storage unavailable).
    #[error("backend: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Key-value string store.
pub trait Storage {
    /// `Ok(None)` when absent.
    ///
    /// # Errors
    /// When the backend cannot read `key`.
    fn get(&self, key: &str) -> Result<Option<String>, PersistError>;
    /// Overwrites any existing value.
    ///
    /// # Errors
    /// When the backend rejects the write.
    fn set(&mut self, key: &str, value: String) -> Result<(), PersistError>;
    /// No error when absent.
    ///
    /// # Errors
    /// When the backend cannot delete `key`.
    fn remove(&mut self, key: &str) -> Result<(), PersistError>;
}

/// Serialization format for stored values.
pub trait Codec {
    /// Text form of `value`.
    ///
    /// # Errors
    /// When `value` cannot be serialized.
    fn encode<T: Serialize>(value: &T) -> Result<String, PersistError>;
    /// Inverse of [`Self::encode`].
    ///
    /// # Errors
    /// When `raw` does not parse as `T`.
    fn decode<T: DeserializeOwned>(raw: &str) -> Result<T, PersistError>;
}

/// Decodes the value at `key`; `None` when absent.
///
/// # Errors
/// When the storage read or the decode fails.
pub fn load<C: Codec, T: DeserializeOwned>(s: &impl Storage, key: &str) -> Result<Option<T>, PersistError> {
    s.get(key)?.map(|raw| C::decode(&raw)).transpose()
}

/// Encodes `value` and stores it at `key`.
///
/// # Errors
/// When the encode or the storage write fails.
pub fn save<C: Codec, T: Serialize>(s: &mut impl Storage, key: &str, value: &T) -> Result<(), PersistError> {
    s.set(key, C::encode(value)?)
}

/// In-memory [`Storage`].
#[derive(Clone, Debug, Default)]
pub struct MemoryStorage(std::collections::HashMap<String, String>);

impl Storage for MemoryStorage {
    fn get(&self, key: &str) -> Result<Option<String>, PersistError> {
        Ok(self.0.get(key).cloned())
    }

    fn set(&mut self, key: &str, value: String) -> Result<(), PersistError> {
        self.0.insert(key.into(), value);
        Ok(())
    }

    fn remove(&mut self, key: &str) -> Result<(), PersistError> {
        self.0.remove(key);
        Ok(())
    }
}

/// JSON [`Codec`].
#[cfg(feature = "persist-json")]
#[derive(Clone, Debug)]
pub struct Json;

#[cfg(feature = "persist-json")]
impl Codec for Json {
    fn encode<T: serde::Serialize>(value: &T) -> Result<String, PersistError> {
        serde_json::to_string_pretty(value).map_err(|e| PersistError::Codec(e.into()))
    }

    fn decode<T: serde::de::DeserializeOwned>(raw: &str) -> Result<T, PersistError> {
        serde_json::from_str(raw).map_err(|e| PersistError::Codec(e.into()))
    }
}

/// TOML [`Codec`].
#[cfg(feature = "persist-toml")]
#[derive(Clone, Debug)]
pub struct Toml;

#[cfg(feature = "persist-toml")]
impl Codec for Toml {
    fn encode<T: serde::Serialize>(value: &T) -> Result<String, PersistError> {
        toml::to_string_pretty(value).map_err(|e| PersistError::Codec(e.into()))
    }

    fn decode<T: serde::de::DeserializeOwned>(raw: &str) -> Result<T, PersistError> {
        toml::from_str(raw).map_err(|e| PersistError::Codec(e.into()))
    }
}

#[cfg(all(feature = "persist-fs", not(target_arch = "wasm32")))]
pub use fs::FsStorage;

#[cfg(all(feature = "persist-fs", not(target_arch = "wasm32")))]
mod fs {
    use std::path::PathBuf;

    use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

    use super::{PersistError, Storage};

    /// One file per key under `root`; writes are atomic (temp file + rename).
    #[derive(Clone, Debug)]
    pub struct FsStorage {
        root: PathBuf,
    }

    impl FsStorage {
        /// Makes `root` and its parents when missing.
        ///
        /// # Errors
        /// When `root` or a parent cannot be created.
        pub fn new(root: impl Into<PathBuf>) -> Result<Self, PersistError> {
            let root = root.into();
            std::fs::create_dir_all(&root)?;
            Ok(Self { root })
        }

        /// `<app>` under the platform config dir ([`dirs::config_dir`]); `app` is one path segment, encoded like keys.
        ///
        /// # Errors
        /// [`PersistError::NoConfigDir`], [`PersistError::InvalidKey`] for an empty, `.` or `..` `app`, or when the
        /// app directory cannot be created.
        pub fn for_app(app: &str) -> Result<Self, PersistError> {
            Self::under(&dirs::config_dir().ok_or(PersistError::NoConfigDir)?, app)
        }

        fn under(base: &std::path::Path, app: &str) -> Result<Self, PersistError> {
            Self::new(base.join(Self::segment(app)?))
        }

        const KEY_ESCAPE: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'_').remove(b'.');

        fn segment(name: &str) -> Result<String, PersistError> {
            if name.is_empty() || name == "." || name == ".." {
                return Err(PersistError::InvalidKey(name.into()));
            }
            Ok(utf8_percent_encode(name, Self::KEY_ESCAPE).to_string())
        }

        fn path(&self, key: &str) -> Result<PathBuf, PersistError> {
            Ok(self.root.join(Self::segment(key)?))
        }
    }

    impl Storage for FsStorage {
        fn get(&self, key: &str) -> Result<Option<String>, PersistError> {
            match std::fs::read_to_string(self.path(key)?) {
                Ok(s) => Ok(Some(s)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(e.into()),
            }
        }

        fn set(&mut self, key: &str, value: String) -> Result<(), PersistError> {
            let path = self.path(key)?;
            // "%tm" is never produced by key encoding, so no key maps to a temp path.
            let mut tmp = path.clone().into_os_string();
            tmp.push("%tmp");
            let tmp = PathBuf::from(tmp);
            std::fs::write(&tmp, value)?;
            std::fs::rename(&tmp, &path).map_err(|e| {
                if let Err(rm) = std::fs::remove_file(&tmp) {
                    tracing::warn!(path = %tmp.display(), error = %rm, "temp file left behind");
                }
                e.into()
            })
        }

        fn remove(&mut self, key: &str) -> Result<(), PersistError> {
            match std::fs::remove_file(self.path(key)?) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
                _ => Ok(()),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn dotted_keys_do_not_share_temp_path() {
            let dir = std::env::temp_dir().join(format!("mineral-persist-dots-{}", std::process::id()));
            let mut s = FsStorage::new(&dir).unwrap();
            // `FsStorage::set` writes key `a.b` through this path.
            let temp_of_a_b = "a.b%tmp";
            s.set(temp_of_a_b, "3".into()).unwrap();
            s.set("a.b", "1".into()).unwrap();
            s.set("a.c", "2".into()).unwrap();
            assert_eq!(s.get("a.b").unwrap().as_deref(), Some("1"));
            assert_eq!(s.get("a.c").unwrap().as_deref(), Some("2"));
            assert_eq!(s.get(temp_of_a_b).unwrap().as_deref(), Some("3"));
            std::fs::remove_dir_all(&dir).unwrap();
        }

        #[test]
        fn fs_roundtrip_with_odd_keys() {
            let dir = std::env::temp_dir().join(format!("mineral-persist-{}", std::process::id()));
            let mut s = FsStorage::new(&dir).unwrap();
            s.set("a/b c", "[1,2,3]".into()).unwrap();
            assert_eq!(s.get("a/b c").unwrap().as_deref(), Some("[1,2,3]"));
            s.remove("a/b c").unwrap();
            assert_eq!(s.get("a/b c").unwrap(), None);
            assert!(s.get("..").is_err());
            std::fs::remove_dir_all(dir).unwrap();
        }

        #[test]
        fn app_dir_stays_under_the_config_dir() {
            let base = std::env::temp_dir().join(format!("mineral-persist-app-{}", std::process::id()));
            for bad in ["", ".", ".."] {
                assert!(
                    matches!(FsStorage::under(&base, bad), Err(PersistError::InvalidKey(_))),
                    "{bad:?}"
                );
            }
            for app in ["../up", "/abs", "a/b"] {
                let s = FsStorage::under(&base, app).unwrap();
                assert_eq!(s.root.parent(), Some(base.as_path()), "{app:?}");
            }
            std::fs::remove_dir_all(base).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_storage_save_load_remove() {
        let mut s = MemoryStorage::default();
        assert_eq!(s.get("k").unwrap(), None);
        s.set("k", "v".into()).unwrap();
        assert_eq!(s.get("k").unwrap().as_deref(), Some("v"));
        s.remove("k").unwrap();
        assert_eq!(s.get("k").unwrap(), None);
        s.remove("k").unwrap();
    }

    #[cfg(feature = "persist-toml")]
    #[test]
    fn toml_roundtrip_and_corrupt_text_is_a_codec_error() {
        #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
        struct Prefs {
            name: String,
            size: u32,
        }

        let mut s = MemoryStorage::default();
        let prefs = Prefs {
            name: "a".into(),
            size: 3,
        };
        save::<Toml, _>(&mut s, "prefs", &prefs).unwrap();
        assert_eq!(load::<Toml, Prefs>(&s, "prefs").unwrap(), Some(prefs));
        assert_eq!(load::<Toml, Prefs>(&s, "absent").unwrap(), None);

        s.set("prefs", "not = [valid".into()).unwrap();
        let err = load::<Toml, Prefs>(&s, "prefs").unwrap_err();
        assert!(matches!(err, PersistError::Codec(_)));
        assert!(
            std::error::Error::source(&err).is_some(),
            "the codec error is the source"
        );
        assert!(err.to_string().starts_with("codec: "));
    }
}

/// Browser `localStorage`, keys prefixed with `namespace:`.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug)]
pub struct LocalStorage {
    namespace: String,
    storage: web_sys::Storage,
}

#[cfg(target_arch = "wasm32")]
impl LocalStorage {
    /// Store under `namespace`.
    ///
    /// # Errors
    /// [`PersistError::Backend`] without a `window` or when the browser denies `localStorage`.
    pub fn new(namespace: impl Into<String>) -> Result<Self, PersistError> {
        let window = web_sys::window().ok_or_else(|| PersistError::Backend("no window".into()))?;
        let storage = window
            .local_storage()
            .map_err(js_err)?
            .ok_or_else(|| PersistError::Backend("localStorage unavailable".into()))?;
        Ok(Self {
            namespace: namespace.into(),
            storage,
        })
    }

    fn key(&self, key: &str) -> String {
        format!("{}:{key}", self.namespace)
    }
}

#[cfg(target_arch = "wasm32")]
impl Storage for LocalStorage {
    fn get(&self, key: &str) -> Result<Option<String>, PersistError> {
        self.storage.get_item(&self.key(key)).map_err(js_err)
    }

    fn set(&mut self, key: &str, value: String) -> Result<(), PersistError> {
        self.storage.set_item(&self.key(key), &value).map_err(js_err)
    }

    fn remove(&mut self, key: &str) -> Result<(), PersistError> {
        self.storage.remove_item(&self.key(key)).map_err(js_err)
    }
}

#[cfg(target_arch = "wasm32")]
fn js_err(e: wasm_bindgen::JsValue) -> PersistError {
    PersistError::Backend(Box::new(crate::js::JsError::from(e)))
}

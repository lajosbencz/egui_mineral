use wasm_bindgen::{JsCast, JsValue};

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub(crate) struct JsError(String);

impl From<JsValue> for JsError {
    fn from(e: JsValue) -> Self {
        Self(match e.dyn_ref::<js_sys::Error>() {
            Some(err) => String::from(err.message()),
            None => e.as_string().unwrap_or_else(|| "non-error JS exception".to_owned()),
        })
    }
}

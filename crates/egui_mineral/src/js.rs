use wasm_bindgen::{JsCast, JsValue};

// `name` and `stack` exist only for `Error` objects.
#[derive(Debug, thiserror::Error)]
#[error("{}", self.display())]
#[non_exhaustive]
pub(crate) struct JsError {
    pub(crate) name: Option<String>,
    pub(crate) message: String,
    pub(crate) stack: Option<String>,
}

impl JsError {
    fn display(&self) -> String {
        match &self.name {
            Some(name) => format!("{name}: {}", self.message),
            None => self.message.clone(),
        }
    }
}

impl From<JsValue> for JsError {
    fn from(e: JsValue) -> Self {
        match e.dyn_ref::<js_sys::Error>() {
            Some(err) => Self {
                name: Some(String::from(err.name())),
                message: String::from(err.message()),
                stack: js_sys::Reflect::get(err, &JsValue::from_str("stack"))
                    .ok()
                    .and_then(|s| s.as_string()),
            },
            None => Self {
                name: None,
                message: e.as_string().unwrap_or_else(|| "non-error JS exception".to_owned()),
                stack: None,
            },
        }
    }
}

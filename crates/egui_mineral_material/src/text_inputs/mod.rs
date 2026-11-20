//! Text inputs: text fields ([m3](https://m3.material.io/components)).

pub(crate) mod form;
pub(crate) mod text_field;

pub use form::{FormExt, FormUiExt};
pub use text_field::{Elide, TextField, TextFieldOutput, TextFieldVariant};

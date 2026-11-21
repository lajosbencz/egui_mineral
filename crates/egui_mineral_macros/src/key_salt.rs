use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Error, Fields, LitStr, spanned::Spanned};

struct Attrs {
    root: Option<syn::Path>,
    rename: Option<LitStr>,
}

fn attrs(attrs: &[syn::Attribute], allow_crate: bool) -> syn::Result<Attrs> {
    let mut out = Attrs {
        root: None,
        rename: None,
    };
    for attr in attrs.iter().filter(|a| a.path().is_ident("key_salt")) {
        attr.parse_nested_meta(|meta| {
            if allow_crate && meta.path.is_ident("crate") {
                out.root = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("rename") {
                out.rename = Some(meta.value()?.parse()?);
            } else {
                return Err(meta.error(if allow_crate {
                    "expected `crate = path` or `rename = \"key\"`"
                } else {
                    "expected `rename = \"key\"`"
                }));
            }
            Ok(())
        })?;
    }
    Ok(out)
}

// `ListItem` -> `list-item`, `HTTPServer` -> `http-server`.
fn kebab(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::with_capacity(name.len().saturating_add(4));
    for (i, c) in chars.iter().enumerate() {
        if c.is_uppercase() {
            let prev_lower = i > 0
                && chars
                    .get(i.wrapping_sub(1))
                    .is_some_and(|p| p.is_lowercase() || p.is_ascii_digit());
            let next_lower = chars.get(i.saturating_add(1)).is_some_and(|n| n.is_lowercase());
            let prev_upper = i > 0 && chars.get(i.wrapping_sub(1)).is_some_and(|p| p.is_uppercase());
            if i > 0 && (prev_lower || (prev_upper && next_lower)) {
                out.push('-');
            }
            out.extend(c.to_lowercase());
        } else if *c == '_' {
            out.push('-');
        } else {
            out.push(*c);
        }
    }
    out
}

fn name_of(ident: &syn::Ident, rename: Option<&LitStr>) -> syn::Result<LitStr> {
    let text = rename.map_or_else(|| kebab(&ident.to_string()), LitStr::value);
    let valid = !text.is_empty()
        && text
            .split('-')
            .all(|w| !w.is_empty() && w.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()));
    if !valid {
        let span = rename.map_or_else(|| ident.span(), LitStr::span);
        return Err(Error::new(
            span,
            format!("key `{text}` must be lowercase ASCII words joined by single `-`"),
        ));
    }
    Ok(LitStr::new(&text, ident.span()))
}

fn fields(fields: &Fields) -> (TokenStream2, Vec<syn::Ident>) {
    let ids: Vec<syn::Ident> = (0..fields.len()).map(|i| format_ident!("__f{i}")).collect();
    let pat = match fields {
        Fields::Unit => quote!(),
        Fields::Unnamed(_) => quote!((#(#ids),*)),
        Fields::Named(f) => {
            let names = f.named.iter().filter_map(|f| f.ident.as_ref());
            quote!({ #(#names: #ids),* })
        }
    };
    (pat, ids)
}

pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let own = attrs(&input.attrs, true)?;
    let root = own.root.unwrap_or_else(crate::host);
    let key = quote!(#root::semantic);
    let name = &input.ident;
    // Type parameters are keyed by their own `KeySalt`.
    let mut generics = input.generics.clone();
    let params: Vec<_> = generics.type_params().map(|p| p.ident.clone()).collect();
    let clause = generics.make_where_clause();
    for p in params {
        clause.predicates.push(syn::parse_quote!(#p: #key::KeySalt));
    }
    let (impl_g, ty_g, where_g) = generics.split_for_impl();

    // (pattern, leading name: `None` for a struct keyed by its fields alone, field bindings)
    let mut arms = Vec::new();
    match &input.data {
        Data::Enum(data) => {
            for v in &data.variants {
                let a = attrs(&v.attrs, false)?;
                let text = name_of(&v.ident, a.rename.as_ref())?;
                let (pat, ids) = fields(&v.fields);
                let ident = &v.ident;
                arms.push((quote!(Self::#ident #pat), Some(text), ids));
            }
        }
        Data::Struct(data) => {
            let (pat, ids) = fields(&data.fields);
            let text = match (&data.fields, own.rename.as_ref()) {
                (Fields::Unit, r) => Some(name_of(name, r)?),
                (_, Some(r)) => Some(name_of(name, Some(r))?),
                _ => None,
            };
            arms.push((quote!(Self #pat), text, ids));
        }
        Data::Union(_) => return Err(Error::new(input.span(), "KeySalt cannot be derived for unions")),
    }

    let write = arms.iter().map(|(pat, text, ids)| {
        let head = text.as_ref().map(|t| quote!(__out.push_str(#t);));
        let rest = ids.iter().enumerate().map(|(i, id)| {
            let sep = (text.is_some() || i > 0).then(|| quote!(__out.push('-');));
            quote!(#sep #key::KeySalt::write_key(#id, __out);)
        });
        quote!(#pat => { #head #(#rest)* })
    });
    let parts = arms.iter().map(|(pat, text, ids)| {
        let head = text.as_ref().map(|t| quote!(__parts.push_str(#t) &&));
        quote!(#pat => #head #(#key::KeySalt::key_parts(#ids, __parts) &&)* true)
    });
    let allow = quote!(#[allow(unused_variables, clippy::match_single_binding)]);
    Ok(quote! {
        #[automatically_derived]
        impl #impl_g #key::KeySalt for #name #ty_g #where_g {
            #allow
            fn write_key(&self, __out: &mut ::std::string::String) {
                match self { #(#write)* }
            }

            #allow
            fn key_parts(&self, __parts: &mut #key::KeyParts) -> bool {
                match self { #(#parts,)* }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::kebab;

    #[test]
    fn kebab_splits_words_and_acronyms() {
        assert_eq!(kebab("Home"), "home");
        assert_eq!(kebab("ListItem"), "list-item");
        assert_eq!(kebab("HTTPServer"), "http-server");
        assert_eq!(kebab("Page2Up"), "page2-up");
        assert_eq!(kebab("snake_case"), "snake-case");
    }
}

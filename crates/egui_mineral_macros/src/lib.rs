//! Macros for [egui_mineral](https://crates.io/crates/egui_mineral). Do not depend on this crate:
//! `egui_mineral` re-exports it.
//!
//! `#[derive(Route)]` implements `egui_mineral::nav::Route` for an enum whose variants carry
//! `#[route("/user/{id}")]` patterns (`{param?}` for a trailing optional parameter). Use it through
//! `egui_mineral::nav::Route`, which names both the trait and this derive. The derives find `egui_mineral` from the
//! calling crate's manifest: its own dependency, else the re-export of the stylekit it depends on
//! (`egui_mineral_material::egui_mineral`); `#[route(crate = path)]` and `#[key_salt(crate = path)]` name it
//! explicitly.
//!
//! `#[derive(KeySalt)]` implements `egui_mineral::semantic::KeySalt`: a unit variant or unit struct is keyed by its
//! kebab-case name (`ListItem` -> `list-item`), fields append their own keys joined by `-`, a struct with fields is
//! keyed by its fields alone. `#[key_salt(rename = "key")]` on a variant or the type fixes the name,
//! `#[key_salt(crate = path)]` on the type overrides the found `egui_mineral`.
//!
//! `messages!(mod msg = "locales/en-US.ftl")` turns the Fluent catalog at that path (relative to the crate root)
//! into a module of typed message ids. Use it through `egui_mineral::i18n::messages`, which documents it.

#![warn(clippy::arithmetic_side_effects)]

mod key_salt;
mod messages;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, LitStr, parse_macro_input, spanned::Spanned};

enum Segment {
    Lit(String),
    Param(syn::Ident),
    OptParam(syn::Ident),
}

fn parse_pattern(lit: &LitStr) -> syn::Result<Vec<Segment>> {
    let value = lit.value();
    if !value.starts_with('/') {
        return Err(Error::new(lit.span(), "route pattern must start with `/`"));
    }
    let mut segs = Vec::new();
    for raw in value.split('/').filter(|s| !s.is_empty()) {
        let seg = if let Some(inner) = raw.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            match inner.strip_suffix('?') {
                Some(name) => Segment::OptParam(param_ident(name, lit)?),
                None => Segment::Param(param_ident(inner, lit)?),
            }
        } else {
            Segment::Lit(raw.to_owned())
        };
        if matches!(segs.last(), Some(Segment::OptParam(_))) {
            return Err(Error::new(lit.span(), "optional `{param?}` must be the last segment"));
        }
        segs.push(seg);
    }
    Ok(segs)
}

fn param_ident(name: &str, lit: &LitStr) -> syn::Result<syn::Ident> {
    let mut ident = syn::parse_str::<syn::Ident>(name)
        .map_err(|_| Error::new(lit.span(), format!("`{{{name}}}` is not a valid field name")))?;
    ident.set_span(lit.span());
    Ok(ident)
}

fn parser(ident: &syn::Ident, segs: &[Segment], bind: &TokenStream2, nav: &TokenStream2) -> TokenStream2 {
    let private = quote!(#nav::__private);
    let len = segs.len();
    let min = if matches!(segs.last(), Some(Segment::OptParam(_))) {
        len.saturating_sub(1)
    } else {
        len
    };
    let literals = segs.iter().enumerate().filter_map(|(i, s)| match s {
        Segment::Lit(l) => Some(quote!(&& __segs[#i] == #l)),
        _ => None,
    });
    let params = segs.iter().enumerate().filter_map(|(i, s)| match s {
        Segment::Lit(_) => None,
        Segment::Param(p) => Some(quote!(let #p = #private::param(__path, stringify!(#p), __segs[#i])?;)),
        Segment::OptParam(p) => Some(quote! {
            let #p = match __segs.get(#i) {
                ::core::option::Option::Some(__s) => ::core::option::Option::Some(#private::param(__path, stringify!(#p), __s)?),
                ::core::option::Option::None => ::core::option::Option::None,
            };
        }),
    });
    quote! {
        if (#min..=#len).contains(&__segs.len()) #(#literals)* {
            match (|| -> ::core::result::Result<Self, #nav::RouteError> {
                #(#params)*
                ::core::result::Result::Ok(Self::#ident #bind)
            })() {
                ::core::result::Result::Ok(__r) => return ::core::result::Result::Ok(__r),
                ::core::result::Result::Err(__e) => {
                    __err.get_or_insert(__e);
                }
            }
        }
    }
}

/// Derives `KeySalt`; see the crate docs for the key of each shape.
#[proc_macro_derive(KeySalt, attributes(key_salt))]
pub fn derive_key_salt(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    key_salt::expand(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// Derives `Route` for an enum from `#[route("/path")]` attributes. The enum itself may carry
/// `#[route(crate = path)]` naming the crate that hosts `nav` (default: found in the calling crate's manifest, see
/// the crate docs).
#[proc_macro_derive(Route, attributes(route))]
pub fn derive_route(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(&input).unwrap_or_else(Error::into_compile_error).into()
}

// Path of `egui_mineral` from the calling crate: its own dependency, else the re-export of a stylekit it depends on.
// `egui_mineral` itself declares `extern crate self as egui_mineral`; its doctests also report `Itself`.
pub(crate) fn host() -> syn::Path {
    use proc_macro_crate::{FoundCrate, crate_name};
    let named = |name: &str| syn::Ident::new(name, proc_macro2::Span::call_site());
    match crate_name("egui_mineral") {
        Ok(FoundCrate::Itself) => return syn::parse_quote!(::egui_mineral),
        Ok(FoundCrate::Name(n)) => {
            let n = named(&n);
            return syn::parse_quote!(::#n);
        }
        Err(_) => {}
    }
    // A stylekit depends on `egui_mineral` itself, so it never reaches here as `Itself`.
    for kit in ["egui_mineral_material", "egui_mineral_fluent"] {
        if let Ok(FoundCrate::Name(n)) = crate_name(kit) {
            let n = named(&n);
            return syn::parse_quote!(::#n::egui_mineral);
        }
    }
    syn::parse_quote!(::egui_mineral)
}

fn crate_path(attrs: &[syn::Attribute]) -> syn::Result<syn::Path> {
    let mut root = None;
    for attr in attrs.iter().filter(|a| a.path().is_ident("route")) {
        attr.parse_nested_meta(|meta| {
            if !meta.path.is_ident("crate") {
                return Err(meta.error("expected `crate = path`"));
            }
            root = Some(meta.value()?.parse::<syn::Path>()?);
            Ok(())
        })?;
    }
    Ok(root.unwrap_or_else(host))
}

fn expand(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let Data::Enum(data) = &input.data else {
        return Err(Error::new(input.span(), "Route can only be derived for enums"));
    };
    let name = &input.ident;
    let (impl_g, ty_g, where_g) = input.generics.split_for_impl();
    let root = crate_path(&input.attrs)?;
    let nav = quote!(#root::nav);
    let private = quote!(#nav::__private);

    let mut path_arms = Vec::new();
    let mut parsers = Vec::new();
    for variant in &data.variants {
        let attr = variant
            .attrs
            .iter()
            .find(|a| a.path().is_ident("route"))
            .ok_or_else(|| Error::new(variant.span(), "missing #[route(\"/...\")]"))?;
        let segs = parse_pattern(&attr.parse_args::<LitStr>()?)?;
        let ident = &variant.ident;
        let field_names: Vec<_> = match &variant.fields {
            Fields::Unit => Vec::new(),
            Fields::Named(f) => f.named.iter().filter_map(|f| f.ident.clone()).collect(),
            Fields::Unnamed(_) => {
                return Err(Error::new(
                    variant.span(),
                    "Route variants must be unit or have named fields",
                ));
            }
        };
        for s in &segs {
            if let Segment::Param(p) | Segment::OptParam(p) = s
                && !field_names.contains(p)
            {
                return Err(Error::new(p.span(), format!("no field `{p}` in variant `{ident}`")));
            }
        }
        for f in &field_names {
            let used = segs
                .iter()
                .any(|s| matches!(s, Segment::Param(p) | Segment::OptParam(p) if p == f));
            if !used {
                return Err(Error::new(
                    f.span(),
                    format!("field `{f}` is not part of the route pattern"),
                ));
            }
        }
        let bind = if field_names.is_empty() {
            quote!()
        } else {
            quote!({ #(#field_names),* })
        };

        let pushes = segs.iter().map(|s| match s {
            Segment::Lit(l) => quote!(__p.push('/'); __p.push_str(#l);),
            Segment::Param(p) => quote!(__p.push('/'); __p.push_str(&#private::segment(#p));),
            Segment::OptParam(p) => quote! {
                if let ::core::option::Option::Some(__v) = #p {
                    __p.push('/'); __p.push_str(&#private::segment(__v));
                }
            },
        });
        path_arms.push(quote! {
            Self::#ident #bind => {
                let mut __p = ::std::string::String::new();
                #(#pushes)*
                if __p.is_empty() { __p.push('/'); }
                __p
            }
        });

        parsers.push(parser(ident, &segs, &bind, &nav));
    }

    Ok(quote! {
        impl #impl_g #nav::Route for #name #ty_g #where_g {
            fn path(&self) -> ::std::string::String {
                match self { #(#path_arms)* }
            }
            fn parse(__path: &str) -> ::core::result::Result<Self, #nav::RouteError> {
                let __segs = #private::segments(__path);
                let mut __err: ::core::option::Option<#nav::RouteError> = ::core::option::Option::None;
                #(#parsers)*
                ::core::result::Result::Err(__err.unwrap_or_else(|| #nav::RouteError::NoMatch { path: __path.to_owned() }))
            }
        }

        impl #impl_g ::core::fmt::Display for #name #ty_g #where_g {
            fn fmt(&self, __f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                __f.write_str(&#nav::Route::path(self))
            }
        }

        impl #impl_g ::core::str::FromStr for #name #ty_g #where_g {
            type Err = #nav::RouteError;

            fn from_str(__s: &str) -> ::core::result::Result<Self, Self::Err> {
                <Self as #nav::Route>::parse(__s)
            }
        }
    })
}

/// Typed message ids of a Fluent catalog; documented at `egui_mineral::i18n::messages`.
#[proc_macro]
pub fn messages(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as messages::Input);
    messages::expand(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned};
use syn::{
    Data, DeriveInput, Error, Expr, Fields, GenericArgument, Ident, LitStr, PathArguments, Type, spanned::Spanned,
};

use crate::key_salt::kebab;

#[derive(Default)]
struct TypeAttrs {
    root: Option<syn::Path>,
    checks: Vec<Expr>,
    messages: Option<syn::Path>,
    prefix: Option<LitStr>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Leaf,
    Nested,
    List,
}

struct FieldAttrs {
    label: Option<Expr>,
    supporting: Option<Expr>,
    required: bool,
    checks: Vec<Expr>,
    on: Option<Ident>,
    kind: Option<syn::Path>,
    rename: Option<LitStr>,
    skip: bool,
    shape: Shape,
    // Kind options forwarded to `FieldSpec` methods: `masked` -> `.masked(true)`, `rows = 3` -> `.rows(3)`.
    options: Vec<(Ident, Expr)>,
}

fn type_attrs(attrs: &[syn::Attribute]) -> syn::Result<TypeAttrs> {
    let mut out = TypeAttrs::default();
    for attr in attrs.iter().filter(|a| a.path().is_ident("form")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("crate") {
                out.root = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("check") {
                out.checks.push(meta.value()?.parse()?);
            } else if meta.path.is_ident("messages") {
                out.messages = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("prefix") {
                out.prefix = Some(meta.value()?.parse()?);
            } else {
                return Err(
                    meta.error("expected `crate = path`, `check = path`, `messages = path` or `prefix = \"..\"`")
                );
            }
            Ok(())
        })?;
    }
    Ok(out)
}

fn field_attrs(attrs: &[syn::Attribute], span: Span) -> syn::Result<FieldAttrs> {
    let mut out = FieldAttrs {
        label: None,
        supporting: None,
        required: false,
        checks: Vec::new(),
        on: None,
        kind: None,
        rename: None,
        skip: false,
        shape: Shape::Leaf,
        options: Vec::new(),
    };
    for attr in attrs.iter().filter(|a| a.path().is_ident("form")) {
        attr.parse_nested_meta(|meta| {
            let Some(name) = meta.path.get_ident().cloned() else {
                return Err(meta.error("expected a single identifier"));
            };
            match name.to_string().as_str() {
                "label" => out.label = Some(meta.value()?.parse()?),
                "supporting" => out.supporting = Some(meta.value()?.parse()?),
                "required" => out.required = true,
                "check" => out.checks.push(meta.value()?.parse()?),
                "on" => {
                    let on: Ident = meta.value()?.parse()?;
                    if !["Change", "Blur", "Submit"].contains(&on.to_string().as_str()) {
                        return Err(Error::new(on.span(), "expected `Change`, `Blur` or `Submit`"));
                    }
                    out.on = Some(on);
                }
                "kind" => out.kind = Some(meta.value()?.parse()?),
                "rename" => out.rename = Some(meta.value()?.parse()?),
                "skip" => out.skip = true,
                "nested" => out.shape = Shape::Nested,
                "list" => out.shape = Shape::List,
                _ => {
                    let value = if meta.input.peek(syn::Token![=]) {
                        meta.value()?.parse()?
                    } else {
                        syn::parse_quote_spanned!(name.span()=> true)
                    };
                    out.options.push((name, value));
                }
            }
            Ok(())
        })?;
    }
    if out.shape != Shape::Leaf
        && (out.required || !out.checks.is_empty() || !out.options.is_empty() || out.kind.is_some())
    {
        return Err(Error::new(
            span,
            "`nested` and `list` fields take their rules and options from their own type",
        ));
    }
    Ok(out)
}

// `Vec<Item>` -> `Item`.
fn list_item(ty: &Type) -> Option<&Type> {
    let Type::Path(p) = ty else { return None };
    let last = p.path.segments.last()?;
    if last.ident != "Vec" {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return None;
    };
    match args.args.first()? {
        GenericArgument::Type(t) => Some(t),
        _ => None,
    }
}

// `start_date` -> `Start date`.
fn humanize(name: &str) -> String {
    let words = name.replace('_', " ");
    let mut chars = words.chars();
    chars
        .next()
        .map_or_else(String::new, |c| c.to_uppercase().chain(chars).collect())
}

fn shouty(kebab: &str) -> String {
    kebab.replace('-', "_").to_uppercase()
}

// Generated items, collected field by field.
#[derive(Default)]
struct Out {
    consts: Vec<TokenStream2>,
    // Const names and keys taken so far.
    names: std::collections::HashSet<String>,
    keys: std::collections::HashSet<String>,
    specs: Vec<TokenStream2>,
    checks: Vec<TokenStream2>,
    arms: Vec<TokenStream2>,
    bounds: Vec<TokenStream2>,
}

struct Ctx<'a> {
    form: TokenStream2,
    // Generated bindings, unreachable from user expressions in attributes.
    v: Ident,
    f: Ident,
    s: Ident,
    messages: Option<&'a syn::Path>,
    prefix: String,
}

fn leaf(out: &mut Out, cx: &Ctx<'_>, field: &syn::Field, ident: &Ident, a: &FieldAttrs, key: &str, i: usize) {
    let (form, v, f, s) = (&cx.form, &cx.v, &cx.f, &cx.s);
    let raw = ident.to_string();
    let raw = raw.strip_prefix("r#").unwrap_or(&raw);
    let konst = format_ident!("{}", raw.to_uppercase(), span = ident.span());
    let (vis, fty) = (&field.vis, &field.ty);
    let doc = format!("Form field `{key}`.");
    let spec_fn = format_ident!("__form_spec_{}", raw);
    // A bare name is a core kind; a path names an app kind.
    let kd = match &a.kind {
        Some(k) if k.get_ident().is_some() => quote!(#form::kind::#k),
        Some(k) => quote!(#k),
        None => quote!(<#fty as #form::FieldInput>::Kind),
    };
    let message = cx.messages.map(|m| {
        let item = Ident::new(&shouty(&format!("{}-{}", cx.prefix, kebab(raw))), ident.span());
        quote!(#m::#item)
    });
    let label = match (&a.label, &message) {
        (Some(l), _) => quote!(#form::Label::from(#l)),
        (None, Some(m)) => quote!(#form::Label::from(#m)),
        (None, None) => {
            let text = humanize(raw);
            quote!(#form::Label::from(#text))
        }
    };
    let supporting = match (&a.supporting, &message) {
        (Some(s), _) => quote!(.supporting(#s)),
        (None, Some(m)) => quote!(.supporting(#form::Label::attribute(#m, "supporting"))),
        (None, None) => quote!(),
    };
    let on = a.on.as_ref().map(|on| quote!(.validate_on(#form::ValidateOn::#on)));
    let options = a
        .options
        .iter()
        .map(|(name, value)| quote_spanned!(name.span()=> .#name(#value)));
    out.specs.push(quote! {
        #[doc(hidden)]
        fn #spec_fn(&self) -> #form::FieldSpec<#fty, #kd> {
            #form::FieldSpec::new(#label) #supporting #on #(#options)*
        }
    });
    out.consts.push(quote! {
        #[doc = #doc]
        #vis const #konst: #form::FieldKey<Self, #fty, #kd> =
            #form::FieldKey::new(#key, #i, |#s| &#s.#ident, |#s| &mut #s.#ident, Self::#spec_fn);
    });
    let required = a.required.then(|| quote!(.required()));
    let checks = a.checks.iter().map(|c| quote_spanned!(c.span()=> .check(#c)));
    out.checks.push(quote!(#v.field(Self::#konst) #required #(#checks)*;));
    out.arms.push(quote!(#i => { #f.show(Self::#konst); }));
    out.bounds.push(quote!(__K: #form::FieldWidget<#kd, #fty>));
}

fn field(out: &mut Out, cx: &Ctx<'_>, field: &syn::Field, index: &mut usize) -> syn::Result<()> {
    let Some(ident) = &field.ident else { return Ok(()) };
    let a = field_attrs(&field.attrs, ident.span())?;
    if a.skip {
        return Ok(());
    }
    let (form, v, f, s) = (&cx.form, &cx.v, &cx.f, &cx.s);
    let raw = ident.to_string();
    let raw = raw.strip_prefix("r#").unwrap_or(&raw);
    let key = a.rename.as_ref().map_or_else(|| kebab(raw), LitStr::value);
    let konst = format_ident!("{}", raw.to_uppercase(), span = ident.span());
    if !out.names.insert(konst.to_string()) {
        return Err(Error::new(
            ident.span(),
            format!("another field also generates `{konst}`"),
        ));
    }
    if !out.keys.insert(key.clone()) {
        return Err(Error::new(
            ident.span(),
            format!("another field is also keyed `{key}`; `rename` one"),
        ));
    }
    let (vis, fty) = (&field.vis, &field.ty);
    let doc = format!("Form field `{key}`.");
    let i = *index;
    *index = index.saturating_add(1);
    match a.shape {
        Shape::Leaf => leaf(out, cx, field, ident, &a, &key, i),
        Shape::Nested => {
            out.consts.push(quote! {
                #[doc = #doc]
                #vis const #konst: #form::NestedKey<Self, #fty> =
                    #form::NestedKey::new(#key, #i, |#s| &#s.#ident, |#s| &mut #s.#ident);
            });
            out.checks.push(quote!(#v.nested(Self::#konst);));
            out.arms.push(quote!(#i => #f.nested(Self::#konst, |#f| #f.rest()),));
            out.bounds.push(quote!(#fty: #form::FormFields<__K>));
        }
        Shape::List => {
            let item = list_item(fty).ok_or_else(|| Error::new(fty.span(), "`list` fields must be `Vec<Item>`"))?;
            out.consts.push(quote! {
                #[doc = #doc]
                #vis const #konst: #form::ListKey<Self, #item> =
                    #form::ListKey::new(#key, #i, |#s| &#s.#ident, |#s| &mut #s.#ident);
            });
            out.checks.push(quote!(#v.each(Self::#konst);));
            out.arms.push(quote!(#i => #f.list(Self::#konst, |#f, _| #f.rest()),));
            out.bounds.push(quote!(#item: #form::FormFields<__K>));
        }
    }
    Ok(())
}

pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let ty = &input.ident;
    let Data::Struct(data) = &input.data else {
        return Err(Error::new(ty.span(), "Form can only be derived for structs"));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(Error::new(ty.span(), "Form needs a struct with named fields"));
    };
    let attrs = type_attrs(&input.attrs)?;
    let root = attrs.root.clone().unwrap_or_else(crate::host);
    let cx = Ctx {
        form: quote!(#root::form),
        v: Ident::new("v", Span::mixed_site()),
        f: Ident::new("f", Span::mixed_site()),
        s: Ident::new("s", Span::mixed_site()),
        messages: attrs.messages.as_ref(),
        prefix: attrs
            .prefix
            .as_ref()
            .map_or_else(|| kebab(&ty.to_string()), LitStr::value),
    };
    let mut out = Out::default();
    let mut index = 0usize;
    for f in &fields.named {
        field(&mut out, &cx, f, &mut index)?;
    }
    let Out {
        consts,
        names: _,
        keys: _,
        specs,
        checks,
        arms,
        bounds,
    } = out;
    let (form, v, f) = (&cx.form, &cx.v, &cx.f);
    let type_checks = attrs.checks.iter().map(|c| quote_spanned!(c.span()=> (#c)(self, #v);));
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let mut kit_generics = input.generics.clone();
    kit_generics.params.push(syn::parse_quote!(__K));
    let (kit_impl, _, _) = kit_generics.split_for_impl();
    let predicates = where_clause.map(|w| &w.predicates);
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #ty #ty_generics #where_clause {
            #(#consts)*
            #(#specs)*
        }

        #[automatically_derived]
        impl #impl_generics #form::FormModel for #ty #ty_generics #where_clause {
            fn validate(&self, #v: &mut #form::Validation<'_, Self>) {
                #(#checks)*
                #(#type_checks)*
            }
        }

        #[automatically_derived]
        impl #kit_impl #form::FormFields<__K> for #ty #ty_generics
        where
            #(#bounds,)*
            #predicates
        {
            const FIELDS: usize = #index;

            fn show_index(#f: &mut #form::FormUi<'_, '_, '_, __K, Self>, index: usize) {
                match index {
                    #(#arms)*
                    _ => {}
                }
            }
        }
    })
}

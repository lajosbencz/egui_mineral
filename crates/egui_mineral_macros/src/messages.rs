use std::{collections::BTreeMap, path::PathBuf};

use fluent_syntax::ast::{Entry, Expression, InlineExpression, Message, Pattern, PatternElement};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use syn::{
    Attribute, Error, Ident, LitStr, Token, Visibility,
    ext::IdentExt as _,
    parse::{Parse, ParseStream},
    spanned::Spanned as _,
};

struct Inherited {
    id: String,
    args: Vec<String>,
}

pub(crate) struct Input {
    inherited: Option<Vec<Inherited>>,
    attrs: Vec<Attribute>,
    vis: Visibility,
    module: Ident,
    path: LitStr,
    prefix: Option<LitStr>,
    root: Option<syn::Path>,
    export: Option<Ident>,
    inherit: Option<syn::Path>,
}

impl Parse for Input {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let inherited = if input.peek(Token![@]) {
            input.parse::<Token![@]>()?;
            let list;
            syn::bracketed!(list in input);
            let mut out = Vec::new();
            while !list.is_empty() {
                let id = list.parse::<LitStr>()?.value();
                let names;
                syn::parenthesized!(names in list);
                let mut args = Vec::new();
                while !names.is_empty() {
                    args.push(names.parse::<LitStr>()?.value());
                }
                out.push(Inherited { id, args });
            }
            Some(out)
        } else {
            None
        };
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        input.parse::<Token![mod]>()?;
        let module = input.parse()?;
        input.parse::<Token![=]>()?;
        let mut parsed = Self {
            inherited,
            attrs,
            vis,
            module,
            path: input.parse()?,
            prefix: None,
            root: None,
            export: None,
            inherit: None,
        };
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }
            let key = input.call(Ident::parse_any)?;
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "prefix" => parsed.prefix = Some(input.parse()?),
                "crate" => parsed.root = Some(input.parse()?),
                "export" => parsed.export = Some(input.parse()?),
                "inherit" => parsed.inherit = Some(input.parse()?),
                _ => {
                    return Err(Error::new(
                        key.span(),
                        "expected `prefix`, `crate`, `export` or `inherit`",
                    ));
                }
            }
        }
        Ok(parsed)
    }
}

struct Catalog<'s> {
    messages: BTreeMap<&'s str, &'s Message<&'s str>>,
}

impl<'s> Catalog<'s> {
    fn variables(&self, id: &'s str, pattern: &'s Pattern<&'s str>) -> Vec<&'s str> {
        let mut out = Vec::new();
        self.pattern(pattern, &mut vec![id], &mut out);
        out
    }

    fn pattern(&self, pattern: &'s Pattern<&'s str>, seen: &mut Vec<&'s str>, out: &mut Vec<&'s str>) {
        for element in &pattern.elements {
            if let PatternElement::Placeable { expression } = element {
                self.expression(expression, seen, out);
            }
        }
    }

    fn expression(&self, expression: &'s Expression<&'s str>, seen: &mut Vec<&'s str>, out: &mut Vec<&'s str>) {
        match expression {
            Expression::Inline(inline) => self.inline(inline, seen, out),
            Expression::Select { selector, variants } => {
                self.inline(selector, seen, out);
                for variant in variants {
                    self.pattern(&variant.value, seen, out);
                }
            }
        }
    }

    fn inline(&self, inline: &'s InlineExpression<&'s str>, seen: &mut Vec<&'s str>, out: &mut Vec<&'s str>) {
        match inline {
            InlineExpression::VariableReference { id } => {
                if !out.contains(&id.name) {
                    out.push(id.name);
                }
            }
            InlineExpression::FunctionReference { arguments, .. } => {
                let named = arguments.named.iter().map(|n| &n.value);
                for argument in arguments.positional.iter().chain(named) {
                    self.inline(argument, seen, out);
                }
            }
            InlineExpression::MessageReference { id, attribute } => {
                let Some(message) = self.messages.get(id.name) else {
                    return;
                };
                if seen.contains(&id.name) {
                    return;
                }
                seen.push(id.name);
                let pattern = match attribute {
                    Some(attribute) => message
                        .attributes
                        .iter()
                        .find(|a| a.id.name == attribute.name)
                        .map(|a| &a.value),
                    None => message.value.as_ref(),
                };
                if let Some(pattern) = pattern {
                    self.pattern(pattern, seen, out);
                }
            }
            InlineExpression::Placeable { expression } => self.expression(expression, seen, out),
            // A term sees only the arguments of its call, which are literals.
            InlineExpression::TermReference { .. }
            | InlineExpression::StringLiteral { .. }
            | InlineExpression::NumberLiteral { .. } => {}
        }
    }
}

fn summary(pattern: &Pattern<&str>) -> String {
    let mut text = String::new();
    for element in &pattern.elements {
        match element {
            PatternElement::TextElement { value } => text.push_str(value),
            PatternElement::Placeable {
                expression: Expression::Inline(InlineExpression::VariableReference { id }),
            } => {
                text.push_str("{ $");
                text.push_str(id.name);
                text.push_str(" }");
            }
            PatternElement::Placeable { .. } => text.push_str("{ ... }"),
        }
    }
    text.lines().next().unwrap_or_default().trim().replace('`', "'")
}

fn words(name: &str) -> impl Iterator<Item = &str> {
    name.split(['-', '_']).filter(|w| !w.is_empty())
}

fn pascal(name: &str) -> String {
    words(name)
        .flat_map(|word| {
            let mut chars = word.chars();
            let head = chars.next().map(|c| c.to_ascii_uppercase());
            head.into_iter().chain(chars)
        })
        .collect()
}

fn ident(name: &str, of: &str, at: Span) -> syn::Result<Ident> {
    syn::parse_str::<Ident>(name)
        .or_else(|_| syn::parse_str::<Ident>(&format!("r#{name}")))
        .map(|mut ident| {
            ident.set_span(at);
            ident
        })
        .map_err(|_| Error::new(at, format!("`{of}` does not name a Rust item (`{name}`)")))
}

fn forward(input: &Input, inherit: &syn::Path) -> TokenStream2 {
    let Input {
        attrs,
        vis,
        module,
        path,
        ..
    } = input;
    let prefix = input.prefix.iter();
    let root = input.root.iter();
    let export = input.export.iter();
    quote! {
        #inherit! { #(#attrs)* #vis mod #module = #path #(, prefix = #prefix)* #(, crate = #root)* #(, export = #export)* }
    }
}

struct Scope<'a> {
    at: Span,
    relative: &'a str,
    i18n: TokenStream2,
    vis: TokenStream2,
}

impl Scope<'_> {
    fn error(&self, text: impl std::fmt::Display) -> Error {
        Error::new(self.at, format!("{}: {text}", self.relative))
    }

    fn value<'s>(&self, id: &str, message: &'s Message<&'s str>) -> syn::Result<&'s Pattern<&'s str>> {
        message
            .value
            .as_ref()
            .ok_or_else(|| self.error(format!("`{id}` has no value")))
    }

    fn check_inherited(&self, catalog: &Catalog<'_>, inherited: &[Inherited]) -> syn::Result<()> {
        for Inherited { id, args } in inherited {
            let (id, message) = catalog
                .messages
                .get_key_value(id.as_str())
                .ok_or_else(|| self.error(format!("inherited message `{id}` is missing")))?;
            let found = catalog.variables(id, self.value(id, message)?);
            if let Some(unknown) = found.iter().find(|v| !args.iter().any(|a| a == *v)) {
                return Err(self.error(format!(
                    "`{id}` uses `${unknown}`, which the inherited message does not take"
                )));
            }
        }
        Ok(())
    }

    fn item(&self, id: &str, item: &Ident, pattern: &Pattern<&str>, args: &[&str]) -> syn::Result<TokenStream2> {
        let Self { at, i18n, vis, .. } = self;
        let doc = format!("`{id}`: `{}`", summary(pattern));
        if args.is_empty() {
            return Ok(quote_spanned! {*at=>
                #[doc = #doc]
                #vis const #item: #i18n::Message<'static, 0> = #i18n::__private::message(#id, []);
            });
        }
        let fields = args
            .iter()
            .map(|arg| ident(&arg.replace('-', "_"), id, *at))
            .collect::<syn::Result<Vec<_>>>()?;
        let docs = args.iter().map(|arg| format!("`${arg}`"));
        let count = args.len();
        Ok(quote_spanned! {*at=>
            #[doc = #doc]
            #[derive(Clone, Debug)]
            #vis struct #item<'a> {
                #(#[doc = #docs] #vis #fields: #i18n::Arg<'a>,)*
            }

            impl<'a> #i18n::IntoMessage<'a, #count> for #item<'a> {
                fn into_message(self) -> #i18n::Message<'a, #count> {
                    #i18n::__private::message(#id, [#((#args, self.#fields)),*])
                }
            }
        })
    }

    fn tail(&self, export: Option<&Ident>, exported: &[TokenStream2]) -> TokenStream2 {
        let Self { relative, vis, .. } = self;
        let tracked = quote!(::core::include_str!(
            ::core::concat!(::core::env!("CARGO_MANIFEST_DIR"), "/", #relative)
        ));
        let Some(export) = export else {
            return quote! {
                /// FTL source of the catalog the messages come from.
                #vis const SOURCE: &str = #tracked;
            };
        };
        quote! {
            const _: &str = #tracked;

            /// Message ids a catalog must cover: `messages!(mod msg = "locales/en-US.ftl", inherit = <this macro>)`.
            #[macro_export]
            macro_rules! #export {
                ($($input:tt)*) => {
                    $crate::i18n::messages! { @[#(#exported)*] $($input)* }
                };
            }
        }
    }
}

fn item_visibility(vis: &Visibility, at: Span) -> syn::Result<TokenStream2> {
    match vis {
        Visibility::Inherited => Ok(quote_spanned!(at=> pub(super))),
        Visibility::Restricted(r) if r.path.is_ident("self") || r.path.is_ident("super") => Err(Error::new(
            r.path.span(),
            "expected no visibility, `pub(crate)`, `pub(in crate::..)` or `pub`",
        )),
        vis => Ok(quote!(#vis)),
    }
}

pub(crate) fn expand(input: &Input) -> syn::Result<TokenStream2> {
    let at = input.path.span();
    let relative = input.path.value();
    if let (Some(inherit), None) = (&input.inherit, &input.inherited) {
        return Ok(forward(input, inherit));
    }
    let root = input.root.as_ref().map_or_else(
        || {
            let host = crate::host();
            quote_spanned!(at=> #host)
        },
        |root| quote!(#root),
    );
    let scope = Scope {
        at,
        relative: &relative,
        i18n: quote_spanned!(at=> #root::i18n),
        vis: item_visibility(&input.vis, at)?,
    };
    let manifest = std::env::var("CARGO_MANIFEST_DIR").map_err(|e| scope.error(format!("CARGO_MANIFEST_DIR: {e}")))?;
    let source = std::fs::read_to_string(PathBuf::from(manifest).join(&relative)).map_err(|e| scope.error(e))?;
    let resource = fluent_syntax::parser::parse(source.as_str()).map_err(|(_, errors)| {
        let text = errors
            .iter()
            .map(|e| {
                let line = source.get(..e.pos.start).map_or(0, |head| head.lines().count().max(1));
                format!("line {line}: {e}")
            })
            .collect::<Vec<_>>()
            .join("; ");
        scope.error(text)
    })?;

    let mut catalog = Catalog {
        messages: BTreeMap::new(),
    };
    for entry in &resource.body {
        if let Entry::Message(message) = entry
            && catalog.messages.insert(message.id.name, message).is_some()
        {
            return Err(scope.error(format!("`{}` is defined twice", message.id.name)));
        }
    }
    let inherited = input.inherited.as_deref().unwrap_or_default();
    scope.check_inherited(&catalog, inherited)?;

    let prefix = input.prefix.as_ref().map_or_else(String::new, LitStr::value);
    let mut items = Vec::new();
    let mut exported = Vec::new();
    let mut names = BTreeMap::new();
    for (id, message) in &catalog.messages {
        let id = *id;
        if inherited.iter().any(|i| i.id == id) {
            continue;
        }
        let name = id
            .strip_prefix(prefix.as_str())
            .ok_or_else(|| scope.error(format!("`{id}` lacks the `{prefix}` prefix")))?;
        let pattern = scope.value(id, message)?;
        let args = catalog.variables(id, pattern);
        let item_name = if args.is_empty() {
            words(name).map(str::to_ascii_uppercase).collect::<Vec<_>>().join("_")
        } else {
            pascal(name)
        };
        let item = ident(&item_name, id, at)?;
        if let Some(other) = names.insert(item.to_string(), id) {
            return Err(scope.error(format!("`{id}` and `{other}` both map to `{item}`")));
        }
        items.push(scope.item(id, &item, pattern, &args)?);
        exported.push(quote!(#id (#(#args)*)));
    }

    let tail = scope.tail(input.export.as_ref(), &exported);
    let Input {
        attrs,
        vis: module_vis,
        module,
        ..
    } = input;
    let doc = attrs
        .is_empty()
        .then(|| format!("Messages of `{relative}`."))
        .into_iter();
    Ok(quote! {
        #(#attrs)*
        #(#[doc = #doc])*
        #module_vis mod #module {
            #(#items)*
            #tail
        }
    })
}

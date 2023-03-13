use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::spanned::Spanned;

#[proc_macro_derive(MessageExt, attributes(prost_extra))]
pub fn reflect_message(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match reflect_message_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

struct Args {
    args_span: Span,
    message_name: Option<syn::Lit>,
}

fn reflect_message_impl(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
    match &input.data {
        syn::Data::Struct(_) => (),
        syn::Data::Enum(_) => return Ok(Default::default()),
        syn::Data::Union(_) => return Ok(Default::default()),
    };

    let args = Args::parse(input.ident.span(), &input.attrs)?;

    let name = &input.ident;
    let message_name = args.message_name()?;

    Ok(quote! {
        impl ::prost_extra::MessageExt for #name {
            fn full_name() -> &'static str {
                #message_name
            }
        }
    })
}

fn is_prost_descriptor_attribute(attr: &syn::Attribute) -> bool {
    attr.path.is_ident("prost_extra")
}

impl Args {
    fn parse(input_span: Span, attrs: &[syn::Attribute]) -> Result<Args, syn::Error> {
        let reflect_attrs: Vec<_> = attrs
            .iter()
            .filter(|attr| is_prost_descriptor_attribute(attr))
            .collect();

        if reflect_attrs.is_empty() {
            return Err(syn::Error::new(
                input_span,
                "missing #[prost_extra] attribute",
            ));
        }

        let mut span: Option<Span> = None;
        let mut nested = Vec::new();
        for attr in reflect_attrs {
            span = match span {
                Some(span) => span.join(attr.span()),
                None => Some(attr.span()),
            };
            match attr.parse_meta()? {
                syn::Meta::List(list) => nested.extend(list.nested),
                meta => return Err(syn::Error::new(meta.span(), "expected list of attributes")),
            }
        }

        let mut args = Args {
            args_span: span.unwrap_or_else(Span::call_site),
            message_name: None,
        };
        for item in nested {
            match item {
                syn::NestedMeta::Meta(syn::Meta::NameValue(value)) => {
                    if value.path.is_ident("message_name") {
                        args.message_name = Some(value.lit);
                    } else {
                        return Err(syn::Error::new(
                            value.span(),
                            "unknown argument (expected 'message_name')",
                        ));
                    }
                }
                _ => return Err(syn::Error::new(item.span(), "unexpected attribute")),
            }
        }

        Ok(args)
    }

    fn message_name(&self) -> Result<proc_macro2::TokenStream, syn::Error> {
        if let Some(message_name) = &self.message_name {
            Ok(message_name.to_token_stream())
        } else {
            Err(syn::Error::new(
                self.args_span,
                "missing required argument 'message_name'",
            ))
        }
    }
}

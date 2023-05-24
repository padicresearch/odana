use std::collections::HashSet;

use super::{Attributes, Method, Service};
use crate::{
    format_method_name, format_method_path, format_service_name, generate_doc_comment,
    generate_doc_comments, naive_snake_case,
};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Ident, Lit, LitByteStr};

/// Generate service for Server.
///
/// This takes some `Service` and will generate a `TokenStream` that contains
/// a public module containing the server service and handler trait.
#[deprecated(since = "0.8.3", note = "Use CodeGenBuilder::generate_server")]
pub fn generate<T: Service>(
    service: &T,
    emit_package: bool,
    proto_path: &str,
    compile_well_known_types: bool,
    attributes: &Attributes,
) -> TokenStream {
    generate_internal(
        service,
        emit_package,
        proto_path,
        compile_well_known_types,
        attributes,
        &HashSet::default(),
    )
}

pub(crate) fn generate_internal<T: Service>(
    service: &T,
    emit_package: bool,
    proto_path: &str,
    compile_well_known_types: bool,
    attributes: &Attributes,
    disable_comments: &HashSet<String>,
) -> TokenStream {
    let methods = generate_methods(service, emit_package);

    let server_service = quote::format_ident!("{}Instance", service.name());
    let server_trait = quote::format_ident!("{}Service", service.name());
    let server_mod = quote::format_ident!("{}", naive_snake_case(service.name()));
    let generated_trait = generate_trait(
        service,
        emit_package,
        proto_path,
        compile_well_known_types,
        server_trait.clone(),
        disable_comments,
    );
    let package = if emit_package { service.package() } else { "" };
    // Transport based implementations
    let service_name = format_service_name(service, emit_package);

    let service_doc = if disable_comments.contains(&service_name) {
        TokenStream::new()
    } else {
        generate_doc_comments(service.comment())
    };

    let named = generate_named(&server_service, &server_trait, &service_name);
    let mod_attributes = attributes.for_mod(package);
    let struct_attributes = attributes.for_struct(&service_name);
    quote! {
        /// Generated server implementations.
        #(#mod_attributes)*
        pub mod #server_mod {
            #![allow(
                unused_variables,
                dead_code,
                missing_docs,
                clippy::new_without_default,
                clippy::needless_return,
                // will trigger if compression is disabled
                clippy::let_unit_value,
            )]
            use rune_framework::prelude::*;
            use rune_std::marker::PhantomData;

            #generated_trait

            #service_doc
            #(#struct_attributes)*
            #[derive(Debug)]
            pub struct #server_service<T: #server_trait> {
                inner: PhantomData<T>,
            }

            impl<T: #server_trait> #server_service<T> {
                pub fn new() -> Self {
                    Self {
                        inner : PhantomData::default()
                    }
                }
            }

            impl<T> Service for #server_service<T>
                where
                    T: #server_trait,
            {
                fn call(&self,method: u64, payload: &[u8]) -> CallResponse {
                    #methods
                    return CallResponse::default();
                }
            }

            #named
        }
    }
}

fn generate_trait<T: Service>(
    service: &T,
    emit_package: bool,
    proto_path: &str,
    compile_well_known_types: bool,
    server_trait: Ident,
    disable_comments: &HashSet<String>,
) -> TokenStream {
    let methods = generate_trait_methods(
        service,
        emit_package,
        proto_path,
        compile_well_known_types,
        disable_comments,
    );
    let trait_doc = generate_doc_comment(format!(
        " Generated trait containing gRPC methods that should be implemented for use with {}Server.",
        service.name()
    ));

    quote! {
        #trait_doc
        pub trait #server_trait : Send + Sync + 'static {
            #methods
        }
    }
}

fn generate_trait_methods<T: Service>(
    service: &T,
    emit_package: bool,
    proto_path: &str,
    compile_well_known_types: bool,
    disable_comments: &HashSet<String>,
) -> TokenStream {
    let mut stream = TokenStream::new();

    for method in service.methods() {
        let name = quote::format_ident!("{}", method.name());

        let (req_message, res_message) =
            method.request_response_name(proto_path, compile_well_known_types);

        let method_doc =
            if disable_comments.contains(&format_method_name(service, method, emit_package)) {
                TokenStream::new()
            } else {
                generate_doc_comments(method.comment())
            };

        let method = match (method.client_streaming(), method.server_streaming()) {
            (false, false) => {
                quote! {
                    #method_doc
                    fn #name(call: Call<#req_message>) -> ::anyhow::Result<#res_message>;
                }
            }
            (true, false) => {
                unimplemented!()
            }
            (false, true) => {
                unimplemented!()
            }
            (true, true) => {
                unimplemented!()
            }
        };

        stream.extend(method);
    }

    stream
}

fn generate_named(
    server_service: &syn::Ident,
    server_trait: &syn::Ident,
    service_name: &str,
) -> TokenStream {
    let service_name = syn::LitStr::new(service_name, proc_macro2::Span::call_site());

    quote! {
        impl<T: #server_trait> rune_framework::NamedService for #server_service<T> {
            const NAME: &'static str = #service_name;
        }
    }
}

fn generate_methods<T: Service>(service: &T, emit_package: bool) -> TokenStream {
    let mut stream = TokenStream::new();

    for method in service.methods() {
        let path = format_method_path(service, method, emit_package);
        let method_path = Lit::ByteStr(LitByteStr::new(path.as_bytes(), Span::call_site()));
        let ident = quote::format_ident!("{}", method.name());

        let method_stream = match (method.client_streaming(), method.server_streaming()) {
            (false, false) => generate_unary(ident),
            _ => unimplemented!(),
        };

        let method = quote! {
            if Hashing::twox_64_hash(#method_path) == method {
                 #method_stream
            }
        };
        stream.extend(method);
    }

    stream
}

fn generate_unary(method_ident: Ident) -> TokenStream {
    quote! {
        return T::#method_ident(Call::new(payload).unwrap()).map(|response| {
            CallResponse::from(response)
        }).unwrap_or_default()
    }
}

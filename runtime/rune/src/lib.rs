extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, GenericArgument, Ident, Item, ItemType, Path, PathArguments, PathSegment,
    Type, TypePath,
};

fn get_module_path(visibility: &syn::Visibility) -> String {
    match visibility {
        syn::Visibility::Public(_) => "".to_owned(),
        syn::Visibility::Crate(_) => "crate".to_owned(),
        syn::Visibility::Restricted(restricted) => {
            let path = &restricted.path;
            quote!(#path).to_string()
        }
        syn::Visibility::Inherited => String::new(),
    }
}

#[proc_macro_attribute]
pub fn storage_map(_attr: TokenStream, item: TokenStream) -> TokenStream {
    if let Item::Type(type_item) = parse_macro_input!(item as Item) {
        let ident = &type_item.ident;
        let prefix = format!("{}::{}", get_module_path(&type_item.vis), ident);

        // Generate a new struct name for the StorageKeyPrefix
        let storage_key_prefix_ident = format_ident!("{}StorageKeyPrefix", ident);

        //Extract the generics from the original type
        let (replace_first_generic, generics) =
            extract_generics(&type_item, &storage_key_prefix_ident);

        // Only generate the new struct and the redefinition if the first generic is an underscore
        if replace_first_generic {
            let output = quote! {
                pub struct #storage_key_prefix_ident;

                impl StorageKeyPrefix for #storage_key_prefix_ident {
                    fn key_prefix() -> &'static [u8] {
                        #prefix.as_bytes()
                    }
                }

                pub type #ident = ::rune_framework::prelude::StorageMap<#(#generics),*>;
            };

            output.into()
        } else {
            // If the first generic is not an underscore, just return the input
            TokenStream::from(quote! {
                #type_item
            })
        }
    } else {
        panic!("This macro can only be used with type alias");
    }
}

#[proc_macro_attribute]
pub fn storage_value(_attr: TokenStream, item: TokenStream) -> TokenStream {
    if let Item::Type(type_item) = parse_macro_input!(item as Item) {
        let ident = &type_item.ident;
        let prefix = format!("{}::{}", get_module_path(&type_item.vis), ident);

        // Generate a new struct name for the StorageKeyPrefix
        let storage_key_prefix_ident = format_ident!("{}StorageKeyPrefix", ident);

        //Extract the generics from the original type
        let (replace_first_generic, generics) =
            extract_generics(&type_item, &storage_key_prefix_ident);

        // Only generate the new struct and the redefinition if the first generic is an underscore
        if replace_first_generic {
            let output = quote! {
                pub struct #storage_key_prefix_ident;

                impl StorageKeyPrefix for #storage_key_prefix_ident {
                    fn key_prefix() -> &'static [u8] {
                        #prefix.as_bytes()
                    }
                }

                pub type #ident = ::rune_framework::prelude::StorageValue<#(#generics),*>;
            };

            output.into()
        } else {
            // If the first generic is not an underscore, just return the input
            TokenStream::from(quote! {
                #type_item
            })
        }
    } else {
        panic!("This macro can only be used with type alias");
    }
}

fn extract_generics(
    type_item: &ItemType,
    storage_key_prefix_ident: &Ident,
) -> (bool, Vec<GenericArgument>) {
    let (replace_first_generic, generics) = if let Type::Path(TypePath { path, .. }) =
        &*type_item.ty
    {
        if let Some(seg) = path.segments.last() {
            if let PathArguments::AngleBracketed(ref args) = seg.arguments {
                let mut generics = args.args.iter().cloned().collect::<Vec<_>>();
                let replace_first_generic =
                    matches!(generics.get(0), Some(GenericArgument::Type(Type::Infer(_))));
                if replace_first_generic {
                    let path = Path {
                        leading_colon: None,
                        segments: vec![PathSegment {
                            ident: storage_key_prefix_ident.clone(),
                            arguments: PathArguments::None,
                        }]
                        .into_iter()
                        .collect(),
                    };
                    generics[0] = GenericArgument::Type(Type::Path(TypePath { qself: None, path }));
                }
                (replace_first_generic, generics)
            } else {
                (false, vec![])
            }
        } else {
            (false, vec![])
        }
    } else {
        (false, vec![])
    };
    (replace_first_generic, generics)
}

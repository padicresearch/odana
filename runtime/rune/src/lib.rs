extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Attribute, AttributeArgs, DeriveInput, Lit, Meta, NestedMeta, Path};

#[proc_macro_attribute]
pub fn rune(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input module
    let input = parse_macro_input!(item as syn::ItemMod);
    let mod_name = input.ident.clone();
    let mut app_name = None;

    // Get the module name and contents
    // let mod_name = input.ident;
    let content = input.content.as_ref().unwrap().1.as_slice();

    // Get all the structs in the module with the #[route(with = ...)] attribute
    let mut services = Vec::new();
    for item in content {
        if let syn::Item::Struct(item_struct) = item {
            if let Some(route_attr) = get_route_attr(&item_struct.attrs) {
                // Get the struct name and #[route(with = ...)] attribute value
                let struct_name = item_struct.ident.clone();
                let with_value = get_with_value(route_attr);
                services.push(quote! {
                    r.register_service(#with_value::<#mod_name::#struct_name>::new());
                });
            }
            if let Some(_) = get_app_attr(&item_struct.attrs) {
                // Get the struct name and #[route(with = ...)] attribute value
                if app_name.is_some() {
                    panic!("there can only be one #[rune]");
                }
                app_name = Some(item_struct.ident.clone());
            }
        }
    }

    let app_name = app_name.expect("#[rune] not found");

    // Generate the code for the router function
    let router_fn = quote! {
        fn router() -> &'static Router {
            static INSTANCE: OnceCell<Router> = OnceCell::new();
            INSTANCE.get_or_init(|| {
                let mut r = Router::new();
                #(#services)*
                r
            })
        }
    };

    // Combine the router function with the original module content
    let output = quote! {
        #input

        #router_fn

        use crate::#mod_name::#app_name;

        impl RuntimeApplication for #app_name {
            fn call(context: Context, arg: &[u8]) -> anyhow::Result<CallResponse> {
                Ok(router().handle(context,arg))
            }
            fn descriptor() -> &'static [u8] {
                include_bytes!(concat!(env!("OUT_DIR"), "/file_descriptor_set.bin"))
            }
        }

        pub use ::rune_framework::export_app;
        export_app!(#app_name);
    };

    output.into()
}

fn get_route_attr(attrs: &[Attribute]) -> Option<&Attribute> {
    for attr in attrs {
        if let Ok(meta) = attr.parse_meta() {
            if is_ident(meta.path(), "route") {
                return Some(attr);
            }
        }
    }
    None
}

fn get_app_attr(attrs: &[Attribute]) -> Option<&Attribute> {
    for attr in attrs {
        if let Ok(meta) = attr.parse_meta() {
            if is_ident(meta.path(), "app") {
                return Some(attr);
            }
        }
    }
    None
}

fn get_ident(path: &Path) -> Option<&Ident> {
    if path.leading_colon.is_none()
        && path.segments.len() == 1
        && path.segments[0].arguments.is_none()
    {
        Some(&path.segments[0].ident)
    } else if path.leading_colon.is_none()
        && path.segments.len() == 2
        && path.segments[1].arguments.is_none()
    {
        Some(&path.segments[1].ident)
    } else {
        None
    }
}

fn is_ident<I: ?Sized>(path: &Path, ident: &I) -> bool
where
    Ident: PartialEq<I>,
{
    match get_ident(path) {
        Some(id) => id == ident,
        None => false,
    }
}

fn get_with_value(attr: &Attribute) -> Path {
    let meta = attr.parse_meta().unwrap();
    let meta_list = match meta {
        Meta::List(ml) => ml,
        _ => panic!("#[rune::route(with = ...)] attribute must have a list form"),
    };
    let with_value = meta_list.nested.iter().find_map(|nested_meta| {
        if let NestedMeta::Meta(Meta::NameValue(nv)) = nested_meta {
            if nv.path.is_ident("with") {
                if let Lit::Str(lit_str) = &nv.lit {
                    Some(lit_str.value())
                } else {
                    panic!("#[rune::route(with = ...)] attribute value must be a string literal")
                }
            } else {
                None
            }
        } else {
            None
        }
    });
    if let Some(value) = with_value {
        syn::parse_str::<Path>(&value).unwrap()
    } else {
        panic!("#[rune::route(with = ...)] attribute missing 'with' argument")
    }
}

#[proc_macro_attribute]
pub fn route(_: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn app(_: TokenStream, item: TokenStream) -> TokenStream {
    item
}

fn generate_trait_impl(
    module_path: &str,
    struct_name: &Ident,
    hasher_type: &Ident,
    key_type: &Ident,
    value_type: &Ident,
) -> proc_macro2::TokenStream {
    let prefix = format!(
        "{}::{}::{}::{}::{}",
        module_path, struct_name, hasher_type, key_type, value_type
    );

    quote! {
        impl ::rune_framework::prelude::StorageMap<#hasher_type,  #key_type,  #value_type> for #struct_name
        {
            fn storage_prefix() -> &'static [u8] {
                #prefix.as_bytes()
            }
        }
    }
}

// Extract the type parameters and attribute values from the input struct
fn extract_type_params_and_attrs(
    input: &DeriveInput,
    attr_args: &AttributeArgs,
) -> (Ident, Ident, Ident) {
    let mut hasher_type = None;
    let mut key_type = None;
    let mut value_type = None;

    for arg in attr_args.iter() {
        if let NestedMeta::Meta(Meta::NameValue(name_value)) = arg {
            match name_value.path.get_ident().map(|id| id.to_string()) {
                Some(s) if s == "storage_key_hasher" => {
                    if let Lit::Str(lit) = &name_value.lit {
                        if let Ok(ident) = syn::parse_str::<Ident>(&lit.value()) {
                            hasher_type = Some(ident);
                        }
                    }
                }
                Some(s) if s == "key_type" => {
                    if let Lit::Str(lit) = &name_value.lit {
                        if let Ok(ident) = syn::parse_str::<Ident>(&lit.value()) {
                            key_type = Some(ident);
                        }
                    }
                }
                Some(s) if s == "value_type" => {
                    if let Lit::Str(lit) = &name_value.lit {
                        if let Ok(ident) = syn::parse_str::<Ident>(&lit.value()) {
                            value_type = Some(ident);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    (
        hasher_type.unwrap_or_else(|| Ident::new("Twox128Hasher", input.span())),
        key_type.unwrap_or_else(|| Ident::new("Vec<u8>", input.span())),
        value_type.unwrap_or_else(|| Ident::new("Vec<u8>", input.span())),
    )
}

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
pub fn storage_map(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the attribute arguments into a syntax tree representation
    let attr_args = parse_macro_input!(attr as AttributeArgs);

    // Parse the input tokens into a syntax tree representation
    let input = parse_macro_input!(item as DeriveInput);

    // Get the name of the struct being derived for
    let struct_name = &input.ident;

    // Extract the type parameters and attribute values
    let (hasher_type, key_type, value_type) = extract_type_params_and_attrs(&input, &attr_args);
    let module_path = get_module_path(&input.vis);
    // Generate the trait implementation
    let trait_impl = generate_trait_impl(
        &module_path,
        struct_name,
        &hasher_type,
        &key_type,
        &value_type,
    );
    // Return the generated impl as a TokenStream
    let output = quote! {
        #input

        #trait_impl
    };

    output.into()
}

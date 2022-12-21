use proc_macro::TokenStream;
use quote::quote;

use crate::common;

pub fn impl_transition_input_tokens_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let unpack =
        common::struct_field_names_types(&ast)
            .iter()
            .fold(quote! {}, |acc, (field, ty)| {
                let field_str = field.to_string();
                quote! {
                    #acc
                    #field: *map.remove_entry(
                        &(#field_str.to_string(), ::std::any::TypeId::of::<#ty>())
                    ).unwrap().1.downcast::<#ty>().unwrap(),
                }
            });

    let field_descriptions = common::field_descriptions_hash_set(&ast);

    let gen = quote! {
        impl ::ntpnet_lib::transition_input_tokens::TransitionInputTokens for #name {
            fn from_map(map: &mut ::std::collections::HashMap<(String, ::std::any::TypeId), ::ntpnet_lib::Token>)
                -> Self
            {
                Self{
                    #unpack
                }
            }
            fn in_edges() -> ::std::collections::HashSet<(String, ::std::any::TypeId)> {
                #field_descriptions
            }
        }
    };
    gen.into()
}

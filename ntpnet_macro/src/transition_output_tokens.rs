use proc_macro::TokenStream;
use quote::quote;

use crate::common;

pub fn impl_transition_output_tokens_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let pack = common::struct_field_names_types(&ast).iter().fold(quote!{},
        |acc, (field, ty)| {
            let field_str = field.to_string();
            quote!{
                #acc
                map.insert((#field_str.to_string(), ::std::any::TypeId::of::<#ty>()), Box::new(self.#field));
            }
        }
    );

    let field_descriptions = common::field_descriptions_hash_set(&ast);

    let gen = quote! {
        impl ::ntpnet_lib::transition_output_tokens::TransitionOutputTokens for #name {
            fn into_map(self: Self,
            map: &mut ::std::collections::HashMap<(String, ::std::any::TypeId), ::ntpnet_lib::Token>)
            {
                #pack
            }
            fn out_edges() -> ::std::collections::HashSet<(String, ::std::any::TypeId)> {
                #field_descriptions
            }
        }
    };
    gen.into()
}

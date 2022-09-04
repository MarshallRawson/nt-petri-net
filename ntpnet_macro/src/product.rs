use proc_macro::TokenStream;
use quote::quote;

use crate::common;

pub fn impl_product_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let pack = common::struct_field_names(&ast).iter().fold(quote!{},
        |acc, field| {
            quote!{
                #acc
                map.insert("#field".to_string(), Box::new(self.#field)).unwrap();
            }
        }
    );

    let field_descriptions = common::field_descriptions_hash_set(&ast);

    let gen = quote! {
        impl ::ntpnet_lib::product::Product for #name {
            fn into_map(self: Self,
            map: &mut ::std::collections::HashMap<String, ::ntpnet_lib::Token>)
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

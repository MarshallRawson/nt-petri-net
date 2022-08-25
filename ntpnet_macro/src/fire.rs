use proc_macro::TokenStream;
use quote::quote;

use crate::common;

pub fn impl_fire_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let unpack = common::struct_field_names(&ast).iter().fold(quote!{},
        |acc, field| {
            quote!{
                #acc
                #field: *map.remove_entry(
                    &"#field".to_string()
                ).unwrap().1.downcast::<_>().unwrap(),
            }
        }
    );

    let field_descriptions = common::field_descriptions_HashSet(&ast);

    let gen = quote! {
        impl ::ntpnet_lib::fire::Fire for #name {
            fn from_map(map: &mut ::std::collections::HashMap<String, ::ntpnet_lib::Token>)
                -> Self
            {
                Self{
                    #unpack
                }
            }
            fn edges() -> ::std::collections::HashSet<(String, ::std::any::TypeId)> {
                #field_descriptions
            }
        }
    };
    gen.into()
}

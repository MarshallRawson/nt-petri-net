use proc_macro::TokenStream;
use proc_macro2;
use quote::quote;
use quote::ToTokens;
use syn;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Result, Token, Type};


pub fn impl_fire_macro(ast: &syn::DeriveInput) -> TokenStream {
    println!("fire: {:#?}", ast);
    let gen = quote! {};
    gen.into()
}

use proc_macro::TokenStream;
use proc_macro2;
use quote::quote;
use quote::ToTokens;
use syn;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Result, Token, Type};


pub fn impl_product_macro(ast: &syn::DeriveInput) -> TokenStream {
    println!("product: {:#?}", ast);
    let gen = quote! {};
    gen.into()
}

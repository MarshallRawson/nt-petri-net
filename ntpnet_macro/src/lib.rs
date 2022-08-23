use proc_macro::TokenStream;
use syn;

mod fire;
#[proc_macro_derive(Fire)]
pub fn fire_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    crate::fire::impl_fire_macro(&ast)
}

mod product;
#[proc_macro_derive(Product)]
pub fn product_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    crate::product::impl_product_macro(&ast)
}

mod transition;
#[proc_macro_derive(Transition, attributes(ntpnet_in_edges, ntpnet_out_edges, ntpnet_transitions))]
pub fn transition_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    crate::transition::impl_transition_macro(&ast)
}




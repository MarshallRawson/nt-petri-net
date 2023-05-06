use proc_macro::TokenStream;
use syn;

mod transition_input_tokens_macro;
#[proc_macro_derive(TransitionInputTokensMacro)]
pub fn transition_input_tokens_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    crate::transition_input_tokens_macro::impl_transition_input_tokens_macro(&ast)
}

mod transition_output_tokens_macro;
#[proc_macro_derive(TransitionOutputTokensMacro)]
pub fn transition_output_tokens_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    crate::transition_output_tokens_macro::impl_transition_output_tokens_macro(&ast)
}

mod transition_macro;
#[proc_macro_derive(Transition, attributes(ntpnet_transition))]
pub fn transition_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    crate::transition_macro::impl_transition_macro(&ast)
}

mod common;

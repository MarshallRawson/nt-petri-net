use proc_macro::TokenStream;
use proc_macro2::TokenTree::Group;
use proc_macro2::{Delimiter, Ident, TokenTree};
use quote::quote;
use std::collections::HashSet;

pub fn impl_transition_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let lt_token = if let Some(_) = &ast.generics.lt_token {
        quote! {< '_ >}
    } else {
        quote! {}
    };
    //#[derive(Debug)]
    struct TransitionCallback {
        name: Ident,
        input: (Ident, Vec<Ident>),
        output: (Ident, Vec<Ident>),
    }
    let token_callbacks = get_attr(ast, "ntpnet_transition")
        .iter()
        .map(|ts| {
            let mut vt = ts.clone().into_iter().collect::<Vec<_>>();
            let name = match vt.remove(0) {
                TokenTree::Ident(i) => i,
                _ => unimplemented!(),
            };
            match vt.remove(0) {
                TokenTree::Punct(p) => assert!(p.as_char() == ':'),
                _ => unimplemented!(),
            }
            let input = pop_interface(&mut vt);
            match vt.remove(0) {
                TokenTree::Punct(p) => assert!(p.as_char() == '-'),
                _ => unimplemented!(),
            }
            match vt.remove(0) {
                TokenTree::Punct(p) => assert!(p.as_char() == '>'),
                _ => unimplemented!(),
            };
            let output = pop_interface(&mut vt);
            TransitionCallback {
                name: name,
                input: input,
                output: output,
            }
        })
        .collect::<Vec<TransitionCallback>>();
    let interface_enums = token_callbacks
        .iter()
        .fold(vec![], |mut acc, tc| {
            acc.push(tc.input.clone());
            acc.push(tc.output.clone());
            acc
        })
        .iter()
        .fold(quote! {}, |acc, (name, enums)| {
            let enum_fields = enums.iter().fold(quote! {}, |acc, e| quote! {#acc #e(#e),});
            quote! {#acc enum #name {#enum_fields}}
        });
    let in_edges = token_callbacks
        .iter()
        .fold(HashSet::new(), |mut acc, tc| {
            for e in &tc.input.1 {
                acc.insert(e.clone());
            }
            acc
        })
        .into_iter()
        .collect::<Vec<_>>();
    let in_edges = {
        let enum_first = &in_edges[0];
        if in_edges.len() > 1 {
            let in_edges = in_edges[1..].iter().fold(
                quote!{<#enum_first as ::ntpnet_lib::transition_input_tokens::TransitionInputTokens>::in_edges() },
                |acc, e| quote!{(#acc .union(&<#e as ::ntpnet_lib::transition_input_tokens::TransitionInputTokens>::in_edges())
                    .cloned().collect::<::std::collections::HashSet<_>>())}
            );
            quote! { #in_edges }
        } else {
            quote! {<#enum_first as ::ntpnet_lib::transition_input_tokens::TransitionInputTokens>::in_edges() }
        }
    };
    let out_edges = token_callbacks
        .iter()
        .fold(HashSet::new(), |mut acc, tc| {
            for e in &tc.output.1 {
                acc.insert(e.clone());
            }
            acc
        })
        .into_iter()
        .collect::<Vec<_>>();
    let out_edges = {
        let enum_first = &out_edges[0];
        if out_edges.len() > 1 {
            let out_edges = out_edges[1..].iter().fold(
                quote!{<#enum_first as ::ntpnet_lib::transition_output_tokens::TransitionOutputTokens>::out_edges() },
                |acc, e| quote!{(#acc .union(&<#e as ::ntpnet_lib::transition_output_tokens::TransitionOutputTokens>::out_edges())
                    .cloned().collect::<::std::collections::HashSet<_>>())}
            );
            quote! { #out_edges }
        } else {
            quote! {<#enum_first as ::ntpnet_lib::transition_output_tokens::TransitionOutputTokens>::out_edges() }
        }
    };
    let cases = token_callbacks.iter().fold(quote!{},
        |acc, tc| {
            let inputs = tc.input.1.iter().fold(quote!{},
                |acc_cond, e| {
                    quote!{#acc_cond <#e as ::ntpnet_lib::transition_input_tokens::TransitionInputTokens>::in_edges(),}
                }
            );
            let inputs = quote!{vec![#inputs]};
            let outputs = tc.output.1.iter().fold(quote!{},
                |acc_prod, e| {
                    quote!{#acc_prod <#e as ::ntpnet_lib::transition_output_tokens::TransitionOutputTokens>::out_edges(),}
                }
            );
            let outputs = quote!{vec![#outputs]};
            let name_str = tc.name.to_string();
            quote!{#acc
                (#name_str.into(), ::ntpnet_lib::transition::Case {
                    inputs: #inputs,
                    outputs: #outputs,
                }),
            }
        }
    );
    let cases = quote! {::std::collections::HashMap::from([#cases])};
    let callbacks = token_callbacks.iter().fold(quote!{},
        |acc, tc| {
            let input = &tc.input.0;
            let input_conditions = tc.input.1.iter().enumerate().fold(quote!{},
                |acc_input, (i, f)| {
                    quote!{#acc_input #i => #input::#f(<#f as ::ntpnet_lib::transition_input_tokens::TransitionInputTokens>::from_map(in_map)),}
                }
            );
            let output = &tc.output.0;
            let output_outcomes = tc.output.1.iter().enumerate().fold(quote!{},
                |acc_output, (i, p)| {
                    quote!{#acc_output #output::#p(p) => {
                        ::ntpnet_lib::transition_output_tokens::TransitionOutputTokens::into_map(p, out_map);
                        #i
                    }}
                }
            );
            let name_str = tc.name.to_string();
            let name = &tc.name;
            quote!{#acc
                #name_str => {
                    let output = self.#name( match condition {
                        #input_conditions
                        _ => unimplemented!(),
                    });
                    match output {
                        #output_outcomes
                    }
                },
            }
        }
    );
    let gen = quote! {
        #interface_enums
        impl ::ntpnet_lib::transition::Transition for #name #lt_token {
            fn description(&self) -> ::ntpnet_lib::transition::Description
            {
                ::ntpnet_lib::transition::Description {
                    in_edges: #in_edges,
                    out_edges: #out_edges,
                    cases: #cases
                }
            }
            fn call(&mut self, case: &str, condition: usize,
                in_map: &mut ::std::collections::HashMap<(String, ::std::any::TypeId), ::ntpnet_lib::Token>,
                out_map: &mut ::std::collections::HashMap<(String, ::std::any::TypeId), ::ntpnet_lib::Token>,
            ) -> usize {
                let r = match case {
                    #callbacks
                    _ => unimplemented!(),
                };
                r
            }
        }
    };
    gen.into()
}

fn pop_interface(vt: &mut Vec<TokenTree>) -> (Ident, Vec<Ident>) {
    let name = match vt.remove(0) {
        TokenTree::Ident(i) => i,
        _ => unimplemented!(),
    };
    let enums = match vt.remove(0) {
        TokenTree::Group(g) => {
            assert!(g.delimiter() == Delimiter::Parenthesis);
            g.stream()
                .into_iter()
                .filter_map(|t| match t {
                    TokenTree::Ident(i) => Some(i),
                    _ => None,
                })
                .collect::<Vec<Ident>>()
        }
        _ => unimplemented!(),
    };
    (name, enums)
}

fn get_attr(ast: &syn::DeriveInput, attr: &str) -> Vec<proc_macro2::TokenStream> {
    ast.attrs
        .iter()
        .filter(|a| a.path.segments[0].ident == attr)
        .map(|a| a.tokens.clone())
        .fold(quote! {}, |acc, x| quote! {#acc #x,})
        .into_iter()
        .filter_map(|g| match g {
            Group(g) => Some(g.stream().into()),
            _ => None,
        })
        .collect::<Vec<_>>()
}

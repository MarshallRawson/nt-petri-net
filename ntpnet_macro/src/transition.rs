use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenTree, Delimiter};
use proc_macro2::TokenTree::{Group};
use quote::quote;
use quote::ToTokens;
use std::collections::HashSet;

pub fn impl_transition_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    //#[derive(Debug)]
    struct TransitionCallback {
        name: Ident,
        fire: (Ident, Vec<Ident>),
        product: (Ident, Vec<Ident>),
    }
    let token_callbacks = get_attr(ast, "ntpnet_transition").iter().map(|ts| {
            let mut vt = ts.clone().into_iter().collect::<Vec<_>>();
            let name = match vt.remove(0) {
                TokenTree::Ident(i) => i,
                _ => unimplemented!(),
            };
            match vt.remove(0) {
                TokenTree::Punct(p) => assert!(p.as_char() == ':'),
                _ => unimplemented!(),
            }
            let fire = pop_interface(&mut vt);
            match vt.remove(0) {
                TokenTree::Punct(p) => assert!(p.as_char() == '-'),
                _ => unimplemented!(),
            }
            match vt.remove(0) {
                TokenTree::Punct(p) => assert!(p.as_char() == '>'),
                _ => unimplemented!(),
            };
            let product = pop_interface(&mut vt);
            TransitionCallback {
                name: name,
                fire: fire,
                product: product,
            }
        }).collect::<Vec<TransitionCallback>>();
    let interface_enums = token_callbacks.iter().fold(vec![], |mut acc, tc| {
            acc.push(tc.fire.clone());
            acc.push(tc.product.clone());
            acc
        }).iter().fold(quote!{}, |acc, (name, enums)| {
            let enum_fields = enums.iter().fold(quote!{}, |acc, e| quote!{#acc #e(#e),});
            quote!{#acc enum #name {#enum_fields}}
        });
    let in_edges = token_callbacks.iter().fold(HashSet::new(), |mut acc, tc| {
            for e in &tc.fire.1 {
                acc.insert(e.clone());
            }
            acc
        }).into_iter().collect::<Vec<_>>();
    let in_edges = {
        let enum_first = &in_edges[0];
        if in_edges.len() > 1 {
            let in_edges = in_edges[1..].iter().fold(
                quote!{<#enum_first as ::ntpnet_lib::fire::Fire>::in_edges() },
                |acc, e| quote!{#acc .union(&<#e as ::ntpnet_lib::fire::Fire>::in_edges())}
            );
            quote!{ #in_edges.cloned().collect() }
        } else {
            quote!{<#enum_first as ::ntpnet_lib::fire::Fire>::in_edges() }
        }
    };
    let out_edges = token_callbacks.iter().fold(HashSet::new(), |mut acc, tc| {
            for e in &tc.product.1 {
                acc.insert(e.clone());
            }
            acc
        }).into_iter().collect::<Vec<_>>();
    let out_edges = {
        let enum_first = &out_edges[0];
        if out_edges.len() > 1 {
            let out_edges = out_edges[1..].iter().fold(
                quote!{<#enum_first as ::ntpnet_lib::product::Product>::out_edges() },
                |acc, e| quote!{#acc .union(&<#e as ::ntpnet_lib::product::Product>::out_edges())}
            );
            quote!{ #out_edges.cloned().collect() }
        } else {
            quote!{<#enum_first as ::ntpnet_lib::product::Product>::out_edges() }
        }
    };
    let gen = quote! {
        #interface_enums
        impl ::ntpnet_lib::transition::Transition for #name {
            fn in_edges(&self) -> ::std::collections::HashSet<(String, ::std::any::TypeId)> {
                #in_edges
            }
            fn out_edges(&self) -> ::std::collections::HashSet<(String, ::std::any::TypeId)> {
                #out_edges
            }
            fn transitions(&self) -> Vec<::ntpnet_lib::transition::TransitionCase> {
                vec![]
            }
            fn call(&mut self, map: &mut ::std::collections::HashMap<(String, ::std::any::TypeId),
                    ::ntpnet_lib::Token>) ->
                ::std::collections::HashMap<(String, ::std::any::TypeId), ::ntpnet_lib::Token>
            {
                ::std::collections::HashMap::from([])
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
            g.stream().into_iter().filter_map(|t| {
                match t {
                    TokenTree::Ident(i) => Some(i),
                    _ => None,
                }
            }).collect::<Vec<Ident>>()
        },
        _ => unimplemented!(),
    };
    (name, enums)
}

fn get_attr(ast: &syn::DeriveInput, attr: &str) -> Vec<proc_macro2::TokenStream> {
    ast.attrs
        .iter()
        .filter(|a| a.path.segments[0].ident == attr)
        .map(|a| a.tokens.clone())
        .fold(quote!{}, |acc, x| quote!{#acc #x,})
        .into_iter().filter_map(|g| {
            match g {
                Group(g) => Some(g.stream().into()),
                _ => None,
            }
        }).collect::<Vec<_>>()
}

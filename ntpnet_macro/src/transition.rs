use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenTree, Delimiter};
use proc_macro2::TokenTree::{Group};
use quote::quote;
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
    let cases = token_callbacks.iter().fold(quote!{},
        |acc, tc| {
            let conditions = tc.fire.1.iter().fold(quote!{},
                |acc_cond, e| {
                    quote!{#acc_cond <#e as ::ntpnet_lib::fire::Fire>::in_edges(),}
                }
            );
            let conditions = quote!{vec![#conditions]};
            let products = tc.product.1.iter().fold(quote!{},
                |acc_prod, e| {
                    quote!{#acc_prod <#e as ::ntpnet_lib::product::Product>::out_edges(),}
                }
            );
            let products = quote!{vec![#products]};
            let name_str = tc.name.to_string();
            quote!{#acc
                (#name_str.into(), ::ntpnet_lib::transition::TransitionCase {
                    conditions: #conditions,
                    products: #products,
                }),
            }
        }
    );
    let cases = quote!{::std::collections::HashMap::from([#cases])};
    let callbacks = token_callbacks.iter().fold(quote!{},
        |acc, tc| {
            let fire = &tc.fire.0;
            let fire_conditions = tc.fire.1.iter().enumerate().fold(quote!{},
                |acc_fire, (i, f)| {
                    quote!{#acc_fire #i => #fire::#f(<#f as ::ntpnet_lib::fire::Fire>::from_map(in_map)),}
                }
            );
            let product = &tc.product.0;
            let product_outcomes = tc.product.1.iter().enumerate().fold(quote!{},
                |acc_product, (i, p)| {
                    quote!{#acc_product #product::#p(p) => {
                        ::ntpnet_lib::product::Product::into_map(p, out_map);
                        #i
                    }}
                }
            );
            let name_str = tc.name.to_string();
            let name = &tc.name;
            quote!{#acc
                #name_str => {
                    let product = self.#name( match condition {
                        #fire_conditions
                        _ => unimplemented!(),
                    });
                    match product {
                        #product_outcomes
                    }
                },
            }
        }
    );
    let gen = quote! {
        #interface_enums
        impl ::ntpnet_lib::transition::Transition for #name {
            fn descr(&self) -> ::ntpnet_lib::transition::TransitionDescr
            {
                ::ntpnet_lib::transition::TransitionDescr {
                    in_edges: #in_edges,
                    out_edges: #out_edges,
                    cases: #cases
                }
            }
            fn call(&mut self, case: &str, condition: usize,
                in_map: &mut ::std::collections::HashMap<(String, ::std::any::TypeId), ::ntpnet_lib::Token>,
                out_map: &mut ::std::collections::HashMap<(String, ::std::any::TypeId), ::ntpnet_lib::Token>,
            ) -> usize {
                match case {
                    #callbacks
                    _ => unimplemented!(),
                }
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

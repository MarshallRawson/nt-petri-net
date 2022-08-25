use syn::{Ident, FieldsNamed, Type};
use syn::token::Colon;
use quote::quote;
use proc_macro2::TokenStream;


pub fn struct_field_names(ast: &syn::DeriveInput) -> Vec<Ident> {
    match &ast.data {
        syn::Data::Struct(s) => match &s.fields {
            syn::Fields::Named(FieldsNamed { named, .. }) => {
                named.iter().map(|f| f.ident.as_ref().unwrap().clone()).collect::<Vec<_>>()
            }
            _ => todo!()
        }
        _ => todo!(),
    }
}

fn struct_field_names_types(ast: &syn::DeriveInput) -> Vec<(Ident, Type)> {
    match &ast.data {
        syn::Data::Struct(s) => match &s.fields {
            syn::Fields::Named(FieldsNamed { named, .. }) => {
                named.iter().map(|f| {
                    (f.ident.as_ref().unwrap().clone(), f.ty.clone())
                }).collect::<Vec<_>>()
            }
            _ => todo!()
        }
        _ => todo!(),
    }
}

pub fn field_descriptions_HashSet(ast: &syn::DeriveInput) -> TokenStream {
    let field_descriptions = struct_field_names_types(&ast).iter().fold(quote!{},
        |acc, (field, ty)| {
            quote!{
                #acc
                ("#field".to_string(), ::std::any::TypeId::of::<#ty>()),
            }
        }
    );
    quote!{
        ::std::collections::HashSet::from(
            [#field_descriptions]
        )
    }
}


use proc_macro::TokenStream;
use proc_macro2;
use quote::quote;
use quote::ToTokens;
use syn;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Result, Token, Type};
use proc_macro2::Delimiter::Parenthesis;

pub fn impl_transition_macro(ast: &syn::DeriveInput) -> TokenStream {
    println!("transition Fire: {:#?}", 
        ast.attrs.iter()
            .filter(|a| a.path.segments[0].ident == "ntpnet_transitions")
            .map(|a| a.tokens.clone())
        //.collect::<Vec<_>>()[0].iter()
        //    .filter(|g| g.delimiter == Parenthesis)
        .collect::<Vec<_>>()
    );
    let gen = quote! {};
    gen.into()
}


/*
fn impl_mnet_place_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let PlaceParams {
        function,
        in_type,
        mut out_types,
        mut out_section,
    } = {
        let attribute = ast
            .attrs
            .iter()
            .filter(|a| a.path.segments.len() == 1 && a.path.segments[0].ident == "mnet_place")
            .nth(0)
            .expect("mnet_place is required by MnetPlace");
        syn::parse2(attribute.tokens.clone()).expect("Invalid mnet_place attribute!")
    };
    let place_enum: Option<PlaceEnumParams> = match ast
        .attrs
        .iter()
        .filter(|a| a.path.segments.len() == 1 && a.path.segments[0].ident == "mnet_place_enum")
        .nth(0)
    {
        Some(attribute) => {
            Some(syn::parse2(attribute.tokens.clone()).expect("Invalid mnet_place attribute!"))
        }
        None => None,
    };
    if let Some(enum_params) = place_enum {
        out_section = enum_params.out_section;
        out_types = enum_params.out_types;
    }
    let out_type_block: proc_macro2::TokenStream = {
        let mut out_type_block = "::std::collections::HashSet::from([\n".to_string();
        for t in &out_types {
            out_type_block += &format!("::std::any::TypeId::of::<{}>(),\n", t.to_token_stream());
        }
        out_type_block += "])";
        //println!("{}", out_type_block);
        out_type_block.parse().unwrap()
    };
    let out_type_block_names: proc_macro2::TokenStream = {
        let mut out_type_block_names = "::std::collections::HashSet::from([\n".to_string();
        for t in &out_types {
            out_type_block_names += &format!(
                "::std::any::type_name::<{}>().into(),\n",
                t.to_token_stream()
            );
        }
        out_type_block_names += "])";
        //println!("{}", out_type_block_names);
        out_type_block_names.parse().unwrap()
    };
    let gen = quote! {
        impl #name {
            fn in_type() -> ::std::any::TypeId {
                ::std::any::TypeId::of::<#in_type>()
            }
            fn out_types() -> ::std::collections::HashSet<::std::any::TypeId> {
                #out_type_block
            }
            fn out_types_names() -> ::std::collections::HashSet<::std::string::String> {
                #out_type_block_names
            }
        }
        impl Place for #name {
            fn run(
                &mut self,
                x: Box<dyn ::std::any::Any + ::std::marker::Send>,
                out_map: &mut ::std::collections::HashMap::<
                    ::std::any::TypeId,
                    mnet_lib::Edge
                >
            ) -> ::std::any::TypeId {
                let y = self.#function(*x.downcast::<#in_type>().unwrap());
                #out_section
            }
        }
    };
    //println!("{}", gen);
    gen.into()
}

struct PlaceEnumParams {
    out_types: Vec<Type>,
    out_section: proc_macro2::TokenStream,
}
impl Parse for PlaceEnumParams {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        syn::parenthesized!(content in input);
        let (out_variants, out_types): (Vec<syn::Type>, Vec<syn::Type>) = {
            let mut out_types = vec![];
            let mut out_variants = vec![];
            while let Ok(variant) = content.parse() {
                out_variants.push(variant);
                content.parse::<Token![,]>()?;
                out_types.push(content.parse()?);
                if let Err(_) = content.parse::<Token![,]>() {
                    break;
                }
            }
            (out_variants, out_types)
        };
        let out_section: proc_macro2::TokenStream = {
            let mut ret = "match y {\n".to_string();
            for v in out_variants {
                ret += &format!(
                    "{}(y) => {{
                        let y : Box<dyn ::std::any::Any + ::std::marker::Send> = ::std::boxed::Box::new(y);
                        let out_type = (&*y).type_id();
                        out_map.get_mut(&out_type).unwrap().push(y);
                        out_type
                    }},\n",
                    v.to_token_stream(),
                );
            }
            ret += "}";
            //println!("{}", ret);
            ret.parse().unwrap()
        };
        Ok(PlaceEnumParams {
            out_types: out_types,
            out_section: out_section,
        })
    }
}

struct PlaceParams {
    function: Ident,
    in_type: Type,
    out_types: Vec<Type>,
    out_section: proc_macro2::TokenStream,
}
impl Parse for PlaceParams {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        syn::parenthesized!(content in input);
        let function = content.parse()?;
        content.parse::<Token![,]>()?;
        let in_type = content.parse()?;
        content.parse::<Token![,]>()?;
        let (out_section, out_type): (proc_macro2::TokenStream, syn::Type) = {
            let out_type: syn::Type = content.parse()?;
            let ret = format!("
                out_map.get_mut(&::std::any::TypeId::of::<{}>()).unwrap().push(::std::boxed::Box::new(y)); ::std::any::TypeId::of::<{}>()\n",
                out_type.to_token_stream(),
                out_type.to_token_stream(),
            ).to_string();
            //println!("{}", ret);
            (ret.parse().unwrap(), out_type)
        };
        Ok(PlaceParams {
            function: function,
            in_type: in_type,
            out_types: vec![out_type],
            out_section: out_section,
        })
    }
}
*/

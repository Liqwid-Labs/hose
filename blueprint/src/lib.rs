use std::fs;

use heck::ToSnakeCase;
use proc_macro::TokenStream;
use proc_macro2::Literal;
use quote::{format_ident, quote};
use schema::{BlueprintSchema, TypeSchema, TypeSchemaTagged};
use syn::{parse_macro_input, LitStr};

mod schema;

#[proc_macro]
pub fn generate_cbor_struct(input: TokenStream) -> TokenStream {
    // Parse the input as a string literal
    let input = parse_macro_input!(input as LitStr);

    let file_path = input.value();
    let contents = fs::read_to_string(file_path.clone())
        .unwrap_or_else(|_| panic!("Failed to read plutus json definition: {file_path}"));
    let schema: BlueprintSchema = serde_json::from_str(&contents).unwrap();

    let types = schema.definitions.into_iter().map(|(key, def)| {
        match def {
            TypeSchema::Tagged(TypeSchemaTagged::Int) => {
                let ident = format_ident!("{}", key);
                quote! { type #ident = pallas::codec::utils::AnyUInt; }
            }
            TypeSchema::Tagged(TypeSchemaTagged::Bytes) => {
                let ident = format_ident!("{}", key);
                quote! { type #ident = pallas::codec::utils::Bytes; }
            }
            TypeSchema::Tagged(TypeSchemaTagged::Constructor { title, fields }) => {
                generate_constructor_struct(&title, &fields)
            }

            TypeSchema::Variants { title: _, any_of } => {
                // TODO: We would need to use enum instead of struct if there are multiple
                if any_of.len() != 1 {
                    todo!("Multiple variants aren't supported");
                }

                match &any_of[0] {
                    TypeSchema::Tagged(TypeSchemaTagged::Constructor { title, fields }) => {
                        generate_constructor_struct(title, fields)
                    }
                    _ => todo!("Only constructor datatypes are supported in variants"),
                }
            }

            TypeSchema::Reference {
                title: _,
                reference: _,
            } => {
                todo!("Top level reference types are not supported")
            }
        }
    });

    TokenStream::from(quote! {
        #(#types)*
    })
}

fn generate_constructor_struct(title: &str, fields: &[TypeSchema]) -> proc_macro2::TokenStream {
    let field_definitions = fields.iter().enumerate().map(|(i, field)| match field {
        TypeSchema::Reference { title, reference } => {
            let index = Literal::usize_unsuffixed(i);
            let field_name = format_ident!("{}", title.to_snake_case());
            let field_type = format_ident!("{}", reference.split('/').last().unwrap());
            quote! {
                #[n(#index)] pub #field_name: #field_type
            }
        }
        _ => todo!("Only reference fields are supported in constructor variants"),
    });

    let struct_ident = format_ident!("{}", title);
    quote! {
        #[derive(Debug, minicbor::Encode, minicbor::Decode, PartialEq, Clone)]
        pub struct #struct_ident {
            #(#field_definitions),*
        }
    }
}

use std::{collections::HashMap, fs};

use heck::ToSnakeCase;
use proc_macro2::{Literal, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use schema::{BlueprintSchema, ListItems, TypeSchema, TypeSchemaTagged};
use syn::{parse_macro_input, LitStr};

pub mod ir;
pub mod module;
pub mod reference;
pub mod safe_rename;
pub mod schema;

use safe_rename::{SafeRename, UnsafeName};

// Format a type schema as just its type.
fn format_type_schema(schema: TypeSchema) -> Option<TokenStream2> {
    match schema {
        TypeSchema::Tagged(TypeSchemaTagged::Int) => Some(quote! { pallas::codec::utils::AnyUInt }),
        TypeSchema::Tagged(TypeSchemaTagged::Bytes) => Some(quote! { pallas::codec::utils::Bytes }),
        TypeSchema::OpaqueData { .. } => Some(quote! { pallas::codec::utils::AnyCbor }),
        TypeSchema::Tagged(TypeSchemaTagged::List { items, title: _ }) => match items {
            ListItems::Monomorphic(items) => {
                let items = format_type_schema(*items)?;
                Some(quote! { Vec<#items> })
            }
            ListItems::Polymorphic(items) => {
                let items = items
                    .into_iter()
                    .map(format_type_schema)
                    .collect::<Vec<Option<_>>>();

                let items: Vec<_> = items.into_iter().collect::<Option<_>>()?;

                Some(quote! { (#(#items),*) })
            }
        },

        TypeSchema::Reference { title, reference } => {
            let reference = reference
                .split()
                .last()
                .expect(".last on reference")
                .to_owned();
            let ty = format_ident!("{}", reference.safe_rename());
            Some(quote! { #ty })
        }
        _ => None,
    }
}

fn format_definition(key: &str, schema: TypeSchema) -> anyhow::Result<TokenStream2> {
    match schema {
        TypeSchema::Tagged(
            TypeSchemaTagged::Int | TypeSchemaTagged::Bytes | TypeSchemaTagged::List { .. },
        )
        | TypeSchema::OpaqueData { .. } => {
            let ident = format_ident!("{}", key);
            let ty = format_type_schema(schema).expect(
                "Failed to format type schema, but we should always be able to format these types",
            );
            Ok(quote! { type #ident = #ty; })
        }

        TypeSchema::Variants { title, any_of } => {
            // TODO: We would need to use enum instead of struct if there are multiple
            if any_of.len() != 1 {
                Ok(generate_constructor_enum(key, title, any_of))
            } else {
                match &any_of[0] {
                    TypeSchema::Tagged(TypeSchemaTagged::Constructor {
                        title: _,
                        index,
                        fields,
                        // Should not clone here!!!
                    }) => Ok(generate_constructor_struct(key, *index, fields.clone())),
                    _ => todo!("Only constructor datatypes are supported in variants"),
                }
            }
        }

        TypeSchema::Reference {
            title: _,
            reference: _,
        } => {
            todo!("Top level reference types are not supported")
        }

        _ => todo!("This type schema is not supported: {:?}", schema),
    }
}

pub fn generate_cbor_struct2(file_path: &str) -> TokenStream2 {
    // Parse the input as a string literal
    let contents = fs::read_to_string(file_path)
        .unwrap_or_else(|_| panic!("Failed to read plutus json definition: {file_path}"));
    let schema: BlueprintSchema = serde_json::from_str(&contents)
        .unwrap_or_else(|e| panic!("Failed to parse blueprint schema: {e}"));

    let types = schema.definitions.into_iter().map(|(key, def)| {
        match format_definition(&key.safe_rename(), def) {
            Ok(v) => Some(v),
            Err(e) => panic!("Failed to format definition: {e}"),
        }
    });

    quote! {
        use pallas;
        use minicbor;

        #(#types)*
    }
}

fn generate_constructor_enum(
    type_name: &str,
    title: UnsafeName,
    any_of: Vec<TypeSchema>,
) -> proc_macro2::TokenStream {
    let enum_name = format_ident!("{}", type_name);

    let variants = any_of.iter().enumerate().map(|(i, variant)| {
        let (index, variant_name, fields) =
            if let TypeSchema::Tagged(TypeSchemaTagged::Constructor {
                title,
                index,
                fields,
            }) = variant
            {
                (index, title, fields)
            } else {
                todo!("Only constructor datatypes are supported in variants")
            };

        let field_definitions = fields
            .iter()
            .enumerate()
            .map(|(i, field)| match field {
                TypeSchema::Reference { title, reference } => {
                    // Should not clone here
                    let type_name = reference
                        .clone()
                        .split()
                        .last()
                        .unwrap()
                        .to_owned()
                        .safe_rename();
                    ConstructorField {
                        index: i,
                        title: title.clone(),
                        type_name,
                    }
                }
                _ => todo!("Only reference fields are supported in constructor variants"),
            })
            .collect::<Vec<_>>();

        let field_definitions = generate_constructor_fields(field_definitions);

        let variant_name = format_ident!("{}", variant_name.clone().safe_rename());

        quote! {
            #[n(#index)]
            #variant_name #field_definitions
        }
    });

    let title = title.safe_rename();

    quote! {
        #[derive(Debug, minicbor::Encode, minicbor::Decode, PartialEq, Clone)]
        pub enum #enum_name {
            #(#variants),*
        }
    }
}

#[derive(Debug, Clone)]
struct ConstructorField {
    index: usize,
    title: Option<UnsafeName>,
    type_name: String,
}

fn compute_cbor_tag(index: usize) -> usize {
    if index < 7 {
        121 + index
    } else if index < 128 {
        1280 + index - 7
    } else {
        // See: https://github.com/aiken-lang/aiken/blob/6d2e38851eb9b14cf5ea04fdc4722405b5c1544a/crates/uplc/src/ast.rs#L437
        todo!("Constructors with more than 128 fields are not (yet) supported, you have {index}")
    }
}

fn generate_constructor_fields(fields: Vec<ConstructorField>) -> proc_macro2::TokenStream {
    // TODO: Don't clone!
    let all_fields_have_title = fields.clone().iter().all(|field| field.title.is_some());

    if all_fields_have_title {
        let fields = fields.iter().map(|field| {
            let field_name = format_ident!(
                "{}",
                field
                    .title
                    .clone()
                    .expect("We checked all fields have title")
                    .safe_rename()
                    .to_snake_case()
            );
            let field_type = format_ident!("{}", field.clone().type_name);
            let index = Literal::usize_unsuffixed(field.index);
            quote! {
                #[n(#index)] #field_name: #field_type
            }
        });
        quote! {
            { #(#fields),* }
        }
    } else {
        let fields = fields.iter().map(|field| {
            let field_type = format_ident!("{}", field.clone().type_name);
            let index = Literal::usize_unsuffixed(field.index);
            quote! {
                #[n(#index)] #field_type
            }
        });
        quote! {
            ( #(#fields),* )
        }
    }
}

fn generate_constructor_struct(
    title: &str,
    index: usize,
    fields: Vec<TypeSchema>,
) -> proc_macro2::TokenStream {
    let field_definitions = fields
        .iter()
        .enumerate()
        .map(|(i, field)| match field {
            TypeSchema::Reference { title, reference } => {
                // Should not clone here
                let type_name = reference
                    .clone()
                    .split()
                    .last()
                    .unwrap()
                    .to_owned()
                    .safe_rename();
                ConstructorField {
                    index: i,
                    title: title.clone(),
                    type_name,
                }
            }
            _ => todo!("Only reference fields are supported in constructor variants"),
        })
        .collect::<Vec<_>>();

    let field_definitions = generate_constructor_fields(field_definitions);

    let tag = compute_cbor_tag(index);

    let struct_ident = format_ident!("{}", title);
    quote! {
        #[derive(Debug, minicbor::Encode, minicbor::Decode, PartialEq, Clone)]
        #[cbor(tag(#tag))]
        pub struct #struct_ident #field_definitions
    }
}

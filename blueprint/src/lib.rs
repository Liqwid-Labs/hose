use std::fs;

use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use serde_json::Value;
use syn::{parse_macro_input, LitStr};

mod schema;

#[proc_macro]
pub fn generate_cbor_struct(input: TokenStream) -> TokenStream {
    // Parse the input as a string literal
    let input = parse_macro_input!(input as LitStr);

    let file_path = input.value();
    let contents = fs::read_to_string(file_path.clone())
        .unwrap_or_else(|_| panic!("Failed to read plutus json definition: {file_path}"));
    let json: Value = serde_json::from_str(&contents).unwrap();

    // Get the first (and presumably only) definition
    let definitions = json["definitions"].as_object().unwrap();

    let types = definitions.iter().map(|(_, def)| {
        // Get the struct name and fields from the first anyOf variant
        let type_name = def["title"].as_str().unwrap();

        if let Some(datatype) = def.get("dataType") {
            let type_value = match datatype.as_str().unwrap() {
                "integer" => "pallas::codec::utils::AnyUInt",
                "bytes" => "pallas::codec::utils::Bytes",
                _ => panic!("Unsupported data type: {datatype}"),
            };
            quote! {
                type #type_name = #type_value;
            }
        } else if let Some(any_of) = def.get("anyOf") {
            if any_of.as_array().unwrap().len() != 1 {
                todo!("Support multiple anyOf variants");
            }

            // TODO: assumes single anyOf field
            // We would need to use enum instead of struct if there are multiple
            let variant = &any_of[0];
            if variant["dataType"] != "constructor" {
                todo!("Only constructor datatypes are supported as a variant with anyOf");
            }
            let fields = &variant["fields"]
                .as_array()
                .expect("Expected fields in anyOf variant, pure datatype isn't supported");

            // Convert fields to Rust field definitions
            let field_definitions = fields.iter().enumerate().map(|(i, field)| {
                let field_name = field["title"].as_str().unwrap();

                let field_ident = format_ident!("{}", field_name.to_snake_case());
                let field_type = field["$ref"].as_str().unwrap().split('/').last().unwrap();

                quote! {
                    #[n(#i)]
                    pub #field_ident: #field_type
                }
            });

            // Generate the final struct
            let struct_ident = format_ident!("{}", type_name);
            quote! {
                #[derive(Debug, Encode, Decode, Serialize, Deserialize, PartialEq, Clone)]
                pub struct #struct_ident {
                    #(#field_definitions),*
                }
            }
        } else {
            panic!("Unsupported definition: {def:?}")
        }
    });

    TokenStream::from(quote! {
        #(#types)*
    })
}

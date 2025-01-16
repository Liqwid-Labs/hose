use std::fs;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use serde_json::{json, Value};
use syn::{parse_macro_input, LitStr};

#[proc_macro]
pub fn generate_cbor_struct(input: TokenStream) -> TokenStream {
    // Parse the input as a string literal
    let input = parse_macro_input!(input as LitStr);

    let contents = fs::read_to_string(input.value()).expect(&format!(
        "Failed to read plutus json definition: {file_path}"
    ));
    let json: Value = serde_json::from_str(&contents).unwrap();

    // Get the first (and presumably only) definition
    let definitions = json["definitions"].as_object().unwrap();
    let (_, struct_def) = definitions.iter().next().unwrap();

    // Get the struct name and fields from the first anyOf variant
    let struct_name = struct_def["title"].as_str().unwrap();
    // TODO: assumes single anyOf field
    // We would need to use enum instead of struct if there are multiple
    let fields = &struct_def["anyOf"][0]["fields"].as_array().unwrap();

    // Convert fields to Rust field definitions
    let field_definitions = fields.iter().enumerate().map(|(i, field)| {
        let field_name = field["title"].as_str().unwrap();
        let field_ident = format_ident!("{}", to_snake_case(field_name));

        // Convert field type (assuming all are BigInt for this example)
        quote! {
            #[n(#i)]
            pub #field_ident: BigInt
        }
    });

    // Generate the final struct
    let struct_ident = format_ident!("{}", struct_name);
    let expanded = quote! {
        #[derive(Debug, Encode, Decode, Serialize, Deserialize, PartialEq, Clone)]
        pub struct #struct_ident {
            #(#field_definitions),*
        }
    };

    expanded.into()
}

// Helper function to convert camelCase to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c.is_uppercase() {
            if !result.is_empty() && chars.peek().map_or(false, |next| next.is_lowercase()) {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }

    result
}

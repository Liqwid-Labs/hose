use quote::ToTokens;
use syn::{parse_macro_input, LitStr};

use proc_macro2::TokenStream as TokenStream2;

use hose_blueprint_internal::{ir::collect_definitions, module::Module, schema::BlueprintSchema};

#[proc_macro]
pub fn blueprint(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as LitStr);

    let file_path = input.value();

    let blueprint = BlueprintSchema::from_file(&file_path).unwrap();

    let definitions = collect_definitions(&blueprint).unwrap();

    let modules = Module::from_definitions(&definitions);

    let mut tokens = TokenStream2::new();

    modules.to_tokens(&mut tokens);

    proc_macro::TokenStream::from(tokens)
}

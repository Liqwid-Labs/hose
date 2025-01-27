use syn::{parse_macro_input, LitStr};

use hose_blueprint_internal::generate_cbor_struct2;

#[proc_macro]
pub fn generate_cbor_struct(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as LitStr);

    let file_path = input.value();

    generate_cbor_struct2(&file_path).into()
}

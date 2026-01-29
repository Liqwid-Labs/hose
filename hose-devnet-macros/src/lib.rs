extern crate proc_macro;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_quote;

/// A macro for annotating a devnet test with Hose.
/// There are a number of requirements for running a devnet test, and we implicitly
/// ensure that they all are met.
///
/// First, we need a context that implements AsyncTestContext. We use `#[test_context]` to inject the context into the test.
/// We also need to make the test run serially. We do this using the `#[serial]` attribute.
///
/// Example usage:
/// ```
/// #[hose_devnet::test]
/// async fn my_test(context: &mut DevnetContext) -> anyhow::Result<()> {
///     Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn test(_attrs: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);

    let refactored_fn = refactor_fn(input);

    quote! {
        #refactored_fn
    }
    .into()
}

// Tasks:
// - Add attributes to the function
fn refactor_fn(input: syn::ItemFn) -> syn::ItemFn {
    let mut attrs = input.attrs.clone();
    let vis = input.vis.clone();
    let sig = input.sig.clone();
    let block = input.block.clone();

    attrs.push(
        parse_quote!(#[::hose_devnet::test_context::test_context(hose_devnet::DevnetContext)]),
    );
    attrs.push(parse_quote!(#[::hose_devnet::serial_test::serial]));
    attrs.push(parse_quote!(#[::hose_devnet::tokio::test]));

    syn::ItemFn {
        attrs,
        vis,
        sig,
        block,
    }
}

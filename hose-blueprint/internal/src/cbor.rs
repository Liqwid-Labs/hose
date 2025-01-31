use std::collections::HashMap;

use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::ir::{compute_cbor_tag, Constructor};

/// Derive a `minicbor::Encode` implementation for a given enum.
///
/// Why do we have to do this? Because minicbor-derive doesn't support the same encoding as is
/// used in Plutus data structures.
///
/// It essentially always encodes an enum as an array or map with a tag, but we need an explicit
/// tag.
///
/// Reference implementation for Credential:
/// ```rust
/// pub enum Credential {
///     Script(pallas::codec::utils::Bytes),
///     VerificationKey(pallas::codec::utils::Bytes)
/// }
///
/// impl<C> minicbor::encode::Encode<C> for Credential {
///     // Required method
///     fn encode<W: minicbor::encode::Write>(
///         &self,
///         e: &mut minicbor::Encoder<W>,
///         ctx: &mut C,
///     ) -> Result<(), minicbor::encode::Error<W::Error>> {
///         
///         match self {
///             Self::Script(script) => {
///                 e.tag(minicbor::data::Tag::new(121))?;
///                 _ = script.encode(e, ctx);
///             }
///             Self::VerificationKey(key) => {
///                 e.tag(minicbor::data::Tag::new(122))?;
///                 _ = key.encode(e, ctx);
///             }
///         }
///
///         Ok(())
///     }
/// }
///
/// impl<'b, C> minicbor::decode::Decode<'b, C> for Credential {
///     fn decode(d: &mut minicbor::decode::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
///         let tag = d.tag()?;
///         match tag.as_u64() {
///             121 => {
///                 let script = pallas::codec::utils::Bytes::decode(d, ctx)?;
///                 Ok(Self::Script(script))
///             }
///             122 => {
///                 let key = pallas::codec::utils::Bytes::decode(d, ctx)?;
///                 Ok(Self::VerificationKey(key))
///             }
///             _ => Err(minicbor::decode::Error::message("Invalid tag for Credential")),
///         }
///
///     }
/// }
/// ```
pub fn decode_encode_impl_enum(
    definition_name: Ident,
    variants: &HashMap<String, (usize, Constructor)>,
) -> TokenStream {
    let (encode_variant_matches, decode_variant_matches): (Vec<_>, Vec<_>) = variants
        .iter()
        .map(|(name, (index, constructor))| {
            let constructor_name = format_ident!("{}", name.to_upper_camel_case());

            let tag = compute_cbor_tag(*index);

            let (should_curly, binding_names) = match constructor {
                Constructor::Named(fields) => (
                    true,
                    fields
                        .iter()
                        .map(|(name, _)| {
                            let name = format_ident!("{}", name.to_snake_case());
                            quote! { #name }
                        })
                        .collect::<Vec<_>>(),
                ),
                Constructor::Unnamed(fields) => (
                    false,
                    fields
                        .iter()
                        .enumerate()
                        .map(|(index, _)| {
                            let name = format_ident!("inner_{}", index);
                            quote! { #name }
                        })
                        .collect::<Vec<_>>(),
                ),
            };

            let binding = if should_curly {
                quote! { { #(#binding_names),* } }
            } else {
                quote! { ( #(#binding_names),* ) }
            };

            let encodes = binding_names.clone().into_iter().map(|name| {
                quote! {
                    _ = #name.encode(e, ctx);
                }
            });

            let encode_match = quote! {
                Self::#constructor_name #binding => {
                    e.tag(minicbor::data::Tag::new(#tag))?;
                    _ = e.begin_array();
                    #(#encodes)*
                    _ = e.end();
                }
            };

            let parse_bindings = binding_names.clone().into_iter().map(|binding_name| {
                quote! {
                    let #binding_name = minicbor::decode::Decode::decode(d, ctx)?;
                }
            });

            let decode_match = quote! {
                #tag => {
                    _ = d.array()?;
                    #(#parse_bindings)*
                    _ = d.skip();
                    Ok(Self::#constructor_name #binding)
                }
            };

            (encode_match, decode_match)
        })
        .collect();

    let invalid_msg = format!("Invalid tag for {}", definition_name);

    quote! {
        impl<C> minicbor::encode::Encode<C> for #definition_name {
            // Required method
            fn encode<W: minicbor::encode::Write>(
                &self,
                e: &mut minicbor::Encoder<W>,
                ctx: &mut C,
            ) -> Result<(), minicbor::encode::Error<W::Error>> {

                match self {
                    #(#encode_variant_matches)*
                }

                Ok(())
            }
        }

        impl<'b, C> minicbor::decode::Decode<'b, C> for #definition_name {
            fn decode(d: &mut minicbor::decode::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
                let tag = d.tag()?;
                match tag.as_u64() {
                    #(#decode_variant_matches)*
                    _ => Err(minicbor::decode::Error::message(#invalid_msg)),
                }
            }
        }
    }
}

/// Derive a `minicbor::Encode` and `minicbor::Decode` implementation pair for a given struct.
///
/// If index_tag is None, the struct is encoded without a tag, so it will be just a list.
pub fn decode_encode_impl_struct(
    definition_name: Ident,
    index_tag: Option<usize>,
    constructor: Constructor,
) -> TokenStream {
    let tag = index_tag.map(compute_cbor_tag);

    let (should_curly, binding_names) = match constructor {
        Constructor::Named(fields) => (
            true,
            fields
                .iter()
                .map(|(name, _)| {
                    let name = format_ident!("{}", name.to_snake_case());
                    quote! { #name }
                })
                .collect::<Vec<_>>(),
        ),
        Constructor::Unnamed(fields) => (
            false,
            fields
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    let name = format_ident!("inner_{}", index);
                    quote! { #name }
                })
                .collect::<Vec<_>>(),
        ),
    };

    let binding = if should_curly {
        quote! { { #(#binding_names),* } }
    } else {
        quote! { ( #(#binding_names),* ) }
    };

    let encodes = binding_names.clone().into_iter().map(|name| {
        quote! {
            _ = #name.encode(e, ctx);
        }
    });

    let create_tag = if let Some(tag) = tag {
        quote! { e.tag(minicbor::data::Tag::new(#tag))?; }
    } else {
        quote! {}
    };

    let encode_match = quote! {
        Self #binding => {
            #create_tag
            _ = e.begin_array();
            #(#encodes)*
            _ = e.end();
        }
    };

    let parse_bindings = binding_names.clone().into_iter().map(|binding_name| {
        quote! {
            let #binding_name = minicbor::decode::Decode::decode(d, ctx)?;
        }
    });

    let invalid_msg = format!("Invalid tag for {}", definition_name);

    let decode_body = match tag {
        Some(tag) => {
            let decode_match = quote! {
                #tag => {
                    _ = d.array()?;
                    #(#parse_bindings)*
                    _ = d.skip();
                    Ok(Self #binding)
                }
            };
            quote! {
                let tag = d.tag()?;
                match tag.as_u64() {
                    #decode_match
                    _ => Err(minicbor::decode::Error::message(#invalid_msg)),
                }
            }
        }
        None => quote! {
            _ = d.array()?;
            #(#parse_bindings)*
            _ = d.skip();
            Ok(Self #binding)
        },
    };

    quote! {
        impl<C> minicbor::encode::Encode<C> for #definition_name {
            // Required method
            fn encode<W: minicbor::encode::Write>(
                &self,
                e: &mut minicbor::Encoder<W>,
                ctx: &mut C,
            ) -> Result<(), minicbor::encode::Error<W::Error>> {

                match self {
                    #encode_match
                }

                Ok(())
            }
        }

        impl<'b, C> minicbor::decode::Decode<'b, C> for #definition_name {
            fn decode(d: &mut minicbor::decode::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
                #decode_body
            }
        }
    }
}

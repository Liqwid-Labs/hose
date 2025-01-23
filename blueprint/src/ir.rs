use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

use crate::safe_rename::{UnsafeName, UnsafeRef};
use crate::schema;

// Represented as `path0::path1::path2::name`
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Ref {
    pub path: Vec<String>,
    pub name: String,
}

impl Ref {
    pub fn new(path: &[&str], name: &str) -> Self {
        Self {
            path: path
                .iter()
                .map(ToOwned::to_owned)
                .map(String::from)
                .collect(),
            name: name.to_string(),
        }
    }
}

impl ToTokens for Ref {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let path = self.path.clone();
        let name = self.name.clone();
        tokens.extend(quote! { #(#path)::* #name });
    }
}

impl Ref {
    pub fn parse_from_name(full_name: String) -> anyhow::Result<Self> {
        let full_module_path = full_name
            .split("/")
            .map(String::from)
            .collect::<Vec<String>>();

        // Last is name, everything else is module path
        let (module_path, name) = full_module_path.split_at(full_module_path.len() - 1);
        let module_path = module_path
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>();

        let name = name.join("_");

        Ok(Self {
            name,
            path: module_path,
        })
    }

    pub fn parse_from_unsafe_ref(unsafe_ref: UnsafeRef) -> anyhow::Result<Self> {
        let s = unsafe_ref.split().to_owned();

        let full_name = s
            .last()
            .map(ToOwned::to_owned)
            .map(|s| s.unsafe_unwrap().replace("~1", "/"))
            .ok_or(anyhow::anyhow!("No name found in ref"))?;

        Self::parse_from_name(full_name)
    }
}

#[cfg(test)]
mod test {
    use crate::ir::Ref;

    #[test]
    fn test_parse_ref_normal() {
        let ref_ = Ref::parse_from_unsafe_ref(
            "#/definitions/liqwid~1types~1ActionValue"
                .to_string()
                .into(),
        )
        .unwrap();
        assert_eq!(ref_.name, "ActionValue");
        assert_eq!(ref_.path, vec!["liqwid".to_string(), "types".to_string(),]);
    }

    #[test]
    fn test_parse_name_normal() {
        let ref_ = Ref::parse_from_name("liqwid/types/ActionValue".to_string()).unwrap();
        assert_eq!(ref_.name, "ActionValue");
        assert_eq!(ref_.path, vec!["liqwid".to_string(), "types".to_string(),]);
    }

    #[test]
    fn test_no_prefix() {
        let ref_ = Ref::parse_from_unsafe_ref("liqwid~1types~1ActionValue".to_string().into())
            .expect("Failed to parse ref");

        assert_eq!(ref_.name, "ActionValue");
        assert_eq!(ref_.path, vec!["liqwid".to_string(), "types".to_string(),]);
    }
}

// These will inline into the generated structs
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Primitive {
    // Comes from dataType: "list"
    // Represented as `Vec<T>`
    List(Box<Primitive>),

    // Represented as `(A, B, C)`
    Tuple(Vec<Primitive>),

    // Comes from dataType: "map"
    // Represented as `Vec<(K, V)>`
    Map(Box<Primitive>, Box<Primitive>),

    // Recognize by "Optional" and "anyOf" field
    // Represented as `Option<T>`
    Option(Box<Primitive>),

    // Recognize by "Wrapped Redeemer" and "anyOf" field
    // Represented as `WrappedRedeemer<T>`
    WrappedRedeemer(Box<Primitive>),

    // Directly references a type
    // Represented as `T`
    Ref(Ref),

    // Represented as `pallas::AnyCbor`
    OpaqueData,

    // Represented as a bigint
    Int,

    // Represented as `pallas::Bytes`
    Bytes,
}

impl ToTokens for Primitive {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Primitive::OpaqueData => quote! { pallas::codec::utils::AnyCbor },
            Primitive::Int => quote! { pallas::codec::utils::AnyUInt },
            Primitive::Bytes => quote! { pallas::codec::utils::Bytes },
            Primitive::List(inner) => quote! { Vec<#inner> },
            Primitive::Tuple(inner) => quote! { (#(#inner),*) },
            Primitive::Map(inner, inner2) => quote! { Vec<(#inner, #inner2)> },
            Primitive::Option(inner) => quote! { Option<#inner> },
            // Would be nice to represent it somehow.
            Primitive::WrappedRedeemer(_) => quote! { pallas::codec::utils::AnyCbor },
            Primitive::Ref(ref_) => quote! { #ref_ },
        });
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Constructor {
    // pub struct Foo { a: usize, b: i32 };
    Named(Vec<(String, Primitive)>),

    // pub struct Foo(usize, i32);
    Unnamed(Vec<Primitive>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Definition {
    // enum A { A { a, b }, B { c, d } }
    Variant(HashMap<String, Primitive>),

    // struct A { a: A, b: B } // do decode with `Constr` tag
    TaggedConstructor(usize, Constructor),

    // struct Foo { a: A, b: B } // don't decode with `Constr` tag
    UntaggedConstructor(Vec<(String, Primitive)>),
}

// mod a {
//     pub mod c {
//         pub type Foo = ();
//     }
//     pub mod d {
//         type Z = (crate::a::c::Foo, i32);
//     }
// }

pub type Definitions = HashMap<Ref, Definition>;
pub type Primitives = HashMap<Ref, Primitive>;

pub fn convert_to_primitive(schema: &schema::TypeSchema) -> anyhow::Result<Primitive> {
    match schema {
        schema::TypeSchema::Variants { .. } => anyhow::bail!(
            "Variants cannot be converted to a primitive. There are no anonymous Sums in Rust."
        ),
        schema::TypeSchema::Reference {
            title: _,
            reference,
        } => Ok(Primitive::Ref(Ref::parse_from_unsafe_ref(
            reference.clone(),
        )?)),

        schema::TypeSchema::OpaqueData { .. } => Ok(Primitive::OpaqueData),
        schema::TypeSchema::Tagged(schema::TypeSchemaTagged::Int) => Ok(Primitive::Int),
        schema::TypeSchema::Tagged(schema::TypeSchemaTagged::Bytes) => Ok(Primitive::Bytes),
        schema::TypeSchema::Tagged(schema::TypeSchemaTagged::List { items }) => {
            match items {
                schema::ListItems::Monomorphic(items) => {
                    Ok(Primitive::List(Box::new(convert_to_primitive(items)?)))
                }
                schema::ListItems::Polymorphic(items) => {
                    Ok(Primitive::Tuple(items.iter().map(convert_to_primitive).collect::<Result<_, _>>()?))
                }
            }
        }
        schema::TypeSchema::Tagged(schema::TypeSchemaTagged::Constructor {
            ..
        }) => anyhow::bail!("Constructors cannot be converted to a primitive. There are no named anonymous Products in Rust."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_int() {
        // List<Int>
        let schema = schema::TypeSchema::Tagged(schema::TypeSchemaTagged::List {
            items: schema::ListItems::Monomorphic(Box::new(schema::TypeSchema::Tagged(
                schema::TypeSchemaTagged::Int,
            ))),
        });

        let primitive = convert_to_primitive(&schema).unwrap();

        assert_eq!(primitive, Primitive::List(Box::new(Primitive::Int)));
    }

    #[test]
    fn tuple_opaque_data_and_bytes() {
        // Tuple<OpaqueData, Bytes>
        let schema = schema::TypeSchema::Tagged(schema::TypeSchemaTagged::List {
            items: schema::ListItems::Polymorphic(vec![
                schema::TypeSchema::OpaqueData {
                    title: "OpaqueData".to_string().into(),
                    description: None,
                },
                schema::TypeSchema::Tagged(schema::TypeSchemaTagged::Bytes),
            ]),
        });

        let primitive = convert_to_primitive(&schema).unwrap();

        assert_eq!(
            primitive,
            Primitive::Tuple(vec![Primitive::OpaqueData, Primitive::Bytes])
        );
    }

    #[test]
    fn reference() {
        // Reference
        let schema = schema::TypeSchema::Reference {
            title: None,
            reference: "#/definitions/liqwid~1types~1ActionValue"
                .to_string()
                .into(),
        };

        let primitive = convert_to_primitive(&schema).unwrap();

        assert_eq!(
            primitive,
            Primitive::Ref(Ref::new(&["liqwid", "types"], "ActionValue"))
        );
    }
}

// `Primitives` are used later to inline in the generated structs.
// This will give you a HashMap that you can use when you want to refer to a particular ref.
pub fn collect_primitive_definitions(
    blueprint: &schema::BlueprintSchema,
) -> anyhow::Result<Primitives> {
    let mut primitives = HashMap::new();

    for (name, definition) in blueprint.definitions.iter() {
        let ref_ = Ref::parse_from_name(name.clone().unsafe_unwrap())?;

        match definition {
            schema::TypeSchema::Variants { .. } => {
                // Do not inline variants
                primitives.insert(ref_.clone(), Primitive::Ref(ref_));
            }
            schema::TypeSchema::Tagged(schema::TypeSchemaTagged::Constructor { .. }) => {
                // Do not inline constructors
                primitives.insert(ref_.clone(), Primitive::Ref(ref_));
            }

            schema::TypeSchema::Reference { .. } => {
                // Nothing to collect, reference inlining is moot.
            }
            schema::TypeSchema::Tagged(schema::TypeSchemaTagged::Int) => {
                primitives.insert(ref_.clone(), Primitive::Int);
            }
            schema::TypeSchema::Tagged(schema::TypeSchemaTagged::Bytes) => {
                primitives.insert(ref_.clone(), Primitive::Bytes);
            }
            schema::TypeSchema::OpaqueData { .. } => {
                primitives.insert(ref_.clone(), Primitive::OpaqueData);
            }
            schema::TypeSchema::Tagged(schema::TypeSchemaTagged::List { items }) => match items {
                schema::ListItems::Monomorphic(items) => {
                    let items = convert_to_primitive(items)?;
                    primitives.insert(ref_.clone(), Primitive::List(Box::new(items)));
                }
                schema::ListItems::Polymorphic(items) => {
                    let items = items
                        .iter()
                        .map(convert_to_primitive)
                        .collect::<Result<_, _>>()?;
                    primitives.insert(ref_.clone(), Primitive::Tuple(items));
                }
            },
            _ => todo!("Missing implementation for: {:?}", definition),
        }
    }

    Ok(primitives)
}

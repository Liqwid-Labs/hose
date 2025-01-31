use std::collections::HashMap;

use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};

use crate::reference::Ref;
use crate::safe_rename::{SafeRename, UnsafeName};
use crate::{cbor, schema};

trait Inline {
    fn inline_with(&self, primitives: &Primitives) -> anyhow::Result<Self>
    where
        Self: Sized;
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

impl Inline for Primitive {
    fn inline_with(&self, primitives: &Primitives) -> anyhow::Result<Self> {
        match self {
            Primitive::List(inner) => Ok(Primitive::List(Box::new(inner.inline_with(primitives)?))),
            Primitive::Tuple(inner) => Ok(Primitive::Tuple(
                inner
                    .iter()
                    .map(|inner| inner.inline_with(primitives))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Primitive::Map(inner, inner2) => Ok(Primitive::Map(
                Box::new(inner.inline_with(primitives)?),
                Box::new(inner2.inline_with(primitives)?),
            )),
            Primitive::Option(inner) => {
                Ok(Primitive::Option(Box::new(inner.inline_with(primitives)?)))
            }
            Primitive::WrappedRedeemer(inner) => Ok(Primitive::WrappedRedeemer(Box::new(
                inner.inline_with(primitives)?,
            ))),
            Primitive::Ref(ref_) => {
                if let Some(primitive) = primitives.get(ref_) {
                    Ok(primitive.clone())
                } else {
                    Ok(self.clone())
                }
            }
            Primitive::OpaqueData => Ok(Primitive::OpaqueData),
            Primitive::Int => Ok(Primitive::Int),
            Primitive::Bytes => Ok(Primitive::Bytes),
        }
    }
}

impl Primitive {
    pub fn super_all(&self) -> Self {
        match self {
            Primitive::OpaqueData => Primitive::OpaqueData,
            Primitive::Int => Primitive::Int,
            Primitive::Bytes => Primitive::Bytes,
            Primitive::List(inner) => Primitive::List(Box::new(inner.super_all())),
            Primitive::Tuple(inner) => {
                Primitive::Tuple(inner.iter().map(|k| k.super_all()).collect())
            }
            Primitive::Map(inner, inner2) => {
                Primitive::Map(Box::new(inner.super_all()), Box::new(inner2.super_all()))
            }
            Primitive::Option(inner) => Primitive::Option(Box::new(inner.super_all())),
            Primitive::WrappedRedeemer(inner) => {
                Primitive::WrappedRedeemer(Box::new(inner.super_all()))
            }
            Primitive::Ref(ref_) => Primitive::Ref(ref_.prepend_super()),
        }
    }
}

impl ToTokens for Primitive {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Primitive::OpaqueData => quote! { pallas::codec::utils::AnyCbor },
            Primitive::Int => quote! { hose_primitives::bigint::BigInt },
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

impl IntoIterator for Constructor {
    type Item = Primitive;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> std::vec::IntoIter<Self::Item> {
        match self {
            Constructor::Named(fields) => fields
                .into_iter()
                .map(|(_, k)| k)
                // TODO: Can we do this without allocating?
                .collect::<Vec<_>>()
                .into_iter(),
            Constructor::Unnamed(fields) => fields.into_iter(),
        }
    }
}

impl Constructor {
    pub fn super_all(&self) -> Self {
        match self {
            Constructor::Named(fields) => Constructor::Named(
                fields
                    .iter()
                    .map(|(s, k)| (s.clone(), k.super_all()))
                    .collect(),
            ),
            Constructor::Unnamed(fields) => {
                Constructor::Unnamed(fields.iter().map(|k| k.super_all()).collect())
            }
        }
    }
}

impl Inline for Constructor {
    fn inline_with(&self, primitives: &Primitives) -> anyhow::Result<Self> {
        match self {
            Constructor::Named(fields) => {
                let fields = fields
                    .clone()
                    .iter()
                    .map(|(name, primitive)| {
                        let primitive = primitive.inline_with(primitives)?;
                        Ok::<_, anyhow::Error>((name.clone(), primitive))
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(Constructor::Named(fields))
            }
            Constructor::Unnamed(fields) => {
                let fields = fields
                    .clone()
                    .iter()
                    .map(|primitive| primitive.inline_with(primitives))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(Constructor::Unnamed(fields))
            }
        }
    }
}

pub struct EnumConstructror(pub Constructor);

impl ToTokens for EnumConstructror {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match &self.0 {
            Constructor::Named(fields) => {
                let fields = fields.iter().map(|(name, primitive)| {
                    let name = format_ident!("{}", name.to_snake_case());
                    quote! {
                        #name: #primitive
                    }
                });

                quote! { { #(#fields),* } }
            }
            Constructor::Unnamed(fields) => {
                let fields = fields.iter().map(ToTokens::to_token_stream);

                quote! { ( #(#fields),* ) }
            }
        });
    }
}

impl ToTokens for Constructor {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Constructor::Named(fields) => {
                let fields = fields.iter().map(|(name, primitive)| {
                    let name = format_ident!("{}", name.to_snake_case());
                    quote! {
                        pub #name: #primitive
                    }
                });

                quote! { { #(#fields),* } }
            }
            Constructor::Unnamed(fields) => {
                let fields = fields.iter().map(|primitive| {
                    quote! {
                        pub #primitive
                    }
                });

                quote! { ( #(#fields),* ) }
            }
        });
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Definition {
    // enum A { A { a, b }, B { c, d } }
    Variant(HashMap<String, (usize, Constructor)>),

    // struct A { a: A, b: B } // do decode with `Constr` tag
    TaggedConstructor(usize, Constructor),

    // struct Foo { a: A, b: B } // don't decode with `Constr` tag
    UntaggedConstructor(Constructor),
}

impl Definition {
    pub fn super_all(&self) -> Self {
        match self {
            Definition::Variant(variants) => Definition::Variant(
                variants
                    .iter()
                    .map(|(s, k)| (s.clone(), (k.0, k.1.super_all())))
                    .collect(),
            ),
            Definition::TaggedConstructor(index, constructor) => {
                Definition::TaggedConstructor(*index, constructor.super_all())
            }
            Definition::UntaggedConstructor(constructor) => {
                Definition::UntaggedConstructor(constructor.super_all())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedDefinition {
    pub name: String,
    pub definition: Definition,
}

impl NamedDefinition {
    pub fn super_all(&self) -> Self {
        NamedDefinition {
            name: self.name.clone(),
            definition: self.definition.super_all(),
        }
    }
}

pub fn compute_cbor_tag(index: usize) -> u64 {
    if index < 7 {
        (121 + index).try_into().expect("Tag too large")
    } else if index < 128 {
        (1280 + index - 7).try_into().expect("Tag too large")
    } else {
        // See: https://github.com/aiken-lang/aiken/blob/6d2e38851eb9b14cf5ea04fdc4722405b5c1544a/crates/uplc/src/ast.rs#L437
        todo!("Constructors with more than 128 fields are not (yet) supported, you have {index}")
    }
}

impl ToTokens for NamedDefinition {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let definition_name = format_ident!("{}", self.name.clone());
        match &self.definition {
            Definition::Variant(variants) => {
                let cbor_impl = cbor::decode_encode_impl_enum(definition_name.clone(), variants);

                let variants = variants
                    .iter()
                    .map(|(name, (_, constructor))| {
                        let name = format_ident!("{}", name.to_upper_camel_case());
                        let constructor = EnumConstructror(constructor.clone());
                        // TODO: Give it a Tag in CBOR!
                        quote! {
                            #name #constructor
                        }
                    })
                    .collect::<Vec<_>>();

                let variants = quote! { { #(#variants),* } };

                tokens.extend(quote! {
                    #[derive(Debug, Clone, PartialEq, Eq)]
                    pub enum #definition_name #variants
                });

                tokens.extend(cbor_impl);
            }
            Definition::TaggedConstructor(index, constructor) => {
                let cbor_impl = cbor::decode_encode_impl_struct(
                    definition_name.clone(),
                    Some(*index),
                    constructor.clone(),
                );
                // TODO: Give it a Tag in CBOR!
                tokens.extend(quote! {
                    #[derive(Debug, Clone, PartialEq, Eq)]
                    pub struct #definition_name #constructor
                });
                if let Constructor::Unnamed(_) = constructor {
                    tokens.extend(quote! { ; });
                }

                tokens.extend(cbor_impl);
            }
            Definition::UntaggedConstructor(constructor) => {
                let cbor_impl = cbor::decode_encode_impl_struct(
                    definition_name.clone(),
                    None,
                    constructor.clone(),
                );
                tokens.extend(quote! {
                    #[derive(Debug, Clone, PartialEq, Eq)]
                    pub struct #definition_name #constructor
                });
                if let Constructor::Unnamed(_) = constructor {
                    tokens.extend(quote! { ; });
                }
                tokens.extend(cbor_impl);
            }
        }
    }
}

impl Inline for Definition {
    fn inline_with(&self, primitives: &Primitives) -> anyhow::Result<Self> {
        match self {
            Definition::Variant(variants) => {
                let variants = variants
                    .clone()
                    .into_iter()
                    .map(|(name, (index, constructor))| {
                        let constructor = constructor.inline_with(primitives)?;
                        Ok::<_, anyhow::Error>((name, (index, constructor)))
                    })
                    .collect::<Result<HashMap<_, _>, anyhow::Error>>()?;

                Ok(Definition::Variant(variants))
            }
            Definition::TaggedConstructor(index, constructor) => {
                let constructor = constructor.inline_with(primitives)?;
                Ok(Definition::TaggedConstructor(*index, constructor))
            }
            Definition::UntaggedConstructor(constructor) => {
                let constructor = constructor.inline_with(primitives)?;
                Ok(Definition::UntaggedConstructor(constructor))
            }
        }
    }
}

// mod a {
//     pub mod c {
//         pub type Foo = ();
//     }
//     pub mod d {
//         pub type Z(super::c::Foo, i32);
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
        schema::TypeSchema::Tagged(schema::TypeSchemaTagged::List { title: _, items }) => {
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
mod tests_convert {
    use super::*;

    #[test]
    fn list_int() {
        // List<Int>
        let schema = schema::TypeSchema::Tagged(schema::TypeSchemaTagged::List {
            title: None,
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
            title: None,
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
            schema::TypeSchema::Variants { title, any_of } => {
                if title.clone().safe_rename() == "Optional" && any_of.len() == 2 {
                    // Optional does get inlined using Rust type
                    let inner = any_of.first().expect("Misformed Optional");

                    if let schema::TypeSchema::Tagged(schema::TypeSchemaTagged::Constructor {
                        fields,
                        ..
                    }) = inner
                    {
                        let inner = fields.first().expect("Misformed Optional");

                        let inner = convert_to_primitive(inner)?;

                        primitives.insert(ref_.clone(), Primitive::Option(Box::new(inner)));
                    } else {
                        anyhow::bail!("Misformed Optional");
                    }
                } else {
                    // Do not inline variants
                    primitives.insert(ref_.clone(), Primitive::Ref(ref_));
                }
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
            schema::TypeSchema::Tagged(schema::TypeSchemaTagged::List { title, items }) => {
                match items {
                    schema::ListItems::Monomorphic(items) => {
                        let items = convert_to_primitive(items)?;
                        primitives.insert(ref_.clone(), Primitive::List(Box::new(items)));
                    }
                    schema::ListItems::Polymorphic(items) => {
                        if let Some(title) = title {
                            // Only inline Tuples.
                            //
                            // TODO: This actually doesn't work! We might need to either not inline
                            // them at all, or inline them as a custom tuple type.
                            // This same issue applies to `Optional`.
                            if title.clone().unsafe_unwrap() == "Tuple" {
                                let items = items
                                    .iter()
                                    .map(convert_to_primitive)
                                    .collect::<Result<_, _>>()?;

                                primitives.insert(ref_.clone(), Primitive::Tuple(items));
                            }
                        }
                    }
                }
            }
        }
    }

    // Proooobably shouldn't do it this way, but it works for now.
    for _ in 0..10 {
        let preexisting_primitives = primitives.clone();

        for (_, primitive) in primitives.iter_mut() {
            *primitive = primitive.inline_with(&preexisting_primitives).unwrap();
        }

        if preexisting_primitives.eq(&primitives) {
            return Ok(primitives);
        }
    }

    anyhow::bail!("Failed to inline primitives, reached max iterations");
}

pub fn type_schema_title(schema: &schema::TypeSchema) -> Option<UnsafeName> {
    match schema {
        schema::TypeSchema::OpaqueData { title, .. } => Some(title.clone()),
        schema::TypeSchema::Tagged(schema::TypeSchemaTagged::Constructor { title, .. }) => {
            Some(title.clone())
        }
        schema::TypeSchema::Tagged(schema::TypeSchemaTagged::Int { .. }) => None,
        schema::TypeSchema::Tagged(schema::TypeSchemaTagged::List { .. }) => None,
        schema::TypeSchema::Tagged(schema::TypeSchemaTagged::Bytes { .. }) => None,
        schema::TypeSchema::Variants { title, .. } => Some(title.clone()),
        schema::TypeSchema::Reference { title, .. } => title.clone(),
    }
}

pub fn schema_to_constructor(fields: Vec<schema::TypeSchema>) -> anyhow::Result<Constructor> {
    let all_fields_have_title = fields
        .iter()
        .all(|schema| type_schema_title(schema).is_some());
    if all_fields_have_title {
        let fields: Vec<Result<(String, Primitive), anyhow::Error>> = fields
            .iter()
            .map(|field| {
                let field_title = type_schema_title(field)
                    .expect("Need title in field, and we checked!")
                    .safe_rename();
                let field_type = field.clone();

                let field_type = convert_to_primitive(&field_type)?;

                Ok((field_title, field_type))
            })
            .collect::<Vec<_>>();

        let fields = fields.into_iter().collect::<Result<Vec<_>, _>>()?;

        Ok(Constructor::Named(fields))
    } else {
        let fields: Vec<Result<Primitive, anyhow::Error>> = fields
            .iter()
            .map(|field| {
                let field_type = field.clone();

                convert_to_primitive(&field_type)
            })
            .collect::<Vec<_>>();

        let fields = fields.into_iter().collect::<Result<Vec<_>, _>>()?;

        Ok(Constructor::Unnamed(fields))
    }
}

pub fn collect_definitions(blueprint: &schema::BlueprintSchema) -> anyhow::Result<Definitions> {
    let mut definitions = Definitions::new();

    let primitives = collect_primitive_definitions(blueprint)?;

    for (name, definition) in blueprint.definitions.clone() {
        match definition {
            schema::TypeSchema::Variants { title, any_of } => {
                let all_are_constructors = any_of.iter().all(|x| {
                    matches!(
                        x,
                        schema::TypeSchema::Tagged(schema::TypeSchemaTagged::Constructor { .. })
                    )
                });

                if !all_are_constructors {
                    todo!("Gracefully handle non-constructor variants");
                }

                if title.clone().safe_rename() == "Optional" {
                    // Optional does get inlined using Rust type
                    continue;
                }

                let mut variants = HashMap::<String, (usize, Constructor)>::new();

                match &any_of[..] {
                    // Special treatment for single-variant constructors, because they become
                    // `struct`, rather than `enum`.
                    [schema::TypeSchema::Tagged(schema::TypeSchemaTagged::Constructor {
                        // Constructor is already named by the top level title
                        title: _constructor_title,
                        index,
                        fields,
                    })] => {
                        let constructor = schema_to_constructor(fields.clone())?;
                        definitions.insert(
                            Ref::parse_from_name(name.clone().unsafe_unwrap())?,
                            Definition::TaggedConstructor(*index, constructor),
                        );
                    }
                    // All other variants become `enum`s.
                    _ => {
                        for constructor in any_of {
                            if let schema::TypeSchema::Tagged(
                                schema::TypeSchemaTagged::Constructor {
                                    title: constructor_title,
                                    index,
                                    fields,
                                },
                            ) = constructor
                            {
                                let constructor = schema_to_constructor(fields)?;

                                variants.insert(
                                    constructor_title.clone().safe_rename(),
                                    (index, constructor),
                                );
                            } else {
                                todo!("Gracefully handle non-constructor variants");
                            }
                        }

                        definitions.insert(
                            Ref::parse_from_name(name.clone().unsafe_unwrap())?,
                            Definition::Variant(variants),
                        );
                    }
                }
            }
            schema::TypeSchema::Tagged(schema::TypeSchemaTagged::List {
                title,
                items: schema::ListItems::Polymorphic(items),
            }) => {
                let constructor = schema_to_constructor(items)?;

                if let Some(title) = title {
                    if title.clone().safe_rename() == "Tuple" {
                        // Probably shouldn't use 'continue'. But we skip Tuples and inline them
                        // instead because they have bad names.
                        continue;
                    }
                }

                definitions.insert(
                    Ref::parse_from_name(name.clone().unsafe_unwrap())?,
                    Definition::UntaggedConstructor(constructor),
                );
            }
            schema::TypeSchema::Tagged(schema::TypeSchemaTagged::Constructor {
                title: _,
                index,
                fields,
            }) => {
                let constructor = schema_to_constructor(fields.clone())?;
                definitions.insert(
                    Ref::parse_from_name(name.clone().unsafe_unwrap())?,
                    Definition::TaggedConstructor(index, constructor),
                );
            }
            schema::TypeSchema::Tagged(
                schema::TypeSchemaTagged::List {
                    title: _,
                    items: schema::ListItems::Monomorphic(_),
                }
                | schema::TypeSchemaTagged::Int
                | schema::TypeSchemaTagged::Bytes,
            )
            | schema::TypeSchema::Reference { .. } => {
                // We don't need to do anything since these are inlined.
            }
            schema::TypeSchema::OpaqueData { .. } => {
                // We don't need to do anything since these are inlined.
            }
        }
    }

    let definitions = definitions
        .into_iter()
        .map(|(name, definition)| {
            let definition = definition.inline_with(&primitives)?;
            Ok::<_, anyhow::Error>((name, definition))
        })
        .collect::<Result<Definitions, _>>()?;

    Ok(definitions)
}

#[cfg(test)]
mod tests_collect_definitions {
    use schema::{BlueprintSchema, Preamble, TypeSchema};

    use super::*;

    #[test]
    fn structs() {
        let blueprint = BlueprintSchema {
            preamble: Preamble {
                title: "Preamble".to_string(),
                description: "".to_string(),
                version: "".to_string(),
                plutus_version: "1".to_string(),
                license: "".to_string(),
            },
            definitions: HashMap::from([
                (
                    "liqwid/types/ActionValue".to_string().into(),
                    TypeSchema::Tagged(schema::TypeSchemaTagged::Constructor {
                        title: "ActionValue".to_string().into(),
                        index: 0,
                        fields: Vec::from([
                            TypeSchema::Tagged(schema::TypeSchemaTagged::Int),
                            TypeSchema::Tagged(schema::TypeSchemaTagged::Int),
                        ]),
                    }),
                ),
                // Example List<Int> alias
                (
                    "liqwid/types/ListInt".to_string().into(),
                    TypeSchema::Tagged(schema::TypeSchemaTagged::List {
                        title: None,
                        items: schema::ListItems::Monomorphic(Box::new(TypeSchema::Tagged(
                            schema::TypeSchemaTagged::Int,
                        ))),
                    }),
                ),
                (
                    "liqwid/types/Action".to_string().into(),
                    TypeSchema::Variants {
                        title: "Action".to_string().into(),
                        any_of: Vec::from([
                            TypeSchema::Tagged(schema::TypeSchemaTagged::Constructor {
                                title: "ActionValue".to_string().into(),
                                index: 0,
                                fields: Vec::from([
                                    TypeSchema::Tagged(schema::TypeSchemaTagged::Int),
                                    TypeSchema::Reference {
                                        title: None,
                                        reference: "#/definitions/liqwid~1types~1ListInt"
                                            .to_string()
                                            .into(),
                                    },
                                ]),
                            }),
                            TypeSchema::Tagged(schema::TypeSchemaTagged::Constructor {
                                title: "AlsoActionValue".to_string().into(),
                                index: 1,
                                fields: Vec::from([
                                    TypeSchema::Tagged(schema::TypeSchemaTagged::Int),
                                    TypeSchema::Tagged(schema::TypeSchemaTagged::Int),
                                ]),
                            }),
                        ]),
                    },
                ),
            ]),
        };

        let definitions = collect_definitions(&blueprint).unwrap();

        assert_eq!(
            definitions[&Ref::new(&["liqwid", "types"], "ActionValue")].clone(),
            Definition::TaggedConstructor(
                0,
                Constructor::Unnamed(vec![Primitive::Int, Primitive::Int])
            )
        );

        // This should not be present in the definitions, since it's an alias
        assert!(!definitions.contains_key(&Ref::new(&["liqwid", "types"], "ListInt")));

        assert_eq!(
            definitions[&Ref::new(&["liqwid", "types"], "Action")].clone(),
            Definition::Variant(HashMap::from([
                (
                    "ActionValue".to_string(),
                    (
                        0,
                        Constructor::Unnamed(vec![
                            Primitive::Int,
                            Primitive::List(Box::new(Primitive::Int))
                        ])
                    )
                ),
                (
                    "AlsoActionValue".to_string(),
                    (
                        1,
                        Constructor::Unnamed(vec![Primitive::Int, Primitive::Int])
                    )
                ),
            ]))
        );
    }
}

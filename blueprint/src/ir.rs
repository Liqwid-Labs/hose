use std::collections::HashMap;

#[derive(Debug)]
pub struct Ref {
    pub path: Vec<String>,
    pub name: String,
}

// These will inline into the generated structs
#[derive(Debug)]
pub enum Primitive {
    // Comes from dataType: "list"
    List(Box<Primitive>),

    // (a, b, c)
    Tuple(Vec<Primitive>),

    // Comes from dataType: "map"
    Map(Box<Primitive>, Box<Primitive>),

    // Recognize by "Optional" and "anyOf" field
    Option(Box<Primitive>),

    // Recognize by "Wrapped Redeemer" and "anyOf" field
    WrappedRedeemer(Box<Primitive>),

    // Directly references a type
    Ref(Ref),

    OpaqueData,
}

#[derive(Debug)]
pub enum Constructor {
    // pub struct Foo { a: usize, b: i32 };
    Named(Vec<(String, Primitive)>),

    // pub struct Foo(usize, i32);
    Unnamed(Vec<Primitive>),
}

#[derive(Debug)]
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

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::safe_rename::{UnsafeName, UnsafeRef};

#[derive(Debug, Serialize, Deserialize)]
pub struct BlueprintSchema {
    pub preamble: Preamble,
    pub definitions: HashMap<UnsafeName, TypeSchema>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preamble {
    pub title: String,
    pub description: String,
    pub version: String,
    #[serde(rename = "plutusVersion")]
    pub plutus_version: String,
    pub license: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum TypeSchema {
    Variants {
        title: UnsafeName,
        #[serde(rename = "anyOf")]
        any_of: Vec<TypeSchema>,
    },
    Reference {
        title: Option<UnsafeName>,
        #[serde(rename = "$ref")]
        reference: UnsafeRef,
    },
    Tagged(TypeSchemaTagged),
    OpaqueData {
        title: UnsafeName,
        description: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ListItems {
    Monomorphic(Box<TypeSchema>),
    Polymorphic(Vec<TypeSchema>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "dataType")]
pub enum TypeSchemaTagged {
    #[serde(rename = "integer")]
    Int,
    #[serde(rename = "bytes")]
    Bytes,
    #[serde(rename = "list")]
    List {
        #[serde(rename = "items")]
        items: ListItems,
    },
    #[serde(rename = "constructor")]
    Constructor {
        title: UnsafeName,
        // This represents the index of the field in the constructor
        // e.g. in Plutus Data, it looks like this:
        // Data::Constr($constr, $fields)
        index: usize,
        fields: Vec<TypeSchema>,
    },
}

impl From<TypeSchemaTagged> for TypeSchema {
    fn from(tagged: TypeSchemaTagged) -> Self {
        TypeSchema::Tagged(tagged)
    }
}

#[cfg(test)]
mod tests {
    use super::{BlueprintSchema, TypeSchema, TypeSchemaTagged};

    #[test]
    fn test_parse_blueprint_schema() {
        let contents = include_str!("../plutus.json");

        println!("{}", contents);
        let schema: BlueprintSchema = serde_json::from_str(contents).unwrap();

        println!("{:#?}", schema);

        assert_eq!(schema.preamble.title, "liqwid-labs/hello_world");
        assert_eq!(schema.preamble.version, "0.0.0");

        assert_eq!(
            schema.definitions[&"liqwid/ActionValue".to_string().into()],
            TypeSchema::Variants {
                title: "ActionValue".to_string().into(),
                any_of: vec![TypeSchema::Tagged(TypeSchemaTagged::Constructor {
                    title: "ActionValue".to_string().into(),
                    index: 0,
                    fields: vec![
                        TypeSchema::Reference {
                            title: Some("supplyDiff".to_string().into()),
                            reference: "#/definitions/Int".to_string().into(),
                        },
                        TypeSchema::Reference {
                            title: Some("qTokensDiff".to_string().into()),
                            reference: "#/definitions/Int".to_string().into(),
                        },
                        TypeSchema::Reference {
                            title: Some("principalDiff".to_string().into()),
                            reference: "#/definitions/Int".to_string().into(),
                        },
                        TypeSchema::Reference {
                            title: Some("interestDiff".to_string().into()),
                            reference: "#/definitions/Int".to_string().into(),
                        },
                        TypeSchema::Reference {
                            title: Some("extraInterestRepaid".to_string().into()),
                            reference: "#/definitions/Int".to_string().into(),
                        },
                    ],
                }),]
            }
        );
    }
}

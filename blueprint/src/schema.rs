use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BlueprintSchema {
    pub preamble: Preamble,
    pub definitions: HashMap<String, TypeSchema>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Preamble {
    pub title: String,
    pub description: String,
    pub version: String,
    #[serde(rename = "plutusVersion")]
    pub plutus_version: String,
    pub license: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum TypeSchema {
    Variants {
        title: String,
        #[serde(rename = "anyOf")]
        any_of: Vec<TypeSchema>,
    },
    Reference {
        title: String,
        #[serde(rename = "$ref")]
        reference: String,
    },
    Tagged(TypeSchemaTagged),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "dataType")]
pub enum TypeSchemaTagged {
    #[serde(rename = "integer")]
    Int,
    #[serde(rename = "bytes")]
    Bytes,
    #[serde(rename = "constructor")]
    Constructor {
        title: String,
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
        let contents = include_str!("../../blueprint/plutus.json");

        println!("{}", contents);
        let schema: BlueprintSchema = serde_json::from_str(contents).unwrap();

        println!("{:#?}", schema);

        assert_eq!(schema.preamble.title, "liqwid-labs/hello_world");
        assert_eq!(schema.preamble.version, "0.0.0");

        assert_eq!(
            schema.definitions["liqwid/ActionValue"],
            TypeSchema::Variants {
                title: "ActionValue".to_string(),
                any_of: vec![TypeSchema::Tagged(TypeSchemaTagged::Constructor {
                    title: "ActionValue".to_string(),
                    index: 0,
                    fields: vec![
                        TypeSchema::Reference {
                            title: "supplyDiff".to_string(),
                            reference: "#/definitions/Int".to_string(),
                        },
                        TypeSchema::Reference {
                            title: "qTokensDiff".to_string(),
                            reference: "#/definitions/Int".to_string(),
                        },
                        TypeSchema::Reference {
                            title: "principalDiff".to_string(),
                            reference: "#/definitions/Int".to_string(),
                        },
                        TypeSchema::Reference {
                            title: "interestDiff".to_string(),
                            reference: "#/definitions/Int".to_string(),
                        },
                        TypeSchema::Reference {
                            title: "extraInterestRepaid".to_string(),
                            reference: "#/definitions/Int".to_string(),
                        },
                    ],
                }),]
            }
        );
    }
}

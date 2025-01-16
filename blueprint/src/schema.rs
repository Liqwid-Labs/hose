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

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "dataType")]
pub enum TypeSchemaTagged {
    #[serde(rename = "integer")]
    Int,
    #[serde(rename = "bytes")]
    Bytes,
    #[serde(rename = "constructor")]
    Constructor {
        title: String,
        fields: Vec<TypeSchema>,
    },
}

mod tests {
    use super::*;

    #[test]
    fn test_parse_blueprint_schema() {
        let contents = include_str!("../../blueprint/plutus.json");

        println!("{}", contents);
        let schema: BlueprintSchema = serde_json::from_str(contents).unwrap();

        println!("{:#?}", schema);

        assert_eq!(schema.preamble.title, "Liqwid");
        assert_eq!(schema.preamble.version, "1.0.0");
    }
}

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct BlueprintSchema {
    pub preamble: Preamble,
    pub definitions: HashMap<String, TypeSchema>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Preamble {
    title: String,
    description: String,
    version: String,
    #[serde(rename = "plutusVersion")]
    plutus_version: String,
    license: String,
}

#[derive(Debug, Serialize, Deserialize)]
enum DataType {
    Integer,
    Bytes,
}

struct DataTypeSchema {}

// "liqwid/ActionValue": {
//   "title": "ActionValue",
//   "anyOf": [
//     {
//       "title": "ActionValue",
//       "dataType": "constructor",
//       "index": 0,
//       "fields": [
//         {
//           "title": "supplyDiff",
//           "$ref": "#/definitions/Int"
//         },
//         {
//           "title": "qTokensDiff",
//           "$ref": "#/definitions/Int"
//         },
//         {
//           "title": "principalDiff",
//           "$ref": "#/definitions/Int"
//         },
//         {
//           "title": "interestDiff",
//           "$ref": "#/definitions/Int"
//         },
//         {
//           "title": "extraInterestRepaid",
//           "$ref": "#/definitions/Int"
//         }
//       ]
//     }
//   ]
// },

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum TypeSchema {
    OpaqueData {
        title: String,
        #[serde(rename = "anyOf")]
        any_of: Vec<TypeSchema>,
    },
    Tagged(TypeSchemaTagged),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "dataType")]
enum TypeSchemaTagged {
    #[serde(rename = "integer")]
    Int { data_type: String },
    #[serde(rename = "constructor")]
    Constructor { data_type: String, title: String },
}

// #[derive(Debug, Serialize, Deserialize)]
// enum Definition {
//     Struct {
//         title: Option<String>,
//         #[serde(rename = "anyOf")]
//         any_of: Vec<Constructor>,
//     },
// }
//
// #[derive(Debug, Serialize, Deserialize)]
// struct Constructor {
//     title: Option<String>,
//     data_type: DataType,
//     index: usize,
//     fields: Vec<Field>,
// }
//
// #[derive(Debug, Serialize, Deserialize)]
// struct Field {
//     title: Option<String>,
//     #[serde(rename = "$ref")]
//     ref_: String,
// }

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

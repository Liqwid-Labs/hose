use serde::{Deserialize, Serialize};

pub const INVALID_CHARACTERS: &[char] = &['-', '.', '$', '[', ']', '/', '#'];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
// Represents a string that is not yet verified as safe to use as a Rust identifier, and is likely
// a reference to a type (i.e. prefixed with a `#/`)
pub struct UnsafeRef(String);

impl From<String> for UnsafeRef {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl UnsafeRef {
    pub fn split(self) -> impl Iterator<Item = UnsafeName> {
        self.0
            .split('/')
            .map(|s| UnsafeName(s.to_string()))
            .collect::<Vec<_>>()
            .into_iter()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
// Represents a string that is not yet verified as safe to use as a Rust identifier
pub struct UnsafeName(String);

impl From<String> for UnsafeName {
    fn from(name: String) -> Self {
        Self(name)
    }
}

pub trait SafeRename {
    fn safe_rename(self) -> String;
}

impl SafeRename for UnsafeName {
    fn safe_rename(self) -> String {
        safe_rename_special_identifiers(self.0.replace("~1", "/").replace(INVALID_CHARACTERS, "_"))
    }
}

pub const REPLACE_SPECIAL_IDENTS: &[(&str, &str)] = &[
    ("Self", "AikenSelf"),
    ("Option", "AikenOption"),
    ("Optional", "AikenOptional"),
    ("List", "AikenList"),
    ("Int", "AikenInt"),
    ("Bool", "AikenBool"),
    ("ByteArray", "AikenByteArray"),
    ("Data", "AikenData"),
];

fn safe_rename_special_identifiers(ident: String) -> String {
    for (from, to) in REPLACE_SPECIAL_IDENTS {
        if ident == *from {
            return to.to_string();
        }
    }
    ident
}

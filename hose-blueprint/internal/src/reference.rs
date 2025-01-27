use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};

use crate::safe_rename::{self, SafeRename, UnsafeRef};

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

    pub fn is_sub_module_of(&self, path: &[String]) -> bool {
        self.path.starts_with(path)
    }

    pub fn drop_top_module(&self) -> Self {
        Self {
            path: self.path[1..].to_vec(),
            name: self.name.clone(),
        }
    }

    pub fn is_top_module(&self) -> bool {
        self.path.is_empty()
    }

    pub fn prepend_super(&self) -> Self {
        let mut new_path = Vec::from(["super".to_string()]);
        new_path.extend(self.path.clone());
        Self {
            path: new_path,
            name: self.name.clone(),
        }
    }
}

impl ToTokens for Ref {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let path = self
            .path
            .clone()
            .into_iter()
            .map(|p| format_ident!("{}", p));
        let name = format_ident!("{}", self.name.clone());
        if self.path.is_empty() {
            tokens.extend(quote! { #name });
        } else {
            tokens.extend(quote! { #(#path)::* :: #name });
        }
    }
}

impl Ref {
    pub fn parse_from_name(full_name: String) -> anyhow::Result<Self> {
        if full_name.contains("$") {
            // If the identifier contains a dollar sign, it means it's a polymorphic type.
            // We can't yet parse those properly. See the open issue:
            // https://github.com/aiken-lang/aiken/issues/1087
            //
            // For now, we just give it the full name, splitting on the first dollar sign.

            let (module_path, name) = full_name.split_once("$").unwrap();

            let module_path = module_path
                .split("/")
                .map(String::from)
                .collect::<Vec<String>>();

            let (init_name, module_path) = module_path.split_last().unwrap();

            let name = format!("{}_{}", init_name, name);

            return Ok(Self {
                path: module_path.to_vec(),
                name: safe_rename::UnsafeName::from(name.to_string()).safe_rename(),
            });
        }
        let full_module_path = full_name
            .split("/")
            .map(String::from)
            .collect::<Vec<String>>();

        // Last is name, everything else is module path
        let (module_path, name) = full_module_path.split_at(full_module_path.len() - 1);
        let module_path = module_path
            .iter()
            .map(|x| safe_rename::UnsafeName::from(x.to_string()).safe_rename())
            .collect::<Vec<String>>();

        let name = name.join("_");

        let name = safe_rename::UnsafeName::from(name).safe_rename();

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
mod tests_ref {
    use crate::reference::Ref;

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

use std::collections::{HashMap, HashSet};

use quote::{format_ident, quote, ToTokens};

use crate::{
    ir::{Definition, Definitions, NamedDefinition},
    reference::Ref,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub name: String,
    pub to_top_level: usize,
    pub sub_modules: Vec<Module>,
    pub definitions: Vec<NamedDefinition>,
}

impl Module {
    pub fn super_all(&self) -> Self {
        Module {
            name: self.name.clone(),
            to_top_level: self.to_top_level,
            sub_modules: self.sub_modules.iter().map(|k| k.super_all()).collect(),
            definitions: self.definitions.iter().map(|k| k.super_all()).collect(),
        }
    }

    pub fn from_definitions(definitions: &Definitions) -> Self {
        compute_module(definitions, 0)
    }
}

impl ToTokens for Module {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = format_ident!("{}", self.name);
        let sub_modules = self.sub_modules.clone();
        let definitions = self.definitions.clone();
        let module = quote! {
            pub mod #name {
                #(#sub_modules)*
                #(#definitions)*
            }
        };
        tokens.extend(module);
    }
}

fn module_names(definitions: &Definitions) -> HashSet<String> {
    let mut module_names = HashSet::new();
    for ref_ in definitions.keys() {
        if !ref_.is_top_module() {
            module_names.insert(
                ref_.path
                    .clone()
                    .first()
                    .expect("We checked that it's not empty")
                    .clone(),
            );
        }
    }
    module_names
}

// Better implemented as
fn sub_module_defs(definitions: &Definitions, module_name: &str) -> Definitions {
    definitions
        .iter()
        .filter(|(ref_, _)| ref_.is_sub_module_of(&[module_name.to_string()]))
        .map(|(ref_, _)| (ref_.drop_top_module(), definitions[ref_].clone()))
        .collect()
}

fn compute_module(definitions: &Definitions, level: usize) -> Module {
    let mut submodules = Vec::new();

    for sub_module_name in module_names(definitions) {
        let sub_module_defs: HashMap<Ref, Definition> =
            sub_module_defs(definitions, &sub_module_name)
                .into_iter()
                .map(|(k, v)| (k.clone(), v.super_all()))
                .collect();

        let named_definitions = sub_module_defs
            .iter()
            .filter(|(ref_, _)| ref_.is_top_module())
            .map(|(ref_, def)| NamedDefinition {
                name: ref_.name.clone(),
                definition: def.clone(),
            })
            .collect::<Vec<_>>();

        let sub_modules = compute_module(&sub_module_defs, level + 1);

        submodules.push(Module {
            name: sub_module_name.clone(),
            to_top_level: level,
            sub_modules: sub_modules.sub_modules,
            definitions: named_definitions,
        });
    }

    let top_level_defs = definitions
        .iter()
        .filter(|(ref_, _)| ref_.is_top_module())
        .map(|(ref_, def)| NamedDefinition {
            name: ref_.name.clone(),
            definition: def.clone(),
        })
        .collect::<Vec<_>>();

    Module {
        name: "root".to_string(),
        to_top_level: level,
        sub_modules: submodules,
        definitions: top_level_defs,
    }
}

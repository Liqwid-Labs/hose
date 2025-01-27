use std::env;

use hose_blueprint_internal::{ir::collect_definitions, module::Module, schema::BlueprintSchema};

use proc_macro2::TokenStream as TokenStream2;

use quote::ToTokens;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Usage: hose-blueprint <path to plutus.json>");
    }

    let path = args.get(1).unwrap();

    let blueprint = BlueprintSchema::from_file(path).unwrap();

    let definitions = collect_definitions(&blueprint).unwrap();

    let modules = Module::from_definitions(&definitions);

    let mut tokens = TokenStream2::new();

    modules.to_tokens(&mut tokens);

    let pretty_tokens = pretty::bat_pretty_print(&mut tokens).unwrap();

    println!("{}", pretty_tokens);
}

// Stolen from https://github.com/Michael-F-Bryan/scad-rs/blob/4dbff0c30ce991105f1e649e678d68c2767e894b/crates/codegen/src/pretty_print.rs#L8-L22
pub mod pretty {
    use super::*;
    use std::io::Write;
    use std::process::{Command, Output, Stdio};

    pub fn pretty_print(tokens: impl ToTokens) -> anyhow::Result<String> {
        let tokens = tokens.into_token_stream().to_string();

        let mut child = Command::new("rustfmt")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        {
            let mut stdin = child.stdin.take().unwrap();
            stdin.write_fmt(core::format_args!("{tokens}"))?;
            stdin.flush()?;
        }

        let Output {
            status,
            stdout,
            stderr,
        } = child.wait_with_output()?;
        let stdout = String::from_utf8_lossy(&stdout);
        let stderr = String::from_utf8_lossy(&stderr);

        if !status.success() {
            eprintln!("---- Stdout ----");
            eprintln!("{stdout}");
            eprintln!("---- Stderr ----");
            eprintln!("{stderr}");
            let code = status.code();
            match code {
                Some(code) => anyhow::bail!("The `rustfmt` command failed with return code {code}"),
                None => anyhow::bail!("The `rustfmt` command failed"),
            }
        }

        Ok(stdout.into())
    }

    pub fn bat_pretty_print(tokens: impl ToTokens) -> anyhow::Result<String> {
        let pretty_printed = pretty_print(tokens)?;

        let mut child = Command::new("bat")
            .arg("--language=rust")
            .arg("--style=plain")
            .arg("--color=always")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        {
            let mut stdin = child.stdin.take().unwrap();
            stdin.write_fmt(core::format_args!("{pretty_printed}"))?;
            stdin.flush()?;
        }

        let Output {
            status,
            stdout,
            stderr,
        } = child.wait_with_output()?;
        let stdout = String::from_utf8_lossy(&stdout);
        let stderr = String::from_utf8_lossy(&stderr);

        if !status.success() {
            eprintln!("---- Stdout ----");
            eprintln!("{stdout}");
            eprintln!("---- Stderr ----");
            eprintln!("{stderr}");
            let code = status.code();
            match code {
                Some(code) => anyhow::bail!("The `bat` command failed with return code {code}"),
                None => anyhow::bail!("The `bat` command failed"),
            }
        }

        Ok(stdout.into())
    }
}

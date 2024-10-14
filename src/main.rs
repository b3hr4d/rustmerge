mod test;

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use anyhow::{Context, Result};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::File;
use syn::{Item, ItemMod};

struct ModuleInfo {
    content: TokenStream,
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args[1] != "rustmerge" {
        eprintln!("Usage: cargo rustmerge [<package_name>]");
        process::exit(1);
    }

    let current_dir = env::current_dir().context("Failed to get current directory")?;
    let (package_name, package_path) = determine_package(&current_dir, &args)?;
    let src_dir = find_src_dir(&package_path)?;
    let output_file = create_output_file(&current_dir, &package_name)?;

    let module_structure = parse_module_structure(&src_dir)?;
    let merged_content = process_package(&src_dir, &module_structure)?;

    fs::write(&output_file, merged_content.to_string())?;
    println!("Merged Rust program created in {:?}", output_file);
    println!("File size: {} bytes", fs::metadata(&output_file)?.len());

    Ok(())
}

fn determine_package(current_dir: &Path, args: &[String]) -> Result<(String, PathBuf)> {
    let cargo_toml = current_dir.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml)?;
    let parsed_toml: toml::Value = toml::from_str(&content)?;

    if let Some(workspace) = parsed_toml.get("workspace") {
        let members = workspace
            .get("members")
            .and_then(|m| m.as_array())
            .context("Failed to get workspace members")?;

        if args.len() > 2 {
            let package_name = &args[2];
            if members.iter().any(|m| m.as_str() == Some(package_name)) {
                Ok((package_name.clone(), current_dir.join(package_name)))
            } else {
                Err(anyhow::anyhow!(
                    "Package '{}' not found in workspace",
                    package_name
                ))
            }
        } else {
            println!("This is a workspace. Available packages:");
            for (i, member) in members.iter().enumerate() {
                println!("{}. {}", i + 1, member.as_str().unwrap());
            }
            println!("Please run the command again with the package name.");
            process::exit(1);
        }
    } else {
        // Single package project
        parsed_toml
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .map(|name| (name.to_string(), current_dir.to_path_buf()))
            .context("Failed to determine package name")
    }
}

fn find_src_dir(package_path: &Path) -> Result<PathBuf> {
    let cargo_toml = package_path.join("Cargo.toml");
    let content = fs::read_to_string(cargo_toml)?;
    let parsed_toml: toml::Value = toml::from_str(&content)?;

    parsed_toml
        .get("package")
        .and_then(|p| p.get("src"))
        .and_then(|s| s.as_str())
        .map(|src| package_path.join(src))
        .or_else(|| Some(package_path.join("src")))
        .context("Failed to find src directory")
}

fn create_output_file(current_dir: &Path, package_name: &str) -> Result<PathBuf> {
    let output_file = current_dir
        .join("target")
        .join(format!("{}_merged.rs", package_name));
    fs::create_dir_all(output_file.parent().unwrap())?;
    Ok(output_file)
}

fn parse_module_structure(src_dir: &Path) -> Result<HashMap<String, ModuleInfo>> {
    let mut module_structure = HashMap::new();

    let root_file_path = if src_dir.join("lib.rs").exists() {
        src_dir.join("lib.rs")
    } else if src_dir.join("main.rs").exists() {
        src_dir.join("main.rs")
    } else {
        return Err(anyhow::anyhow!(
            "Neither lib.rs nor main.rs found in the src directory"
        ));
    };

    parse_file_and_submodules(&root_file_path, "crate", &mut module_structure)?;

    Ok(module_structure)
}

fn parse_file_and_submodules(
    file_path: &Path,
    module_path: &str,
    module_structure: &mut HashMap<String, ModuleInfo>,
) -> Result<()> {
    let content = fs::read_to_string(file_path)?;
    let file: File = syn::parse_file(&content)?;

    let tokens = parse_with_cfg_items(&file);

    module_structure.insert(
        module_path.to_string(),
        ModuleInfo {
            content: tokens.clone(),
        },
    );

    parse_module_items(&file.items, file_path, module_path, module_structure)?;

    Ok(())
}

fn parse_with_cfg_items(file: &File) -> TokenStream {
    let mut tokens = TokenStream::new();
    for item in &file.items {
        if !is_test_module(item) {
            match item {
                Item::Mod(item_mod) => {
                    let cfg_attrs = item_mod
                        .attrs
                        .iter()
                        .filter(|attr| attr.path.is_ident("cfg"))
                        .cloned()
                        .collect::<Vec<_>>();

                    if !cfg_attrs.is_empty() {
                        tokens.extend(quote::quote! {
                            #(#cfg_attrs)*
                        });
                    }
                    item_mod.to_tokens(&mut tokens);
                }
                _ => item.to_tokens(&mut tokens),
            }
        }
    }
    tokens
}

fn is_test_module(item: &Item) -> bool {
    if let Item::Mod(item_mod) = item {
        item_mod.ident == "test" || item_mod.ident == "tests"
    } else {
        false
    }
}

fn parse_module_items(
    items: &[Item],
    file_path: &Path,
    module_path: &str,
    module_structure: &mut HashMap<String, ModuleInfo>,
) -> Result<()> {
    for item in items {
        if let Item::Mod(item_mod) = item {
            if !is_test_module(item) {
                let submodule_name = item_mod.ident.to_string();
                let submodule_path = if module_path == "crate" {
                    submodule_name.clone()
                } else {
                    format!("{}::{}", module_path, submodule_name)
                };

                let cfg_attrs = item_mod
                    .attrs
                    .iter()
                    .filter(|attr| attr.path.is_ident("cfg"))
                    .cloned()
                    .collect::<Vec<_>>();

                if let Some((_, items)) = &item_mod.content {
                    let submodule_tokens = quote::quote! {
                        #(#cfg_attrs)*
                        #item_mod
                    };
                    module_structure.insert(
                        submodule_path.clone(),
                        ModuleInfo {
                            content: submodule_tokens,
                        },
                    );
                    parse_module_items(items, file_path, &submodule_path, module_structure)?;
                } else {
                    let parent = file_path
                        .parent()
                        .context("Failed to get parent directory")?;
                    let file_module_path = parent.join(&submodule_name).with_extension("rs");
                    let dir_module_path = parent.join(&submodule_name).join("mod.rs");

                    if file_module_path.exists() {
                        parse_file_and_submodules(
                            &file_module_path,
                            &submodule_path,
                            module_structure,
                        )?;
                    } else if dir_module_path.exists() {
                        parse_file_and_submodules(
                            &dir_module_path,
                            &submodule_path,
                            module_structure,
                        )?;
                    } else {
                        let empty_mod = quote::quote! {
                            #(#cfg_attrs)*
                            pub mod #submodule_name {}
                        };
                        module_structure
                            .insert(submodule_path.clone(), ModuleInfo { content: empty_mod });
                    }
                }
            }
        }
    }
    Ok(())
}

fn process_package(
    src_dir: &Path,
    module_structure: &HashMap<String, ModuleInfo>,
) -> Result<TokenStream> {
    let mut merged_content = quote! {
        // Package: #package_name
        // This file was automatically generated by cargo-rustmerge
    };

    let root_module = if src_dir.join("lib.rs").exists() {
        "crate"
    } else if src_dir.join("main.rs").exists() {
        "crate"
    } else {
        return Err(anyhow::anyhow!(
            "Neither lib.rs nor main.rs found in the src directory"
        ));
    };

    process_module(root_module, module_structure, &mut merged_content)?;

    Ok(merged_content)
}

fn process_module(
    module_path: &str,
    module_structure: &HashMap<String, ModuleInfo>,
    output: &mut TokenStream,
) -> Result<()> {
    if let Some(module_info) = module_structure.get(module_path) {
        let file = syn::parse_file(&module_info.content.to_string())?;

        for item in file.items {
            if !is_test_module(&item) {
                match item {
                    Item::Mod(ItemMod { ident, content, .. }) => {
                        let submodule_path = if module_path == "crate" {
                            ident.to_string()
                        } else {
                            format!("{}::{}", module_path, ident)
                        };

                        let mut submodule_content = TokenStream::new();
                        process_module(&submodule_path, module_structure, &mut submodule_content)?;

                        let expanded = if submodule_content.is_empty() && content.is_none() {
                            quote! {
                                pub mod #ident;
                            }
                        } else {
                            quote! {
                                pub mod #ident {
                                    #submodule_content
                                }
                            }
                        };
                        expanded.to_tokens(output);
                    }
                    _ => item.to_tokens(output),
                }
            }
        }
    }

    Ok(())
}

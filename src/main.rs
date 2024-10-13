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
    path: PathBuf,
    content: TokenStream,
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args[1] != "rustmerge" {
        eprintln!("Usage: cargo rustmerge [<package_name>]");
        process::exit(1);
    }

    let current_dir = env::current_dir().context("Failed to get current directory")?;
    let package_name = determine_package_name(&current_dir, &args)?;
    let src_dir = find_src_dir(&current_dir)?;
    println!("Source directory: {:?}", src_dir);
    let output_file = create_output_file(&current_dir, &package_name)?;

    let module_structure = parse_module_structure(&src_dir)?;
    println!("Module structure keys: {:?}", module_structure.keys());

    let merged_content = process_package(&src_dir, &package_name, &module_structure)?;

    println!("Merged content:\n{}", merged_content.to_string());

    fs::write(&output_file, merged_content.to_string())?;
    println!("Merged Rust program created in {:?}", output_file);
    println!("File size: {} bytes", fs::metadata(&output_file)?.len());

    Ok(())
}

fn determine_package_name(current_dir: &Path, args: &[String]) -> Result<String> {
    if args.len() > 2 {
        Ok(args[2].clone())
    } else {
        let cargo_toml = current_dir.join("Cargo.toml");
        let content = fs::read_to_string(cargo_toml)?;
        let parsed_toml: toml::Value = toml::from_str(&content)?;
        parsed_toml
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .map(String::from)
            .context("Failed to determine package name")
    }
}

fn find_src_dir(current_dir: &Path) -> Result<PathBuf> {
    let cargo_toml = current_dir.join("Cargo.toml");
    let content = fs::read_to_string(cargo_toml)?;
    let parsed_toml: toml::Value = toml::from_str(&content)?;

    parsed_toml
        .get("package")
        .and_then(|p| p.get("src"))
        .and_then(|s| s.as_str())
        .map(|src| current_dir.join(src))
        .or_else(|| Some(current_dir.join("src")))
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

    // Handle the root file (main.rs or lib.rs)
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

    println!("Module structure: {:?}", module_structure.keys());
    Ok(module_structure)
}

fn parse_file_and_submodules(
    file_path: &Path,
    module_path: &str,
    module_structure: &mut HashMap<String, ModuleInfo>,
) -> Result<()> {
    println!("Parsing file: {:?}", file_path);
    let content = fs::read_to_string(file_path)?;
    let file: File = syn::parse_file(&content)?;

    // Custom parsing to include cfg attributes
    let tokens = parse_with_cfg_items(&file);

    module_structure.insert(
        module_path.to_string(),
        ModuleInfo {
            path: file_path.to_path_buf(),
            content: tokens.clone(),
        },
    );

    parse_module_items(&file.items, file_path, module_path, module_structure)?;

    Ok(())
}

fn parse_with_cfg_items(file: &File) -> TokenStream {
    let mut tokens = TokenStream::new();
    for item in &file.items {
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
    tokens
}

fn parse_module_items(
    items: &[Item],
    file_path: &Path,
    module_path: &str,
    module_structure: &mut HashMap<String, ModuleInfo>,
) -> Result<()> {
    for item in items {
        if let Item::Mod(item_mod) = item {
            let submodule_name = item_mod.ident.to_string();
            let submodule_path = if module_path == "crate" {
                submodule_name.clone()
            } else {
                format!("{}::{}", module_path, submodule_name)
            };

            // Include cfg attributes in the module content
            let cfg_attrs = item_mod
                .attrs
                .iter()
                .filter(|attr| attr.path.is_ident("cfg"))
                .cloned()
                .collect::<Vec<_>>();

            if let Some((_, items)) = &item_mod.content {
                // Inline module
                let submodule_tokens = quote::quote! {
                    #(#cfg_attrs)*
                    #item_mod
                };
                module_structure.insert(
                    submodule_path.clone(),
                    ModuleInfo {
                        path: file_path.to_path_buf(),
                        content: submodule_tokens,
                    },
                );
                // Recursively parse inline submodules
                parse_module_items(items, file_path, &submodule_path, module_structure)?;
            } else {
                // Check for external module file or directory
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
                    parse_file_and_submodules(&dir_module_path, &submodule_path, module_structure)?;
                } else {
                    println!("Warning: Module not found: {}", submodule_name);
                    // Add an empty module to preserve the structure, including cfg attributes
                    let empty_mod = quote::quote! {
                        #(#cfg_attrs)*
                        pub mod #submodule_name {}
                    };
                    module_structure.insert(
                        submodule_path.clone(),
                        ModuleInfo {
                            path: PathBuf::new(),
                            content: empty_mod,
                        },
                    );
                }
            }
        }
    }
    Ok(())
}

fn process_package(
    src_dir: &Path,
    package_name: &str,
    module_structure: &HashMap<String, ModuleInfo>,
) -> Result<TokenStream> {
    println!("Processing package: {}", package_name);
    println!("Source directory: {:?}", src_dir);
    println!("Number of modules: {}", module_structure.len());

    let mut merged_content = quote! {
        // Package: #package_name
        // This file was automatically generated by cargo-rustmerge
    };

    let root_module = if src_dir.join("lib.rs").exists() {
        println!("Found lib.rs");
        "crate"
    } else if src_dir.join("main.rs").exists() {
        println!("Found main.rs");
        "crate"
    } else {
        println!("Neither lib.rs nor main.rs found");
        return Err(anyhow::anyhow!(
            "Neither lib.rs nor main.rs found in the src directory"
        ));
    };

    println!("Processing root module: {}", root_module);
    process_module(root_module, module_structure, &mut merged_content)?;

    println!(
        "Merged content size: {} bytes",
        merged_content.to_string().len()
    );
    Ok(merged_content)
}

fn process_module(
    module_path: &str,
    module_structure: &HashMap<String, ModuleInfo>,
    output: &mut TokenStream,
) -> Result<()> {
    println!("Processing module: {}", module_path);
    if let Some(module_info) = module_structure.get(module_path) {
        let file = syn::parse_file(&module_info.content.to_string())?;

        for item in file.items {
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
                        // If the submodule is empty and has no inline content, just declare it
                        quote! {
                            pub mod #ident;
                        }
                    } else {
                        // If the submodule has content or inline content, include it
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
    } else {
        println!("No module info found for {}", module_path);
    }

    Ok(())
}

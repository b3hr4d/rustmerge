mod test;

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;

use anyhow::{Context, Result};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::Attribute;
use syn::File;
use syn::{Item, ItemMod};

#[derive(Debug)]
struct ModuleInfo {
    content: TokenStream,
}
#[derive(Debug)]
struct Args {
    package_name: Option<String>,
    output_path: Option<PathBuf>,
    process_all: bool,
}

fn main() -> Result<()> {
    let args = parse_args()?;

    let current_dir = env::current_dir().context("Failed to get current directory")?;

    if args.process_all {
        process_all_packages(&current_dir, &args)?;
    } else {
        let (package_name, package_path) = determine_package(&current_dir, &args.package_name)?;
        process_single_package(&package_name, &package_path, &args)?;
    }

    Ok(())
}

fn parse_args() -> Result<Args> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args[1] != "rustmerge" {
        eprintln!("Usage: cargo rustmerge [--all] [<package_name>] [--output <path>]");
        std::process::exit(1);
    }

    let mut package_name = None;
    let mut output_path = None;
    let mut process_all = false;
    let mut i = 2;

    while i < args.len() {
        match args[i].as_str() {
            "--output" => {
                i += 1;
                if i < args.len() {
                    output_path = Some(PathBuf::from(&args[i]));
                } else {
                    eprintln!("Error: --output option requires a path");
                    std::process::exit(1);
                }
            }
            "--all" => {
                process_all = true;
            }
            _ => {
                if package_name.is_none() {
                    package_name = Some(args[i].clone());
                } else {
                    eprintln!("Error: Unexpected argument '{}'", args[i]);
                    std::process::exit(1);
                }
            }
        }
        i += 1;
    }

    Ok(Args {
        package_name,
        output_path,
        process_all,
    })
}

fn process_all_packages(workspace_root: &Path, args: &Args) -> Result<()> {
    let cargo_toml = workspace_root.join("Cargo.toml");
    let content = fs::read_to_string(cargo_toml)?;
    let parsed_toml: toml::Value = toml::from_str(&content)?;

    if let Some(workspace) = parsed_toml.get("workspace") {
        let members = workspace
            .get("members")
            .and_then(|m| m.as_array())
            .context("Failed to get workspace members")?;

        for member in members {
            let output_path = if args.output_path.is_some() {
                let member_to_name = member.as_str().unwrap().replace("/", "_");
                Some(
                    args.output_path
                        .as_ref()
                        .unwrap()
                        .join(member_to_name)
                        .with_extension("rs"),
                )
            } else {
                None
            };

            let args_with_output = Args {
                output_path,
                process_all: false,
                package_name: None,
            };
            let package_name = member.as_str().unwrap();
            let package_path = workspace_root.join(package_name);
            process_single_package(package_name, &package_path, &args_with_output)?;
        }
    } else {
        // If it's not a workspace, process the single package
        let (package_name, package_path) = determine_package(workspace_root, &None)?;
        process_single_package(&package_name, &package_path, args)?;
    }

    Ok(())
}

fn process_single_package(package_name: &str, package_path: &Path, args: &Args) -> Result<()> {
    let src_dir = find_src_dir(package_path)?;
    let output_file = args
        .output_path
        .clone()
        .unwrap_or_else(|| create_output_file(package_path, package_name));

    let module_structure = parse_module_structure(&src_dir)?;
    let merged_content = process_package(&src_dir, &module_structure)?;

    let formatted_content = format_rust_code(&merged_content.to_string())?;

    fs::create_dir_all(output_file.parent().unwrap())?;
    fs::write(&output_file, formatted_content)?;
    println!(
        "Merged and formatted Rust program for package '{}' created in {:?}",
        package_name, output_file
    );
    println!("File size: {} bytes", fs::metadata(&output_file)?.len());

    Ok(())
}

fn determine_package(
    current_dir: &Path,
    package_name: &Option<String>,
) -> Result<(String, PathBuf)> {
    if let Some(name) = package_name {
        Ok((name.clone(), current_dir.join(name)))
    } else {
        let cargo_toml = current_dir.join("Cargo.toml");
        let content = fs::read_to_string(cargo_toml)?;
        let parsed_toml: toml::Value = toml::from_str(&content)?;

        if let Some(workspace) = parsed_toml.get("workspace") {
            let members = workspace
                .get("members")
                .and_then(|m| m.as_array())
                .context("Failed to get workspace members")?;

            println!("This is a workspace. Available packages:");
            for (i, member) in members.iter().enumerate() {
                println!("{}. {}", i + 1, member.as_str().unwrap());
            }

            println!("Please run the command again with the package name.");
            std::process::exit(1);
        }

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

fn create_output_file(current_dir: &Path, package_name: &str) -> PathBuf {
    current_dir
        .join("target")
        .join("rustmerge")
        .join(format!("{}_merged.rs", package_name))
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

    let mut module_content = TokenStream::new();

    for item in &file.items {
        if !is_test_module(item) {
            match item {
                Item::Mod(item_mod) => {
                    let submodule_name = &item_mod.ident;
                    let submodule_path = if module_path == "crate" {
                        submodule_name.to_string()
                    } else {
                        format!("{}::{}", module_path, submodule_name)
                    };

                    let cfg_attrs = extract_cfg_attrs(&item_mod.attrs);

                    if let Some((_, items)) = &item_mod.content {
                        // Inline module
                        let mut submodule_content = TokenStream::new();
                        for sub_item in items {
                            sub_item.to_tokens(&mut submodule_content);
                        }
                        let expanded = quote! {
                            #(#cfg_attrs)*
                            pub mod #submodule_name {
                                #submodule_content
                            }
                        };
                        expanded.to_tokens(&mut module_content);

                        // Recursively parse nested inline modules
                        parse_module_items(items, file_path, &submodule_path, module_structure)?;
                    } else {
                        // External module file
                        let submodule_file =
                            file_path.with_file_name(format!("{}.rs", submodule_name));
                        if submodule_file.exists() {
                            parse_file_and_submodules(
                                &submodule_file,
                                &submodule_path,
                                module_structure,
                            )?;
                        } else {
                            let submodule_dir =
                                file_path.with_file_name(submodule_name.to_string());
                            let mod_file = submodule_dir.join("mod.rs");
                            if mod_file.exists() {
                                parse_file_and_submodules(
                                    &mod_file,
                                    &submodule_path,
                                    module_structure,
                                )?;
                            }
                        }

                        // Add the parsed submodule content
                        if let Some(submodule_info) = module_structure.get(&submodule_path) {
                            let submodule_content = &submodule_info.content;
                            let expanded = quote! {
                                #(#cfg_attrs)*
                                pub mod #submodule_name {
                                    #submodule_content
                                }
                            };
                            expanded.to_tokens(&mut module_content);
                        }
                    }
                }
                _ => item.to_tokens(&mut module_content),
            }
        }
    }

    module_structure.insert(
        module_path.to_string(),
        ModuleInfo {
            content: module_content,
        },
    );

    Ok(())
}

fn parse_module_items(
    items: &[Item],
    file_path: &Path,
    module_path: &str,
    module_structure: &mut HashMap<String, ModuleInfo>,
) -> Result<()> {
    let mut module_content = TokenStream::new();

    for item in items {
        if !is_test_module(item) {
            match item {
                Item::Mod(item_mod) => {
                    let submodule_name = &item_mod.ident;
                    let submodule_path = format!("{}::{}", module_path, submodule_name);

                    let cfg_attrs = extract_cfg_attrs(&item_mod.attrs);

                    if let Some((_, sub_items)) = &item_mod.content {
                        let mut submodule_content = TokenStream::new();
                        for sub_item in sub_items {
                            sub_item.to_tokens(&mut submodule_content);
                        }
                        let expanded = quote! {
                            #(#cfg_attrs)*
                            pub mod #submodule_name {
                                #submodule_content
                            }
                        };
                        expanded.to_tokens(&mut module_content);

                        // Recursively parse nested inline modules
                        parse_module_items(
                            sub_items,
                            file_path,
                            &submodule_path,
                            module_structure,
                        )?;
                    }
                }
                _ => item.to_tokens(&mut module_content),
            }
        }
    }

    module_structure.insert(
        module_path.to_string(),
        ModuleInfo {
            content: module_content,
        },
    );

    Ok(())
}

fn extract_cfg_attrs(attrs: &[Attribute]) -> Vec<&Attribute> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"))
        .collect()
}

fn is_test_module(item: &Item) -> bool {
    if let Item::Mod(item_mod) = item {
        item_mod.ident == "test" || item_mod.ident == "tests"
    } else {
        false
    }
}

fn process_package(
    src_dir: &Path,
    module_structure: &HashMap<String, ModuleInfo>,
) -> Result<TokenStream> {
    let mut merged_content = TokenStream::new();

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

fn format_rust_code(code: &str) -> Result<String> {
    let mut rustfmt = Command::new("rustfmt")
        .arg("--edition=2021")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to spawn rustfmt")?;

    {
        let stdin = rustfmt.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(code.as_bytes())
            .context("Failed to write to rustfmt stdin")?;
    }

    let output = rustfmt
        .wait_with_output()
        .context("Failed to read rustfmt output")?;

    if output.status.success() {
        Ok(String::from_utf8(output.stdout).context("rustfmt output was not valid UTF-8")?)
    } else {
        Err(anyhow::anyhow!(
            "rustfmt failed: {:?}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

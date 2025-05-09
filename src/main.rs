mod test;

use regex::Regex;
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
    file_path: PathBuf,       // Absolute path to track module origin
    rel_path: Option<String>, // Relative path from src directory
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
    let output_file = args.output_path.clone().unwrap_or_else(|| {
        let output_path = env::current_dir().unwrap().join("target").join("rustmerge");
        create_output_file(&output_path, package_name)
    });

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

fn create_output_file(output_dir: &Path, package_name: &str) -> PathBuf {
    output_dir.join(package_name).with_extension("rs")
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

    parse_file_and_submodules(&root_file_path, "crate", &mut module_structure, src_dir)?;

    Ok(module_structure)
}

fn parse_file_and_submodules(
    file_path: &Path,
    module_path: &str,
    module_structure: &mut HashMap<String, ModuleInfo>,
    src_dir: &Path,
) -> Result<()> {
    let content = fs::read_to_string(file_path)?;
    let file: File = syn::parse_file(&content)?;

    let mut module_content = TokenStream::new();

    for item in &file.items {
        if !is_ignored_item(item) {
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
                        parse_module_items(
                            items,
                            file_path,
                            &submodule_path,
                            module_structure,
                            src_dir,
                        )?;
                    } else {
                        // External module file
                        let parent = file_path
                            .parent()
                            .context("Failed to get parent directory")?;
                        let parent_mod_name = submodule_path.split("::").next().unwrap();

                        let possible_module_files = [
                            parent.join(submodule_name.to_string()).join("mod.rs"),
                            parent
                                .join(parent_mod_name)
                                .join(format!("{}.rs", submodule_name)),
                            parent.join(format!("{}.rs", submodule_name)),
                        ];

                        let submodule_file = possible_module_files
                            .iter()
                            .find(|p| p.exists())
                            .cloned()
                            .ok_or_else(|| {
                                anyhow::anyhow!("Failed to find module file for {}", submodule_name)
                            })?;

                        parse_file_and_submodules(
                            &submodule_file,
                            &submodule_path,
                            module_structure,
                            src_dir,
                        )?;

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

    // Calculate relative path from src directory
    let rel_path = file_path
        .strip_prefix(src_dir)
        .map(|p| p.to_string_lossy().to_string())
        .ok();

    module_structure.insert(
        module_path.to_string(),
        ModuleInfo {
            content: module_content,
            file_path: file_path.to_path_buf(),
            rel_path,
        },
    );

    Ok(())
}

fn parse_module_items(
    items: &[Item],
    file_path: &Path,
    module_path: &str,
    module_structure: &mut HashMap<String, ModuleInfo>,
    src_dir: &Path,
) -> Result<()> {
    let mut module_content = TokenStream::new();

    for item in items {
        if !is_ignored_item(item) {
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
                            src_dir,
                        )?;
                    }
                }
                _ => item.to_tokens(&mut module_content),
            }
        }
    }

    // Calculate relative path from src directory
    let rel_path = file_path
        .strip_prefix(src_dir)
        .map(|p| p.to_string_lossy().to_string())
        .ok();

    module_structure.insert(
        module_path.to_string(),
        ModuleInfo {
            content: module_content,
            file_path: file_path.to_path_buf(),
            rel_path,
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

// Helper function to check for #[cfg(test)]
fn is_cfg_test_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("cfg") {
            // In syn 2.x, attr.meta directly gives the Meta item.
            // For an attribute like #[cfg(test)], attr.meta is Meta::List.
            // The meta_list.path would be `cfg` and meta_list.tokens would be `test`.
            if let syn::Meta::List(meta_list) = &attr.meta {
                // Ensure the attribute is specifically #[cfg(test)]
                // meta_list.path is 'cfg'
                // meta_list.tokens should be 'test'
                if meta_list.tokens.to_string() == "test" {
                    return true;
                }
            }
        }
        false
    })
}

// Combined function to check if an item should be ignored
fn is_ignored_item(item: &Item) -> bool {
    let attributes_to_check: Option<&Vec<Attribute>> = match item {
        Item::Const(item_const) => Some(&item_const.attrs),
        Item::Enum(item_enum) => Some(&item_enum.attrs),
        Item::ExternCrate(item_extern_crate) => Some(&item_extern_crate.attrs),
        Item::Fn(item_fn) => Some(&item_fn.attrs),
        Item::ForeignMod(item_foreign_mod) => Some(&item_foreign_mod.attrs),
        Item::Impl(item_impl) => Some(&item_impl.attrs),
        Item::Macro(item_macro) => Some(&item_macro.attrs),
        Item::Mod(item_mod) => {
            // Special handling for module names like 'test' or 'tests'
            if item_mod.ident == "test" || item_mod.ident == "tests" {
                return true; // Always ignore modules named 'test' or 'tests'
            }
            Some(&item_mod.attrs) // Otherwise, check attributes of the module
        }
        Item::Static(item_static) => Some(&item_static.attrs),
        Item::Struct(item_struct) => Some(&item_struct.attrs),
        Item::Trait(item_trait) => Some(&item_trait.attrs),
        Item::TraitAlias(item_trait_alias) => Some(&item_trait_alias.attrs),
        Item::Type(item_type) => Some(&item_type.attrs),
        Item::Union(item_union) => Some(&item_union.attrs),
        Item::Use(item_use) => Some(&item_use.attrs),
        _ => None, // For item types without attributes or not relevant for this check
    };

    if let Some(attrs) = attributes_to_check {
        if is_cfg_test_attr(attrs) {
            return true;
        }
    }
    false
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

        // Get relative file path for comment for the root module
        let file_path_str = module_info.rel_path.as_deref().unwrap_or_else(|| {
            module_info
                .file_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown.rs")
        });

        // Add root module file comment - encode filename to avoid issues with special characters
        let marker = format!("RUSTMERGE_COMMENT_{}", encode_filename(file_path_str));
        let marker_lit = proc_macro2::Literal::string(&marker);
        let comment_tokens = quote! {
            const _: &'static str = #marker_lit;
        };
        comment_tokens.to_tokens(output);

        for item in file.items {
            if !is_ignored_item(&item) {
                match item {
                    Item::Mod(ItemMod { ident, content, .. }) => {
                        let submodule_path = if module_path == "crate" {
                            ident.to_string()
                        } else {
                            format!("{}::{}", module_path, ident)
                        };

                        let mut submodule_content = TokenStream::new();

                        // Find the actual file path for this module
                        if let Some(submodule_info) = module_structure.get(&submodule_path) {
                            // Get this module's file path
                            let sub_path_str =
                                submodule_info.rel_path.as_deref().unwrap_or_else(|| {
                                    submodule_info
                                        .file_path
                                        .file_name()
                                        .and_then(|name| name.to_str())
                                        .unwrap_or("unknown.rs")
                                });

                            // Only add comment if the module is in a different file than its parent
                            if sub_path_str != file_path_str {
                                let sub_marker =
                                    format!("RUSTMERGE_COMMENT_{}", encode_filename(sub_path_str));
                                let sub_marker_lit = proc_macro2::Literal::string(&sub_marker);

                                let sub_comment_tokens = quote! {
                                    const _: &'static str = #sub_marker_lit;
                                };
                                sub_comment_tokens.to_tokens(&mut submodule_content);
                            }

                            // Process the content of the module
                            process_module_content(
                                &submodule_path,
                                module_structure,
                                &mut submodule_content,
                                sub_path_str, // Pass the current module's file path
                            )?;
                        }

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

// Process module content with tracking of parent file path
fn process_module_content(
    module_path: &str,
    module_structure: &HashMap<String, ModuleInfo>,
    output: &mut TokenStream,
    parent_file_path: &str, // Track parent file path to avoid duplicate comments
) -> Result<()> {
    if let Some(module_info) = module_structure.get(module_path) {
        let file = syn::parse_file(&module_info.content.to_string())?;

        for item in file.items {
            if !is_ignored_item(&item) {
                match item {
                    Item::Mod(ItemMod { ident, content, .. }) => {
                        let submodule_path = if module_path == "crate" {
                            ident.to_string()
                        } else {
                            format!("{}::{}", module_path, ident)
                        };

                        let mut submodule_content = TokenStream::new();

                        // Add file comment if this module is in a different file
                        if let Some(submodule_info) = module_structure.get(&submodule_path) {
                            let sub_path_str =
                                submodule_info.rel_path.as_deref().unwrap_or_else(|| {
                                    submodule_info
                                        .file_path
                                        .file_name()
                                        .and_then(|name| name.to_str())
                                        .unwrap_or("unknown.rs")
                                });

                            // Only add comment if module is in a different file than its parent
                            if sub_path_str != parent_file_path {
                                let sub_marker =
                                    format!("RUSTMERGE_COMMENT_{}", encode_filename(sub_path_str));
                                let sub_marker_lit = proc_macro2::Literal::string(&sub_marker);

                                let sub_comment_tokens = quote! {
                                    const _: &'static str = #sub_marker_lit;
                                };
                                sub_comment_tokens.to_tokens(&mut submodule_content);
                            }

                            // Process this module's content
                            process_module_content(
                                &submodule_path,
                                module_structure,
                                &mut submodule_content,
                                sub_path_str, // Pass this module's file path
                            )?;
                        }

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

// Helper function to safely encode filenames for our markers
fn encode_filename(filename: &str) -> String {
    // Use base64 encoding to safely handle any special characters
    use std::fmt::Write;

    let mut encoded = String::new();
    for byte in filename.bytes() {
        match byte {
            b'.' => write!(encoded, "__DOT__").unwrap(),
            b'_' => write!(encoded, "__UNDERSCORE__").unwrap(),
            b'/' => write!(encoded, "__SLASH__").unwrap(),
            b'-' => write!(encoded, "__DASH__").unwrap(),
            _ => encoded.push(byte as char),
        }
    }
    encoded
}

fn format_rust_code(code: &str) -> Result<String> {
    // Run rustfmt first to get well-formatted code
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

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "rustfmt failed: {:?}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let formatted =
        String::from_utf8(output.stdout).context("rustfmt output was not valid UTF-8")?;

    // Now replace our markers with actual comments
    let pattern = r#"const\s+_\s*:\s*&\s*'static\s*str\s*=\s*"RUSTMERGE_COMMENT_([^"]+)"\s*;"#;
    let re = Regex::new(pattern).unwrap();
    let result = re.replace_all(&formatted, |caps: &regex::Captures| {
        let encoded_filename = &caps[1];
        let filename = decode_filename(encoded_filename);
        format!("// {}", filename)
    });

    Ok(result.to_string())
}

// Helper function to decode our specially encoded filenames
fn decode_filename(encoded: &str) -> String {
    let mut result = encoded.to_string();
    result = result.replace("__DOT__", ".");
    result = result.replace("__UNDERSCORE__", "_");
    result = result.replace("__SLASH__", "/");
    result = result.replace("__DASH__", "-");
    result
}

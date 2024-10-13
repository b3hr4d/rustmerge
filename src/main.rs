mod test;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::process::{Command, Stdio};
use toml::Value;

#[derive(Debug)]
struct PackageNotFoundError(String);

impl fmt::Display for PackageNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Package '{}' not found in workspace", self.0)
    }
}

impl Error for PackageNotFoundError {}

struct ProjectInfo {
    root: PathBuf,
    packages: Vec<(String, PathBuf)>, // Now includes package paths
    is_workspace: bool,
}

struct ModuleInfo {
    path: PathBuf,
    submodules: Vec<String>,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args[1] != "rustmerge" {
        eprintln!("Usage: cargo rustmerge [<package_name>]");
        process::exit(1);
    }

    let current_dir = env::current_dir().expect("Failed to get current directory");
    let project_info = find_project_root(&current_dir);
    let package_name = determine_package_name(&project_info, &args);

    let package_root =
        get_package_root(&project_info, &package_name).expect("Failed to find package root");
    let src_dir = get_src_dir(&package_root);
    let output_file = create_output_file(&current_dir, &package_name);

    let module_structure = parse_module_structure(&src_dir);
    process_package(&src_dir, &output_file, &package_name, &module_structure);
}

fn parse_module_structure(src_dir: &Path) -> HashMap<String, ModuleInfo> {
    let mut module_structure = HashMap::new();
    parse_directory(src_dir, "", &mut module_structure);
    module_structure
}

fn parse_directory(
    dir: &Path,
    parent_module: &str,
    module_structure: &mut HashMap<String, ModuleInfo>,
) {
    let mod_file = dir.join("mod.rs");
    let mut submodules = Vec::new();

    if mod_file.exists() {
        parse_mod_file(&mod_file, &mut submodules);
    }

    let module_name = if parent_module.is_empty() {
        "crate".to_string()
    } else {
        parent_module.to_string()
    };

    module_structure.insert(
        module_name.clone(),
        ModuleInfo {
            path: dir.to_path_buf(),
            submodules,
        },
    );

    for entry in fs::read_dir(dir).expect("Failed to read directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.is_dir() {
            let dir_name = path.file_name().unwrap().to_str().unwrap();
            let new_module = if module_name == "crate" {
                dir_name.to_string()
            } else {
                format!("{}::{}", module_name, dir_name)
            };
            parse_directory(&path, &new_module, module_structure);
        }
    }
}

fn parse_mod_file(mod_file: &Path, submodules: &mut Vec<String>) {
    let file = fs::File::open(mod_file).expect("Failed to open mod.rs");
    let reader = std::io::BufReader::new(file);

    for line in reader.lines() {
        let line = line.expect("Failed to read line");
        if let Some(module) = line.trim().strip_prefix("pub mod ") {
            submodules.push(module.trim_end_matches(';').to_string());
        } else if let Some(module) = line.trim().strip_prefix("mod ") {
            submodules.push(module.trim_end_matches(';').to_string());
        }
    }
}

fn find_project_root(start_path: &Path) -> ProjectInfo {
    let mut current_path = start_path.to_path_buf();
    loop {
        let cargo_toml_path = current_path.join("Cargo.toml");
        if cargo_toml_path.exists() {
            let contents = fs::read_to_string(&cargo_toml_path).expect("Failed to read Cargo.toml");
            let parsed_toml: Value = contents.parse().expect("Failed to parse Cargo.toml");

            if parsed_toml.get("workspace").is_some() {
                return handle_workspace(&current_path, &parsed_toml);
            } else if parsed_toml.get("package").is_some() {
                return handle_single_package(&current_path, &parsed_toml);
            }
        }

        if !current_path.pop() {
            eprintln!("No Cargo.toml found in any parent directory");
            process::exit(1);
        }
    }
}

fn handle_workspace(path: &Path, toml: &Value) -> ProjectInfo {
    let mut packages = Vec::new();
    if let Some(workspace) = toml.get("workspace").and_then(|w| w.get("members")) {
        if let Some(members) = workspace.as_array() {
            for member in members {
                if let Some(member_str) = member.as_str() {
                    let member_path = path.join(member_str);
                    let member_cargo_toml = member_path.join("Cargo.toml");
                    if let Ok(member_contents) = fs::read_to_string(&member_cargo_toml) {
                        if let Ok(member_parsed) = member_contents.parse::<Value>() {
                            if let Some(package_name) = member_parsed
                                .get("package")
                                .and_then(|p| p.get("name"))
                                .and_then(|n| n.as_str())
                            {
                                packages.push((package_name.to_string(), member_path));
                            }
                        }
                    }
                }
            }
        }
    }
    ProjectInfo {
        root: path.to_path_buf(),
        packages,
        is_workspace: true,
    }
}

fn handle_single_package(path: &Path, toml: &Value) -> ProjectInfo {
    let package_name = toml
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .expect("Failed to get package name")
        .to_string();
    ProjectInfo {
        root: path.to_path_buf(),
        packages: vec![(package_name, path.to_path_buf())],
        is_workspace: false,
    }
}

fn determine_package_name(project_info: &ProjectInfo, args: &[String]) -> String {
    if project_info.is_workspace {
        if args.len() > 2 {
            let package_name = &args[2];
            if project_info
                .packages
                .iter()
                .any(|(name, _)| name == package_name)
            {
                package_name.clone()
            } else {
                eprintln!("Package '{}' not found in workspace", package_name);
                eprintln!("Available packages:");
                for (name, _) in &project_info.packages {
                    eprintln!("  {}", name);
                }
                process::exit(1);
            }
        } else if project_info.packages.len() == 1 {
            project_info.packages[0].0.clone()
        } else {
            eprintln!("Multiple packages found in workspace. Please specify a package name.");
            eprintln!("Available packages:");
            for (name, _) in &project_info.packages {
                eprintln!("  {}", name);
            }
            process::exit(1);
        }
    } else {
        project_info.packages[0].0.clone()
    }
}

fn get_package_root(
    project_info: &ProjectInfo,
    package_name: &str,
) -> Result<PathBuf, PackageNotFoundError> {
    if project_info.is_workspace {
        project_info
            .packages
            .iter()
            .find(|(name, _)| name == package_name)
            .map(|(_, path)| path.clone())
            .ok_or_else(|| PackageNotFoundError(package_name.to_string()))
    } else {
        Ok(project_info.root.clone())
    }
}

fn get_src_dir(package_root: &Path) -> PathBuf {
    let cargo_toml_path = package_root.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        eprintln!("Cargo.toml not found in {:?}", package_root);
        process::exit(1);
    }

    let contents = fs::read_to_string(&cargo_toml_path)
        .expect(&format!("Failed to read Cargo.toml in {:?}", package_root));
    let parsed_toml: Value = contents
        .parse()
        .expect(&format!("Failed to parse Cargo.toml in {:?}", package_root));

    parsed_toml
        .get("package")
        .and_then(|p| p.get("src"))
        .and_then(|s| s.as_str())
        .map(|src| package_root.join(src))
        .unwrap_or_else(|| package_root.join("src"))
}

fn create_output_file(current_dir: &Path, package_name: &str) -> PathBuf {
    let output_file = current_dir
        .join("target")
        .join(format!("{}_merged.rs", package_name));
    fs::create_dir_all(output_file.parent().unwrap()).expect("Failed to create target directory");
    output_file
}

fn process_package(
    src_dir: &Path,
    output_file: &Path,
    package_name: &str,
    module_structure: &HashMap<String, ModuleInfo>,
) {
    let mut unformatted_output = Vec::new();

    writeln!(&mut unformatted_output, "// Package: {}", package_name).unwrap();
    writeln!(
        &mut unformatted_output,
        "// This file was automatically generated by cargo-rustmerge"
    )
    .unwrap();
    writeln!(&mut unformatted_output).unwrap();

    process_module("crate", module_structure, &mut unformatted_output, 0);

    println!("Processing Rust files in {:?}", src_dir);

    if src_dir.join("lib.rs").exists() {
        process_file(&src_dir.join("lib.rs"), &mut unformatted_output);
    } else if src_dir.join("main.rs").exists() {
        process_file(&src_dir.join("main.rs"), &mut unformatted_output);
    } else {
        eprintln!(
            "Error: Neither lib.rs nor main.rs found in the src directory: {:?}",
            src_dir
        );
        process::exit(1);
    }

    process_directory(src_dir, &mut unformatted_output, 0);

    // Format the merged code
    let formatted_output = format_rust_code(&unformatted_output);

    // Write the formatted output to the file
    fs::write(output_file, formatted_output).expect("Failed to write output file");

    println!(
        "Merged and formatted Rust program created in {:?}",
        output_file
    );
}

fn process_module(
    module_name: &str,
    module_structure: &HashMap<String, ModuleInfo>,
    output: &mut Vec<u8>,
    depth: usize,
) {
    if let Some(module_info) = module_structure.get(module_name) {
        for submodule in &module_info.submodules {
            let full_module_name = if module_name == "crate" {
                submodule.clone()
            } else {
                format!("{}::{}", module_name, submodule)
            };

            if let Some(submodule_info) = module_structure.get(&full_module_name) {
                writeln!(output, "{}pub mod {} {{", "    ".repeat(depth), submodule).unwrap();
                process_file(
                    &submodule_info.path.join(format!("{}.rs", submodule)),
                    output,
                );
                process_module(&full_module_name, module_structure, output, depth + 1);
                writeln!(output, "{}}}", "    ".repeat(depth)).unwrap();
            }
        }
    }
}

fn process_directory(dir: &Path, output: &mut Vec<u8>, depth: usize) {
    for entry in fs::read_dir(dir).expect("Failed to read directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |ext| ext == "rs") {
            let file_name = path.file_name().unwrap().to_str().unwrap();
            if file_name != "lib.rs" && file_name != "main.rs" && file_name != "mod.rs" {
                let module_name = path.file_stem().unwrap().to_str().unwrap();
                let escaped_module_name = escape_rust_keyword(module_name);
                writeln!(
                    output,
                    "{}pub mod {} {{",
                    "    ".repeat(depth),
                    escaped_module_name
                )
                .unwrap();
                process_file(&path, output);
                writeln!(output, "{}}}", "    ".repeat(depth)).unwrap();
                writeln!(output).unwrap();
            }
        } else if path.is_dir() {
            let dir_name = path.file_name().unwrap().to_str().unwrap();
            let escaped_dir_name = escape_rust_keyword(dir_name);
            writeln!(
                output,
                "{}pub mod {} {{",
                "    ".repeat(depth),
                escaped_dir_name
            )
            .unwrap();
            process_directory(&path, output, depth + 1);
            writeln!(output, "{}}}", "    ".repeat(depth)).unwrap();
            writeln!(output).unwrap();
        }
    }
}

fn escape_rust_keyword(name: &str) -> String {
    match name {
        // List of Rust keywords that need to be escaped
        "as" | "break" | "const" | "continue" | "crate" | "else" | "enum" | "extern" | "false"
        | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" | "match" | "mod" | "move"
        | "mut" | "pub" | "ref" | "return" | "self" | "Self" | "static" | "struct" | "super"
        | "trait" | "true" | "type" | "unsafe" | "use" | "where" | "while" | "async" | "await"
        | "dyn" => format!("r#{}", name),
        _ => name.to_string(),
    }
}

fn process_file(file_path: &Path, output: &mut Vec<u8>) {
    let file = fs::File::open(file_path).expect("Failed to open file");
    let reader = BufReader::new(file);
    let mut in_test_module = false;
    let mut open_braces = 0;

    for line in reader.lines() {
        let line = line.expect("Failed to read line");
        if line.trim().starts_with("#[cfg(test)]") || line.trim().starts_with("#[test]") {
            in_test_module = true;
            continue;
        }

        if in_test_module {
            open_braces += line.matches('{').count() as i32 - line.matches('}').count() as i32;
            if open_braces <= 0 {
                in_test_module = false;
            }
            continue;
        }

        writeln!(output, "{}", line).unwrap();
    }
}

fn format_rust_code(unformatted_code: &[u8]) -> Vec<u8> {
    let mut rustfmt = Command::new("rustfmt")
        .args(&["--edition", "2021"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn rustfmt");

    {
        let stdin = rustfmt.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(unformatted_code)
            .expect("Failed to write to stdin");
    }

    let output = rustfmt.wait_with_output().expect("Failed to read stdout");

    if !output.status.success() {
        eprintln!("Warning: rustfmt failed to format the code. Error:");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        eprintln!("Using unformatted code.");
        return unformatted_code.to_vec();
    }

    output.stdout
}

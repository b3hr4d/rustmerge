#[cfg(test)]
mod tests {
    use crate::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use syn::{parse_quote, Item};
    use tempfile::TempDir;

    fn setup_temp_cargo_toml(package_name: &str) -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");
        let mut cargo_toml = File::create(&cargo_toml_path).unwrap();
        writeln!(
            cargo_toml,
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"
"#,
            package_name
        )
        .unwrap();
        (temp_dir, cargo_toml_path)
    }

    #[test]
    fn test_determine_package_with_provided_name() {
        let package_name = "test_package".to_string();
        let current_dir = PathBuf::from(".");
        let result = determine_package(&current_dir, &Some(package_name.clone()));
        assert!(result.is_ok());
        let (name, path) = result.unwrap();
        assert_eq!(name, package_name);
        assert_eq!(path, current_dir.join(package_name));
    }

    #[test]
    fn test_determine_package_from_cargo_toml() {
        let (temp_dir, _) = setup_temp_cargo_toml("test_package");
        let result = determine_package(temp_dir.path(), &None);
        assert!(result.is_ok());
        let (name, path) = result.unwrap();
        assert_eq!(name, "test_package");
        assert_eq!(path, temp_dir.path());
    }

    #[test]
    fn test_find_src_dir() {
        // Create a temporary directory using tempfile crate
        let temp_dir = tempfile::tempdir().unwrap();

        // Create "src" directory inside the temp directory
        let src_dir = temp_dir.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();

        // Create a dummy Cargo.toml file inside the temp directory
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");
        let mut cargo_toml = File::create(&cargo_toml_path).unwrap();
        writeln!(
            cargo_toml,
            r#"[package]
    name = "test_package"
    version = "0.1.0"
    edition = "2021"
    "#,
        )
        .unwrap();

        // Test the find_src_dir function
        let src_dir_found = find_src_dir(temp_dir.path()).expect("Failed to find src dir");
        assert_eq!(src_dir_found, src_dir);
    }

    #[test]
    fn test_parse_module_structure() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("src")).unwrap();

        let lib_rs_path = temp_dir.path().join("src/lib.rs");
        let mut lib_rs = File::create(&lib_rs_path).unwrap();
        writeln!(lib_rs, "pub mod example_module;").unwrap();

        let mod_path = temp_dir.path().join("src/example_module.rs");
        let mut mod_file = File::create(&mod_path).unwrap();
        writeln!(mod_file, "pub fn example_function() -> i32 {{ 42 }}").unwrap();

        let module_structure = parse_module_structure(&temp_dir.path().join("src"))
            .expect("Failed to parse module structure");

        assert!(module_structure.contains_key("crate"));
        assert!(module_structure.contains_key("example_module"));
    }

    #[test]
    fn test_format_rust_code() {
        let code = r#"fn main() {println!("Hello, world!");}"#;
        let formatted_code = format_rust_code(code).expect("Failed to format code");
        assert!(formatted_code.contains("fn main() {"));
    }

    #[test]
    fn test_parse_nested_module_structure() {
        // Create a temporary directory using tempfile crate
        let temp_dir = tempfile::tempdir().unwrap();

        // Create directories for module_a and module_b
        let module_a_dir = temp_dir.path().join("src/module_a");
        let module_b_dir = temp_dir.path().join("src/module_b/submodule");
        std::fs::create_dir_all(&module_a_dir).unwrap();
        std::fs::create_dir_all(&module_b_dir).unwrap();

        // Create lib.rs and write the module declarations
        let lib_rs_path = temp_dir.path().join("src/lib.rs");
        let mut lib_rs = File::create(&lib_rs_path).unwrap();
        writeln!(lib_rs, "pub mod module_a;\npub mod module_b;").unwrap();

        // Create module_a/mod.rs
        let mod_a_path = module_a_dir.join("mod.rs");
        let mut mod_a_file = File::create(&mod_a_path).unwrap();
        writeln!(mod_a_file, "pub mod mod_a1;\npub mod mod_a2;").unwrap();

        // Write module_a/mod_a1.rs
        let mod_a1_path = module_a_dir.join("mod_a1.rs");
        let mut mod_a1_file = File::create(&mod_a1_path).unwrap();
        writeln!(mod_a1_file, "pub fn function_a1() -> i32 {{ 10 }}").unwrap();

        // Write module_a/mod_a2.rs
        let mod_a2_path = module_a_dir.join("mod_a2.rs");
        let mut mod_a2_file = File::create(&mod_a2_path).unwrap();
        writeln!(mod_a2_file, "pub fn function_a2() -> i32 {{ 20 }}").unwrap();

        // Create module_b/mod.rs
        let mod_b_path = temp_dir.path().join("src/module_b/mod.rs");
        let mut mod_b_file = File::create(&mod_b_path).unwrap();
        writeln!(
            mod_b_file,
            "pub mod mod_b1;\npub mod mod_b2;\npub mod submodule;"
        )
        .unwrap();

        // Write module_b/mod_b1.rs
        let mod_b1_path = temp_dir.path().join("src/module_b/mod_b1.rs");
        let mut mod_b1_file = File::create(&mod_b1_path).unwrap();
        writeln!(mod_b1_file, "pub fn function_b1() -> i32 {{ 30 }}").unwrap();

        // Write module_b/mod_b2.rs
        let mod_b2_path = temp_dir.path().join("src/module_b/mod_b2.rs");
        let mut mod_b2_file = File::create(&mod_b2_path).unwrap();
        writeln!(mod_b2_file, "pub fn function_b2() -> i32 {{ 40 }}").unwrap();

        // Create module_b/submodule/mod.rs
        let mod_b_submodule_path = temp_dir.path().join("src/module_b/submodule/mod.rs");
        let mut mod_b_submodule_file = File::create(&mod_b_submodule_path).unwrap();
        writeln!(mod_b_submodule_file, "pub mod mod_b3;").unwrap();

        // Write module_b/submodule/mod_b3.rs
        let mod_b3_path = module_b_dir.join("mod_b3.rs");
        let mut mod_b3_file = File::create(&mod_b3_path).unwrap();
        writeln!(mod_b3_file, "pub fn function_b3() -> i32 {{ 50 }}").unwrap();

        // Test that the module structure can be parsed correctly
        let module_structure = parse_module_structure(&temp_dir.path().join("src"))
            .expect("Failed to parse module structure");

        // Assert that all modules are present in the parsed structure
        assert!(module_structure.contains_key("crate"));
        assert!(module_structure.contains_key("module_a"));
        assert!(module_structure.contains_key("module_a::mod_a1"));
        assert!(module_structure.contains_key("module_a::mod_a2"));
        assert!(module_structure.contains_key("module_b"));
        assert!(module_structure.contains_key("module_b::mod_b1"));
        assert!(module_structure.contains_key("module_b::mod_b2"));
        assert!(module_structure.contains_key("module_b::submodule::mod_b3"));

        let processed = process_package(&temp_dir.path().join("src"), &module_structure)
            .expect("Failed to process package");

        let formated_code =
            format_rust_code(&processed.to_string()).expect("Failed to format code");
        println!("{}", formated_code);
    }

    #[test]
    fn test_process_package() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("src")).unwrap();

        let lib_rs_path = temp_dir.path().join("src/lib.rs");
        let mut lib_rs = File::create(&lib_rs_path).unwrap();
        writeln!(lib_rs, "pub mod example_module;").unwrap();

        let mod_path = temp_dir.path().join("src/example_module.rs");
        let mut mod_file = File::create(&mod_path).unwrap();
        writeln!(mod_file, "pub fn example_function() -> i32 {{ 42 }}").unwrap();

        let module_structure = parse_module_structure(&temp_dir.path().join("src"))
            .expect("Failed to parse module structure");

        let processed = process_package(&temp_dir.path().join("src"), &module_structure)
            .expect("Failed to process package");

        let formated_code =
            format_rust_code(&processed.to_string()).expect("Failed to format code");
        println!("{}", formated_code);
    }

    #[test]
    fn test_parse_nested_module_structure_with_simple_mod() {
        // Create a temporary directory using tempfile crate
        let temp_dir = tempfile::tempdir().unwrap();
        // Create directories for src
        std::fs::create_dir_all(temp_dir.path().join("src")).unwrap();

        let lib_rs_path = temp_dir.path().join("src/lib.rs");
        // Create lib.rs and write the module declarations
        let mut lib_rs = File::create(&lib_rs_path).unwrap();
        writeln!(
            lib_rs,
            r#"
            const MY_CONSTANT: i32 = 42;

            #[cfg(not(test))]
            mod mother_mod {{
                fn mother_function() -> i32 {{ 10 }} 
                pub mod nested_mod {{
                    fn nested_function() -> i32 {{ 20 }}
                }}
            }}
           "#
        )
        .unwrap();

        // Test that the module structure can be parsed correctly
        let module_structure = parse_module_structure(&temp_dir.path().join("src"))
            .expect("Failed to parse module structure");
        // Assert that all modules are present in the parsed structure
        assert!(module_structure.contains_key("crate"));

        let processed = process_package(&temp_dir.path().join("src"), &module_structure)
            .expect("Failed to process package");

        let formated_code =
            format_rust_code(&processed.to_string()).expect("Failed to format code");
        println!("{}", formated_code);
    }

    #[test]
    fn test_parse_nested_module_structure_with_complex_mod() {
        // Create a temporary directory using tempfile crate
        let temp_dir = tempfile::tempdir().unwrap();
        // Create directories for module_a and module_b
        let module_a_dir = temp_dir.path().join("src/module_a");
        let module_b_dir = temp_dir.path().join("src/module_b/submodule");
        std::fs::create_dir_all(&module_a_dir).unwrap();
        std::fs::create_dir_all(&module_b_dir).unwrap();
        // Create lib.rs and write the module declarations
        let lib_rs_path = temp_dir.path().join("src/lib.rs");
        let mut lib_rs = File::create(&lib_rs_path).unwrap();
        writeln!(lib_rs, "pub mod module_a;\npub mod module_b;").unwrap();
        // Create module_a/mod.rs
        let mod_a_path = module_a_dir.join("mod.rs");
        let mut mod_a_file = File::create(&mod_a_path).unwrap();
        writeln!(mod_a_file, "pub mod mod_a1;\npub mod mod_a2;").unwrap();
        // Write module_a/mod_a1.rs
        let mod_a1_path = module_a_dir.join("mod_a1.rs");
        let mut mod_a1_file = File::create(&mod_a1_path).unwrap();
        writeln!(
            mod_a1_file,
            r#"
        pub mod function_a1_mod {{
            pub fn function_a1() -> i32 {{ 10 }} 
        }}
           "#
        )
        .unwrap();
        // Write module_a/mod_a2.rs
        let mod_a2_path = module_a_dir.join("mod_a2.rs");
        let mut mod_a2_file = File::create(&mod_a2_path).unwrap();
        writeln!(mod_a2_file, "pub fn function_a2() -> i32 {{ 20 }}").unwrap();
        // Create module_b/mod.rs
        let mod_b_path = temp_dir.path().join("src/module_b/mod.rs");
        let mut mod_b_file = File::create(&mod_b_path).unwrap();
        writeln!(
            mod_b_file,
            "pub mod mod_b1;\npub mod mod_b2;\npub mod submodule;"
        )
        .unwrap();
        // Write module_b/mod_b1.rs
        let mod_b1_path = temp_dir.path().join("src/module_b/mod_b1.rs");
        let mut mod_b1_file = File::create(&mod_b1_path).unwrap();
        writeln!(mod_b1_file, "pub fn function_b1() -> i32 {{ 30 }}").unwrap();
        // Write module_b/mod_b2.rs
        let mod_b2_path = temp_dir.path().join("src/module_b/mod_b2.rs");
        let mut mod_b2_file = File::create(&mod_b2_path).unwrap();
        writeln!(mod_b2_file, "pub fn function_b2() -> i32 {{ 40 }}").unwrap();
        // Create module_b/submodule/mod.rs
        let mod_b_submodule_path = temp_dir.path().join("src/module_b/submodule/mod.rs");
        let mut mod_b_submodule_file = File::create(&mod_b_submodule_path).unwrap();
        writeln!(mod_b_submodule_file, "pub mod mod_b3;").unwrap();
        // Write module_b/submodule/mod_b3.rs
        let mod_b3_path = module_b_dir.join("mod_b3.rs");
        let mut mod_b3_file = File::create(&mod_b3_path).unwrap();
        writeln!(mod_b3_file, "pub fn function_b3() -> i32 {{ 50 }}").unwrap();

        // Test that the module structure can be parsed correctly
        let module_structure = parse_module_structure(&temp_dir.path().join("src"))
            .expect("Failed to parse module structure");
        // Assert that all modules are present in the parsed structure
        assert!(module_structure.contains_key("crate"));
        assert!(module_structure.contains_key("module_a"));
        assert!(module_structure.contains_key("module_a::mod_a1"));
        assert!(module_structure.contains_key("module_a::mod_a2"));
        assert!(module_structure.contains_key("module_b"));
        assert!(module_structure.contains_key("module_b::mod_b1"));
        assert!(module_structure.contains_key("module_b::mod_b2"));
        assert!(module_structure.contains_key("module_b::submodule::mod_b3"));

        let processed = process_package(&temp_dir.path().join("src"), &module_structure)
            .expect("Failed to process package");

        let formated_code =
            format_rust_code(&processed.to_string()).expect("Failed to format code");

        println!("{}", formated_code);
    }

    #[test]
    fn test_parse_nested_module_structure_with_complex_mod_2() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let package_dir = temp_dir.path().join("test_package");
        fs::create_dir(&package_dir)?;

        // Create a basic package structure
        fs::write(
            package_dir.join("Cargo.toml"),
            r#"
[package]
name = "test_package"
version = "0.1.0"
edition = "2021"
"#,
        )?;

        let src_dir = package_dir.join("src");
        fs::create_dir(&src_dir)?;

        // Create main.rs
        fs::write(
            src_dir.join("main.rs"),
            r#"
mod mother_mod;
"#,
        )?;

        // Create mother_mod.rs
        fs::write(
            src_dir.join("mother_mod.rs"),
            r#"
fn mother_function() -> i32 {
    10
}

pub mod nested_mod;
"#,
        )?;

        // Create nested_mod.rs inside mother_mod folder
        let nested_mod_dir = src_dir.join("mother_mod");
        fs::create_dir(&nested_mod_dir)?;
        fs::write(
            nested_mod_dir.join("nested_mod.rs"),
            r#"
pub fn nested_function() -> i32 {
    20
}
"#,
        )?;

        let mut module_structure = HashMap::new();
        parse_file_and_submodules(&src_dir.join("main.rs"), "crate", &mut module_structure)?;

        let processed = process_package(&src_dir, &module_structure)?;

        let formatted_code = format_rust_code(&processed.to_string())?;

        println!("{}", formatted_code);

        Ok(())
    }

    #[test]
    fn test_parse_file_and_submodules() {
        let file_content = r#"
            #[cfg(feature = "canbench")]
            mod bench_mod {
                pub fn bench_function() {}
            }

            mod normal_mod {
                pub fn normal_function() {}
            }
        "#;

        let file_path = Path::new("test.rs");
        fs::write(file_path, file_content).unwrap();

        let mut module_structure = HashMap::new();
        parse_file_and_submodules(file_path, "mod", &mut module_structure).unwrap();

        assert!(module_structure.contains_key("mod::bench_mod"));
        assert!(module_structure.contains_key("mod::normal_mod"));

        fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_parse_module_items() {
        let items: Vec<Item> = vec![
            parse_quote! {
                #[cfg(feature = "canbench")]
                mod bench_mod {
                    pub fn bench_function() {}
                }
            },
            parse_quote! {
                mod normal_mod {
                    pub fn normal_function() {}
                }
            },
        ];

        let file_path = Path::new("test.rs");
        let mut module_structure = HashMap::new();
        parse_module_items(&items, file_path, "crate", &mut module_structure).unwrap();

        assert!(module_structure.contains_key("crate::bench_mod"));
        assert!(module_structure.contains_key("crate::normal_mod"));
    }
}

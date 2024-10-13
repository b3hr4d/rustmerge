use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_rust_merge_process() {
    // Set up a temporary directory structure
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    // Create Cargo.toml
    let cargo_toml_path = temp_dir.path().join("Cargo.toml");
    let cargo_toml_content = r#"
[package]
name = "test_package"
version = "0.1.0"
"#;
    File::create(&cargo_toml_path)
        .unwrap()
        .write_all(cargo_toml_content.as_bytes())
        .unwrap();

    // Create lib.rs
    let lib_rs_path = src_dir.join("lib.rs");
    let lib_rs_content = r#"
pub mod module_a;
pub mod module_b;

pub fn root_function() -> &'static str {
    "I'm the root function"
}
"#;
    File::create(&lib_rs_path)
        .unwrap()
        .write_all(lib_rs_content.as_bytes())
        .unwrap();

    // Create module_a.rs
    let module_a_path = src_dir.join("module_a.rs");
    let module_a_content = r#"
pub fn function_a() -> &'static str {
    "I'm function A"
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_function_a() {
        assert_eq!(super::function_a(), "I'm function A");
    }
}
"#;
    File::create(&module_a_path)
        .unwrap()
        .write_all(module_a_content.as_bytes())
        .unwrap();

    // Create module_b.rs
    let module_b_path = src_dir.join("module_b.rs");
    let module_b_content = r#"
pub fn function_b() -> &'static str {
    "I'm function B"
}
"#;
    File::create(&module_b_path)
        .unwrap()
        .write_all(module_b_content.as_bytes())
        .unwrap();

    // Run the merge process
    let args = vec![
        "cargo".to_string(),
        "rustmerge".to_string(),
        "test_package".to_string(),
    ];
    std::env::set_current_dir(temp_dir.path()).unwrap();
    assert!(improved_rust_merge_tool::main().is_ok());

    // Check if the merged file exists
    let merged_file_path = temp_dir
        .path()
        .join("target")
        .join("test_package_merged.rs");
    assert!(merged_file_path.exists());

    // Read the merged file content
    let merged_content = fs::read_to_string(merged_file_path).unwrap();

    // Verify the merged content
    assert!(merged_content.contains("pub fn root_function()"));
    assert!(merged_content.contains("pub fn function_a()"));
    assert!(merged_content.contains("pub fn function_b()"));
    assert!(!merged_content.contains("#[cfg(test)]"));
    assert!(!merged_content.contains("#[test]"));
}

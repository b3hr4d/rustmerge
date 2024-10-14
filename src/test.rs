#[cfg(test)]
mod tests {
    use crate::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_determine_package() {
        let temp_dir = TempDir::new().unwrap();
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

        let args = vec!["cargo".to_string(), "rustmerge".to_string()];
        let (result, _) = determine_package(&temp_dir.path().to_path_buf(), &args).unwrap();
        assert_eq!(result, "test_package");

        let args_with_name = vec![
            "cargo".to_string(),
            "rustmerge".to_string(),
            "custom_name".to_string(),
        ];
        let (result_with_name, _) =
            determine_package(&temp_dir.path().to_path_buf(), &args_with_name).unwrap();
        assert_eq!(result_with_name, "custom_name");
    }

    #[test]
    fn test_find_src_dir() {
        let temp_dir = TempDir::new().unwrap();
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

        let result = find_src_dir(&temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(result, temp_dir.path().join("src"));

        // Test with custom src path
        let cargo_toml_content_custom_src = r#"
[package]
name = "test_package"
version = "0.1.0"
src = "custom_src"
"#;
        File::create(&cargo_toml_path)
            .unwrap()
            .write_all(cargo_toml_content_custom_src.as_bytes())
            .unwrap();

        let result_custom_src = find_src_dir(&temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(result_custom_src, temp_dir.path().join("custom_src"));
    }
}

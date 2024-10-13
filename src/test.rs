#[cfg(test)]
mod tests {
    use crate::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_determine_package_name() {
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
        let result = determine_package_name(temp_dir.path(), &args).unwrap();
        assert_eq!(result, "test_package");

        let args_with_name = vec![
            "cargo".to_string(),
            "rustmerge".to_string(),
            "custom_name".to_string(),
        ];
        let result_with_name = determine_package_name(temp_dir.path(), &args_with_name).unwrap();
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

        let result = find_src_dir(temp_dir.path(), "test_package").unwrap();
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

        let result_custom_src = find_src_dir(temp_dir.path(), "test_package").unwrap();
        assert_eq!(result_custom_src, temp_dir.path().join("custom_src"));
    }

    #[test]
    fn test_is_test_item() {
        let test_fn: Item = syn::parse_quote! {
            #[test]
            fn test_something() {}
        };
        assert!(is_test_item(&test_fn));

        let normal_fn: Item = syn::parse_quote! {
            fn normal_function() {}
        };
        assert!(!is_test_item(&normal_fn));

        let test_mod: Item = syn::parse_quote! {
            #[cfg(test)]
            mod tests {}
        };
        assert!(is_test_item(&test_mod));

        let normal_mod: Item = syn::parse_quote! {
            mod normal_module {}
        };
        assert!(!is_test_item(&normal_mod));
    }
}

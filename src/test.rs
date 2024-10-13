#[cfg(test)]
mod tests {
    use crate::*;
    use std::io::Read;
    use tempfile::tempdir;

    #[test]
    fn test_find_project_root_single_package() {
        let temp_dir = tempdir().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml_path,
            r#"
            [package]
            name = "test_package"
            version = "0.1.0"
            "#,
        )
        .unwrap();

        let project_info = find_project_root(temp_dir.path());
        assert_eq!(project_info.root, temp_dir.path());
        assert_eq!(
            project_info.packages,
            vec![("test_package".to_string(), temp_dir.path().to_path_buf())]
        );
        assert!(!project_info.is_workspace);
    }

    #[test]
    fn test_find_project_root_workspace() {
        let temp_dir = tempdir().unwrap();
        let workspace_toml_path = temp_dir.path().join("Cargo.toml");
        fs::write(
            &workspace_toml_path,
            r#"
            [workspace]
            members = ["member1", "member2"]
            "#,
        )
        .unwrap();

        let member1_dir = temp_dir.path().join("member1");
        fs::create_dir(&member1_dir).unwrap();
        fs::write(
            member1_dir.join("Cargo.toml"),
            r#"
            [package]
            name = "member1"
            version = "0.1.0"
            "#,
        )
        .unwrap();

        let member2_dir = temp_dir.path().join("member2");
        fs::create_dir(&member2_dir).unwrap();
        fs::write(
            member2_dir.join("Cargo.toml"),
            r#"
            [package]
            name = "member2"
            version = "0.1.0"
            "#,
        )
        .unwrap();

        let project_info = find_project_root(temp_dir.path());
        assert_eq!(project_info.root, temp_dir.path());
        assert_eq!(
            project_info.packages,
            vec![
                ("member1".to_string(), member1_dir),
                ("member2".to_string(), member2_dir)
            ]
        );
        assert!(project_info.is_workspace);
    }

    #[test]
    fn test_process_file() {
        let temp_dir = tempdir().unwrap();
        let input_file_path = temp_dir.path().join("input.rs");
        let output_file_path = temp_dir.path().join("output.rs");

        fs::write(
            &input_file_path,
            r#"
            #[cfg(test)]
            mod tests {
                #[test]
                fn test_example() {
                    assert_eq!(2 + 2, 4);
                }
            }

            fn example() {
                println!("Hello, world!");
            }
            "#,
        )
        .unwrap();

        let mut output_file = fs::File::create(&output_file_path).unwrap();

        let mut output_data = Vec::new();

        process_file(&input_file_path, &mut output_data);

        output_file.write_all(&output_data).unwrap();

        let mut output_content = String::new();
        fs::File::open(&output_file_path)
            .unwrap()
            .read_to_string(&mut output_content)
            .unwrap();

        assert!(output_content.contains("fn example()"));
        assert!(!output_content.contains("mod tests"));
    }

    #[test]
    fn test_get_package_root() {
        let temp_dir = tempdir().unwrap();
        let workspace_root = temp_dir.path();

        // Create a workspace structure
        fs::write(
            workspace_root.join("Cargo.toml"),
            r#"
            [workspace]
            members = ["package1", "package2"]
            "#,
        )
        .unwrap();

        let package1_dir = workspace_root.join("package1");
        fs::create_dir(&package1_dir).unwrap();
        fs::write(
            package1_dir.join("Cargo.toml"),
            r#"
            [package]
            name = "package1"
            version = "0.1.0"
            "#,
        )
        .unwrap();

        let package2_dir = workspace_root.join("package2");
        fs::create_dir(&package2_dir).unwrap();
        fs::write(
            package2_dir.join("Cargo.toml"),
            r#"
            [package]
            name = "package2"
            version = "0.1.0"
            "#,
        )
        .unwrap();

        let project_info = ProjectInfo {
            root: workspace_root.to_path_buf(),
            packages: vec![
                ("package1".to_string(), package1_dir.clone()),
                ("package2".to_string(), package2_dir.clone()),
            ],
            is_workspace: true,
        };

        // Test for a package in workspace
        let package_root = get_package_root(&project_info, "package1").unwrap();
        assert_eq!(package_root, package1_dir);

        // Test for a non-existent package
        let result = get_package_root(&project_info, "non_existent");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Package 'non_existent' not found in workspace"
        );
    }

    #[test]
    fn test_get_src_dir() {
        let temp_dir = tempdir().unwrap();
        let package_dir = temp_dir.path();

        // Test with default src directory
        fs::write(
            package_dir.join("Cargo.toml"),
            r#"
            [package]
            name = "test_package"
            version = "0.1.0"
            "#,
        )
        .unwrap();

        let src_dir = get_src_dir(package_dir);
        assert_eq!(src_dir, package_dir.join("src"));

        // Test with custom src directory
        fs::write(
            package_dir.join("Cargo.toml"),
            r#"
            [package]
            name = "test_package"
            version = "0.1.0"
            src = "custom_src"
            "#,
        )
        .unwrap();

        let src_dir = get_src_dir(package_dir);
        assert_eq!(src_dir, package_dir.join("custom_src"));
    }

    #[test]
    fn test_determine_package_name() {
        let temp_dir = tempdir().unwrap();
        let workspace_root = temp_dir.path();

        let project_info = ProjectInfo {
            root: workspace_root.to_path_buf(),
            packages: vec![
                ("package1".to_string(), workspace_root.join("package1")),
                ("package2".to_string(), workspace_root.join("package2")),
            ],
            is_workspace: true,
        };

        // Test with specified package name
        let args = vec![
            "cargo".to_string(),
            "rustmerge".to_string(),
            "package1".to_string(),
        ];
        assert_eq!(determine_package_name(&project_info, &args), "package1");

        // Test with single package in workspace
        let single_package_info = ProjectInfo {
            root: workspace_root.to_path_buf(),
            packages: vec![("package1".to_string(), workspace_root.join("package1"))],
            is_workspace: true,
        };
        let args = vec!["cargo".to_string(), "rustmerge".to_string()];
        assert_eq!(
            determine_package_name(&single_package_info, &args),
            "package1"
        );

        // Test with non-workspace project
        let non_workspace_info = ProjectInfo {
            root: workspace_root.to_path_buf(),
            packages: vec![("single_package".to_string(), workspace_root.to_path_buf())],
            is_workspace: false,
        };
        assert_eq!(
            determine_package_name(&non_workspace_info, &args),
            "single_package"
        );
    }
}

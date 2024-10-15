# rustmerge

`rustmerge` is a Cargo subcommand that merges all Rust source files in a package or workspace into single files. It works with both workspace projects and single-package projects.

Its primary use case is to simplify the process of sharing Rust projects with AI tools, e.g., for training machine learning models or code analysis. By merging all source files into a single file per package, you can easily share the project with tools that require a single file as input.

## Features

- Merges all `.rs` files in a package into a single file
- Works with both workspace and single-package projects
- Can process all packages in a workspace at once
- Excludes test modules from the merged output
- Maintains the module structure of the original project
- Preserves `cfg` attributes on modules
- Custom output path for merged files

## Installation

You can install `rustmerge` using Cargo:

```
cargo install rustmerge
```

## Usage

### In a single-package project:

```
cargo rustmerge
```

### In a workspace (specific package):

```
cargo rustmerge <package_name>
```

### Process all packages in a workspace:

```
cargo rustmerge --all
```

### Custom output path:

```
cargo rustmerge [<package_name>] --output <path>
```

If there's only one package in the workspace and you're not using `--all`, you can omit the package name.

By default, the merged Rust file(s) will be created in the `target` directory of your current working directory, named `rustmerge/<package_name>.rs`.

## Examples

1. Merge a single-package project:

   ```
   cd my-rust-project
   cargo rustmerge
   ```

2. Merge a specific package in a workspace:

   ```
   cd my-rust-workspace
   cargo rustmerge my-package
   ```

3. Merge all packages in a workspace:

   ```
   cd my-rust-workspace
   cargo rustmerge --all
   ```

4. Merge with a custom output path:

   ```
   cargo rustmerge --output /path/to/output/merged_project.rs
   ```

5. Merge all packages with a custom output directory:
   ```
   cargo rustmerge --all --output /path/to/output/dir
   ```

## Output

The tool will print information about the merged files, including their locations and sizes. For example:

```
Merged and formatted Rust program for package 'my-package' created in "/path/to/project/target/rustmerge/my-package.rs"
File size: 12345 bytes
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

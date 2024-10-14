# rustmerge

`rustmerge` is a Cargo subcommand that merges all Rust source files in a package into a single file. It works with both workspace projects and single-package projects.

Its primary use case is to simplify the process of sharing a Rust project with AI tools, e.g., for training machine learning models. By merging all source files into a single file, you can easily share the project with tools that require a single file as input.

## Features

- Merges all `.rs` files in a package into a single file
- Works with both workspace and single-package projects
- Excludes test modules from the merged output
- Maintains the module structure of the original project

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

### In a workspace:

```
cargo rustmerge [package_name]
```

If there's only one package in the workspace, you can omit the package name.

The merged Rust file will be created in the `target` directory of your current working directory, named `<package_name>_merged.rs`.

### Custom output path:

```
cargo rustmerge [<package_name>] --output path/to/output.rs
```

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

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

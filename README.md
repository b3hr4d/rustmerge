# rustmerge

`rustmerge` is a Cargo subcommand that merges all Rust source files in a package or workspace into single files. It works with both workspace projects and single-package projects.

Its primary use case is to simplify the process of sharing Rust projects with AI tools, e.g., for training machine learning models or code analysis. By merging all source files into a single file per package, you can easily share the project with tools that require a single file as input.

## Features

- Merges all `.rs` files in a package into a single file
- Works with both workspace and single-package projects
- Can process all packages in a workspace at once
- Excludes test modules (modules named `test` or `tests`) and any items (functions, structs, other modules, etc.) annotated with `#[cfg(test)]` from the merged output.
- Maintains the module structure of the original project
- Preserves `cfg` attributes on modules
- Custom output path for merged files
- Adds source file path comments for easy navigation

## How It Works

```
   +-------------+
   | Project     |
   | Structure   |
   +------+------+
          |
          v
   +------+------+      +------------+
   | Parse Module +---->             |
   | Structure    |     |  Extract   |
   +------+------+      |  Module    |
          |             |  Content   |
          v             |            |
   +------+------+      +------+-----+
   | Process     |            |
   | Modules     +------------+
   +------+------+
          |
          v
   +------+------+
   | Format and  |
   | Output      |
   +-------------+
```

`rustmerge` parses your Rust project's module structure, extracts the content of each module while preserving its hierarchy, processes all modules into a single merged file, and formats the output with proper file path comments.

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

## Module Structure Preservation

`rustmerge` preserves the module structure of your project and adds helpful file path comments to indicate the original location of each file in the merged output.

Consider this directory structure:

```
project/
├── src/
│   ├── main.rs
│   ├── module1.rs
│   └── service/
│       ├── mod.rs
│       └── module2.rs
```

The merged output would look like:

```rust
// main.rs
pub mod module1 {
    // module1.rs
    pub fn hello() {
        println!("Hello from module1");
    }
    pub mod submodule {
        pub fn nested_function() {
            println!("Nested function in module1");
        }
    }
}
use module1::submodule;
use service::module2;
pub mod service {
    // service/mod.rs
    pub mod module2 {
        // service/module2.rs
        pub fn hello() {
            println!("Hello from module2");
        }
    }
}
fn main() {
    module1::hello();
    module2::hello();
    submodule::nested_function();
}
```

Each file starts with a comment showing its path relative to the `src` directory. This makes it easy to understand the original project structure even after merging. Notice how nested modules like `service/mod.rs` and `service/module2.rs` correctly show their full relative path.

## Compatibility

`rustmerge` is compatible with:

- Rust 2018 and 2021 editions
- Workspace and single-package projects
- Projects using conditional compilation with `cfg` attributes
- Projects with nested module structures
- Both `lib.rs` and `main.rs` based crates

## Troubleshooting

### Missing modules

If some modules appear to be missing in the merged output:

1. Ensure that the module is properly declared with `mod module_name;` in your source code
2. Check if the module is conditionally compiled with `cfg` attributes
3. Verify that the module file exists in the expected location

### Formatting issues

If the merged file has formatting issues:

1. Ensure that `rustfmt` is installed and available in your PATH
2. Try formatting the output file manually: `rustfmt --edition=2021 output.rs`

### Known Limitations

- Test modules (named `test` or `tests`) are excluded from the merged output
- Very large projects might produce files that are difficult to navigate
- Some complex macro expansions might not be fully handled

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

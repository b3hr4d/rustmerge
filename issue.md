Not fixed, let me make more clear example I have this structure`lib/utils/string.rs` with this code on it

```rs
pub fn to_snake_case(input: String) -> String {
    input.to_case(Case::Snake)
}
```

And already put this

```rs
mod string;
pub use string::*;
```

Inside `lib/utils/mod.rs`.
When I merge this project using my merge tools it return this

```rs
pub mod utils {
    pub mod string {
        use convert_case::{Case, Casing};
        /// Converts a string to snake case.
        pub fn to_snake_case(input: String) -> String {
            input.to_case(Case::Snake)
        }
    }
}
```

but it should be

```rs
pub mod utils {
    use convert_case::{Case, Casing};
    /// Converts a string to snake case.
    pub fn to_snake_case(input: String) -> String {
        input.to_case(Case::Snake\*)
    }
}
```

how can I fix this issue?

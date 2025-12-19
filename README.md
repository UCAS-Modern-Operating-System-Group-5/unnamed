# Unnamed

# Development

Install [EditorConfig](https://editorconfig.org/) plugin for the editor you are using.

## Directory Structure

- `apps`: Applications
- `crates`: Libraries we made for this project 

## Commands

```
cargo run -p gui # Run apps/client
cargo run -- serve # Run apps/server. Equivalent to `cargo run -p server -- serve`
cargo test -p ipc # Test crates/ipc
```

If you are familiar with [just](https://github.com/casey/just?tab=readme-ov-file),
a command runner, you can use `just` to execute commands. For example:

```
just
just run -- serve
```

# Rust Learning Resources

- Google's [Comprehensive Rust](https://github.com/google/comprehensive-rust)
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/)

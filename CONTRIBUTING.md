# Contributing

Run the complete local checks before opening a pull request:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Platform input code must always fail open: if processing or output injection
cannot complete immediately, pass the original event through unchanged.

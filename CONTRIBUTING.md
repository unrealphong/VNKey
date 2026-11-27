# Contributing

Run the complete local checks before opening a pull request:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Platform input code must always fail open: if processing or output injection
cannot complete immediately, pass the original event through unchanged.

## Preparing a release

Update `workspace.package.version` in `Cargo.toml` when preparing a release,
then run `cargo check --workspace` to refresh `Cargo.lock`. Increment
`CFBundleVersion` in `packaging/macos/Info.plist` for the new macOS build.
The packaging script reads the release version from `vnkey --version` and
writes it to the bundle before signing. To distinguish a test build, set
`VNKEY_BUILD_NUMBER` when running `packaging/macos/build-app.sh`.

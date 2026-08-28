# Repository Guidelines

## Project Structure & Module Organization

This Rust 2024 workspace contains the public SDK in `crates/spin-sdk/` and its procedural macros in `crates/spin-sdk-macro/`. Capability modules such as HTTP, Redis, and SQLite live under `crates/spin-sdk/src/` and are feature-gated in the crate manifest. WIT definitions are under each crate’s `wit/` directory. Runnable Spin applications live in `examples/`; component fixtures used by SDK tests live in `crates/spin-sdk/test-cases/`. `adapters/` contains checked-in WebAssembly compatibility artifacts, while `scripts/` and `.github/workflows/` define repository automation.

## Build, Test, and Development Commands

- `cargo check --workspace` performs a fast compile check using default features.
- `cargo test --workspace` runs unit tests and Wasmtime-backed component tests.
- `cargo test --doc` validates Rust examples embedded in documentation.
- `cargo fmt --all -- --check` checks formatting without modifying files.
- `cargo clippy --workspace --all-targets -- -D warnings` applies the CI lint policy.
- `cargo check --no-default-features` verifies the minimal feature set. CI also checks isolated features with `cargo check --no-default-features --features json` and `cargo check --no-default-features --features postgres4-types`.
- `scripts/build_examples.sh` builds example applications with Spin. It requires `spin`, `protoc`, and the `wasm32-wasip1` and `wasm32-wasip2` Rust targets.

For interactive example development, run `spin build --up` inside an example directory such as `examples/hello-world/`.

## Coding Style & Naming Conventions

Use standard `rustfmt` output (four-space indentation). Follow Rust naming conventions: `snake_case` for modules, functions, and tests; `UpperCamelCase` for types and traits; `SCREAMING_SNAKE_CASE` for constants. Keep public APIs documented—the SDK denies missing documentation—and include compilable rustdoc examples for user-facing behavior. Keep feature-specific code behind the corresponding Cargo feature.

## Testing Guidelines

Place focused unit tests beside implementation code in `#[cfg(test)]` modules. Use descriptive behavior-oriented test names. Add or update a fixture in `crates/spin-sdk/test-cases/` when behavior crosses the component ABI, and build affected applications when changing examples or manifests. No coverage threshold is enforced; new behavior and regressions should still receive targeted tests.

## Commit & Pull Request Guidelines

Use short, concise commit subjects consistent with history, for example `Fix build-examples workflow`. Recent commits commonly include `Signed-off-by` trailers, but the repository does not document or enforce a DCO requirement. Keep pull requests focused; explain user-visible or API changes, link relevant issues, and note feature, WIT, or compatibility implications. Update documentation and examples with public API changes, and ensure formatting, Clippy, tests, docs, and affected example builds pass before requesting review.

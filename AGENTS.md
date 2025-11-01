# Agent Guidelines for rauncher-mc

## Build & Test Commands
- Build: `cargo build` or `cargo b`
- Run: `cargo run`
- Check (fast): `cargo check` or `cargo c`
- Test all: `cargo test`
- Test single: `cargo test test_name` or `cargo test --package rc-core --lib test_name`
- Lint: `cargo clippy`
- Format: `cargo fmt`

## Project Structure
- Workspace with main binary (`src/`) and `rc-core` crate (`crates/rc-core/`)
- GPUI-based UI framework using `gpui-component` library
- Edition 2024, Apache-2.0 license

## Code Style
- **Imports**: Group std, external crates, then local modules. Use explicit imports from `gpui` and `gpui_component`.
- **Types**: Use strong typing with `#[derive(Debug, Clone, PartialEq, Eq)]` for data structures. Prefer `anyhow::Error` for error handling.
- **Naming**: snake_case for functions/variables, PascalCase for types/structs, SCREAMING_SNAKE_CASE for constants.
- **Components**: Implement `Render` trait for UI components. Use builder pattern for component configuration (e.g., `Button::new().icon().primary()`).
- **Error Handling**: Use `anyhow::Error` and `?` operator. Return `Result<_, anyhow::Error>` from async functions.
- **Async**: Use `cx.spawn()` for async operations with `.detach()` for fire-and-forget tasks.

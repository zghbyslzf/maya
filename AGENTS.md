# AGENTS.md

## Build Commands

```bash
cargo build              # Dev build
cargo build --release   # Release build
cargo test              # Unit tests
cargo test --test integration_tests  # Integration tests
cargo clippy -- -D warnings  # Linting
```

## Special Builds

- **Release with FFmpeg**: `MAYAA_INCLUDE_FFMPEG=1 cargo build --release`
- **Auto-increment version**: `MAYAA_AUTO_INCREMENT_VERSION=1 MAYAA_INCLUDE_FFMPEG=1 cargo build --release`

## Architecture

- Cargo workspace: `crates/*` with 8 sub-crates + `src/main.rs`
- CLI framework: `clap` (derive macros)
- Main entrypoint: `src/main.rs` routes to modules in `src/modules/`
- Windows-only (no cross-platform handling needed)

## Testing

- Unit tests: `cargo test`
- Integration tests: `cargo test --test integration_tests`
- Uses `assert_cmd`, `predicates`, `tempfile` for test fixtures

## Project-Specific Conventions

- Release profile uses `-Oz` + LTO for minimal binary size
- All crates share dependencies via workspace `[workspace.dependencies]`
- `src/modules/*.rs` each handle a CLI subcommand group
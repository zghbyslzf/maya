# AGENTS.md

- 输入和输出全部使用中文

## Build Commands

```bash
cargo build             # Dev build
cargo build --release   # Release build
cargo test              # All tests (unit + integration)
cargo test --test integration_tests  # Integration tests only
cargo clippy -- -D warnings  # Lint
```

## Release Packaging

Uses `cargo-make` (Makefile.toml). The `build` task auto-increments the version in root `Cargo.toml`, then builds, copies binaries + FFmpeg into `pkg/release/`, increments `pkg/package.json`, and publishes to npm:

```bash
cargo make build        # build + version bump + package
cargo make publish-all  # full pipeline through npm publish
```

- FFmpeg is **auto-downloaded at runtime** by `mp4_to_m3u8` via the `ffmpeg-sidecar` crate — no manual install needed.
- The `FFmpeg/` directory at repo root is only used by cargo-make to bundle `ffmpeg.exe` and `ffprobe.exe` for npm distribution; it has no effect on `cargo build`.

## Architecture

- Cargo workspace: 8 crates in `crates/` + binary in `src/`
- CLI framework: `clap` 4.4 with derive macros
- Main entrypoint: `src/main.rs` → dispatches to `src/modules/*.rs`
- `src/modules/*.rs` are thin dispatchers; actual logic lives in `crates/*`
- `maya_common` (shared lib) defines the unified `Error` enum (thiserror) and `Result<T>` alias; downstream crates enable features: `anyhow`, `tokio`, `parallel`
- Windows-only — no cross-platform handling

## Subcommand Cheat Sheet

CLI uses subcommands, not single-letter flags. README examples (`maya -c n`, `maya -g m`, etc.) are **outdated**.

```bash
maya clean    --types n           # node_modules   (aliases: n, node_modules)
maya clean    --types lock        # lock files
maya git      --ops m             # git add . && commit && push
maya pack     --type g            # zip respecting .gitignore
maya pack     --type a            # zip Vite outDir
maya optimize --types all         # compress images (prefix n for new-file mode)
maya transform --types mp4 m3u8   # mp4 → m3u8
```

## Conventions

- Release profile: `-Oz` + LTO + single codegen unit + stripped symbols
- Workspace deps: shared deps defined in root `[workspace.dependencies]`, crates reference via `{ workspace = true }`
- Exception: `compress_pictures` uses `path = "../maya_common"` instead of `{ workspace = true }` — if an agent standardizes this, verify it compiles
- Some crates declare their own external deps (e.g., `indicatif`, `oxipng`, `image`, `ffmpeg-sidecar`) — check individual crate `Cargo.toml` before adding new deps
- No CI/CD, no `.github` directory, no pre-commit hooks

## Testing

- Framework: `assert_cmd` + `predicates` + `tempfile`
- Tests compile the binary via `Command::cargo_bin("maya")` — first `cargo test` on a clean clone will take a full build cycle

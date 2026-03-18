# Development

## Repository Layout

```text
llmfit-core/      Core mining, power, hardware, cache, and ranking logic
llmfit-tui/       Terminal UI, CLI, persistence, and app shell
llmfit-desktop/   Experimental desktop shell
bin/              npm wrapper entrypoint
assets/github/    README logos and presentation assets
.github/          CI, release automation, and contribution templates
```

## Build

```powershell
cargo build -p minefit --manifest-path .\Cargo.toml
```

## Test

```powershell
cargo test --manifest-path .\Cargo.toml
```

## Format

```powershell
cargo fmt --all
```

## Lint

```powershell
cargo clippy --all-targets --all-features
```

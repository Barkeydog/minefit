# Contributing to minefit

Thanks for contributing.

## Development Workflow

1. Create a branch from `main`.
2. Make focused changes with clear commit messages.
3. Run the local verification steps before opening a PR.
4. Include screenshots or terminal output for visible UI changes when relevant.

## Local Verification

```powershell
cargo fmt --all
cargo clippy --all-targets --all-features
cargo test --manifest-path .\Cargo.toml
cargo build -p minefit --manifest-path .\Cargo.toml
```

For runtime verification, it is also useful to run:

```powershell
minefit
minefit --cli -n 12
minefit --json -n 25
```

## Scope Guidelines

- Keep mining logic changes realistic and traceable to the underlying assumptions.
- Prefer improving confidence and transparency over inflating projected profitability.
- Flag inferred behavior clearly in the UI and output.
- Avoid introducing provider lock-in when a public-source fallback is feasible.

## Pull Request Checklist

- The change is documented if user-visible behavior changed.
- Tests were added or updated where practical.
- The TUI still behaves correctly at narrow, normal, and ultrawide widths if layout changed.
- Any feed or pricing assumption changes are explained in the PR description.

## Communication

For bugs and feature requests, use the GitHub issue templates.

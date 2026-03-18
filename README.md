# minefit

[![CI](https://github.com/Barkeydog/minefit/actions/workflows/ci.yml/badge.svg)](https://github.com/Barkeydog/minefit/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-0f172a.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-f97316.svg)](https://www.rust-lang.org/)

`minefit` is a terminal-first mining comparison tool for evaluating live coin opportunities against the hardware that is actually on the current machine.

It combines local CPU/GPU detection, live mining feeds, electricity-rate estimation, and method-level profitability modeling into a single TUI, CLI, and JSON workflow.

## Highlights

- Local-first comparison scope using the detected CPU and GPU on the current system.
- Live mining coverage from WhatToMine, Hashrate.no, MiningPoolStats, and Coinbase spot pricing.
- Discovery catalog fallback that keeps thousands of assets rankable even when a market feed is rate-limited.
- Utility-aware electricity modeling with California tariff support and U.S. state fallback.
- GPU, CPU, and ASIC benchmark profiles with hashrate, power, reject-rate, and tuning assumptions.
- 40+ modeled techniques spanning pool, solo, hosted, opportunistic, and efficiency-focused strategies.
- Eligibility checks for algorithm support, VRAM pressure, backend fit, and benchmark coverage.
- Solo variance modeling with zero-block odds and p50/p90 monthly outcomes.
- Persistent state for filters, sorting, power context, and last-viewed layout.
- Cache-backed startup snapshots for faster boot and graceful degradation when feeds fail.

## Quick Start

Run from source:

```powershell
cargo run -p minefit --manifest-path .\Cargo.toml --
```

Install the local binary:

```powershell
cargo install --path .\llmfit-tui --force
minefit
```

Use the npm wrapper:

```powershell
npm install -g minefit
minefit --cli -n 12
```

## Modes

Interactive TUI:

```powershell
minefit
```

Classic CLI table:

```powershell
minefit --cli -n 12
```

Structured JSON:

```powershell
minefit --json -n 25
```

Useful overrides:

```powershell
minefit --power-plan pge-e-tou-c
minefit --location WA
minefit --electricity 0.16
minefit --memory 24G
```

## What The Rankings Mean

`minefit` mixes two classes of rows:

- Tier-one mining rows backed by real mining telemetry and validated algorithm benchmarks.
- Discovery rows backed by inferred `Discovery Proxy` economics so the long tail stays sortable instead of disappearing.

The ranking model applies:

- gross revenue
- power cost
- fees and stale-share drag
- service drag for hosted strategies
- liquidity penalties
- trend and volatility adjustments
- fit and benchmark confidence

The result is meant to be operationally useful, not just theoretically profitable.

## Data Model Notes

- BTC can appear on CPU and GPU through software SHA256 paths, but those rows are usually economically negative.
- Discovery rows are intentionally penalized relative to validated mining rows.
- Electricity defaults to an estimated local context from your detected location and can be overridden explicitly.
- Snapshot cache and state live under `~/.config/minefit/`.

## Repository Layout

```text
llmfit-core/      Core mining, power, hardware, and ranking logic
llmfit-tui/       Terminal UI, CLI, persistence, and app shell
llmfit-desktop/   Experimental desktop shell
bin/              npm wrapper entrypoint
.github/          CI, release automation, templates
```

## Development

Build:

```powershell
cargo build -p minefit --manifest-path .\Cargo.toml
```

Test:

```powershell
cargo test --manifest-path .\Cargo.toml
```

Format:

```powershell
cargo fmt --all
```

Lint:

```powershell
cargo clippy --all-targets --all-features
```

## Documentation

- [API.md](./API.md)
- [CONTRIBUTING.md](./CONTRIBUTING.md)
- [SECURITY.md](./SECURITY.md)

## Project Status

`minefit` is an actively maintained mining-focused fork with a stable public TUI, CLI, and JSON interface.

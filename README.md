# minefit

`minefit` is a local fork of the `llmfit` TUI, repurposed for live crypto-mining comparisons.

## What It Models

- Live tier-one coin data from WhatToMine, Hashrate.no, and MiningPoolStats, with BTC spot from Coinbase.
- Utility-specific residential tariffs for supported California utilities, with EIA state-average fallback.
- Real rig profiles for selected GPUs, ASICs, and CPUs, including tuned hashrate, watt draw, and reject-rate assumptions.
- Pool-specific fee, stale-share, and uptime drag.
- Coin eligibility checks for VRAM/DAG pressure, backend/vendor fit, and benchmark coverage.
- Solo-mining variance, including zero-block odds and p50/p90 monthly outcomes.
- Liquidity-aware cashflow caps so low-volume coins do not outrank deep markets on impossible exit assumptions.

## Run

From the repo root:

```powershell
cargo run -p minefit --manifest-path .\Cargo.toml --
```

`minefit` uses your detected local GPU and CPU only as the default comparison scope.

Common flags:

```powershell
cargo run -p minefit --manifest-path .\Cargo.toml -- --cli -n 12
cargo run -p minefit --manifest-path .\Cargo.toml -- --json -n 10
cargo run -p minefit --manifest-path .\Cargo.toml -- --power-plan pge-e-tou-c
cargo run -p minefit --manifest-path .\Cargo.toml -- --location WA
cargo run -p minefit --manifest-path .\Cargo.toml -- --electricity 0.16
```

Use `minefit --help` after installing the binary for the full flag list.

## npm Package

`minefit` is also packaged for npm. The npm package wraps the Rust source and invokes Cargo locally, so a working Rust toolchain is still required.

```powershell
npm install -g minefit
minefit --cli -n 12
npx minefit --json -n 20
```

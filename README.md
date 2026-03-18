# minefit

`minefit` is a local fork of the `llmfit` TUI, repurposed for live crypto-mining comparisons.

## What It Models

- Live tier-one mining discovery from WhatToMine, Hashrate.no, and MiningPoolStats, with BTC spot from Coinbase. Current public snapshots are now clearing 100+ rankable mining coins and include BTC again.
- A bulk discovery catalog from CoinPaprika so the app can ingest 10k+ active crypto assets while keeping the mining comparison matrix limited to assets with real mining inputs.
- Utility-specific residential tariffs for supported California utilities, with EIA state-average fallback.
- Real rig profiles for selected GPUs, ASICs, and CPUs, including tuned hashrate, watt draw, and reject-rate assumptions.
- 40+ modeled mining techniques across pool, marketplace, hosted, eco, windowed, and solo strategies.
- Pool-specific fee, stale-share, duty-cycle, tuning, and uptime drag.
- Coin eligibility checks for VRAM/DAG pressure, backend/vendor fit, and benchmark coverage.
- Solo-mining variance, including zero-block odds and p50/p90 monthly outcomes.
- Liquidity-aware cashflow caps so low-volume coins do not outrank deep markets on impossible exit assumptions.
- Persistent local state for power context, filters, sort mode, and last-viewed layout.
- Snapshot caching plus timestamped startup archives under `~/.config/minefit/cache` so boot is faster and feed failures can fall back cleanly.
- Software SHA256 benchmarks for CPU and GPU so BTC can show on the current machine as a theoretical, usually uneconomic path.

## Run

From the repo root:

```powershell
cargo run -p minefit --manifest-path .\Cargo.toml --
```

`minefit` uses your detected local GPU and CPU only as the default comparison scope.

State and cache:

```text
~/.config/minefit/state.json
~/.config/minefit/cache/latest.json
~/.config/minefit/cache/snapshots/
```

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

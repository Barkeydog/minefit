# minefit

Terminal UI for comparing live mineable coins, local rig economics, and power-aware mining strategies.

## Capabilities

- Live mining coverage from WhatToMine, Hashrate.no, MiningPoolStats, and Coinbase spot pricing.
- Discovery fallback so thousands of assets remain sortable through inferred `Discovery Proxy` rows.
- Local CPU and GPU detection with hardware-aware benchmarks and eligibility checks.
- Persistent state in `~/.config/minefit/state.json`.
- Cache-backed startup snapshots under `~/.config/minefit/cache/`.

## Usage

```powershell
minefit
minefit --cli -n 12
minefit --json -n 25
minefit --location WA
minefit --electricity 0.16
```

## Development

From the repository root:

```powershell
cargo run -p minefit --manifest-path .\Cargo.toml --
```

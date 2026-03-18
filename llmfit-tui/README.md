# minefit

Mining coin and method comparison in the terminal, built from the `llmfit` TUI codebase.

The live snapshot now combines WhatToMine, Hashrate.no, and MiningPoolStats so the default feed covers 100+ tier-one rankable mining coins on current public data, including BTC. It also carries a CoinPaprika-backed discovery catalog of 10k+ active assets. Ranking defaults to your detected local CPU and GPU only.

This build also adds:
- 40+ modeled techniques across pool, marketplace, hosted, eco, windowed, and solo paths.
- Persistent sort/filter/power/layout state in `~/.config/minefit/state.json`.
- Warm-start cache and archived startup snapshots in `~/.config/minefit/cache/`.
- Software SHA256 benchmarks so BTC can appear on CPU and GPU as a theoretical, usually uneconomic route.

## Usage

```powershell
minefit
minefit --cli -n 12
minefit --json -n 25
minefit --location WA
minefit --electricity 0.16
```

## Local Development

From the repository root:

```powershell
cargo run -p minefit --manifest-path .\minefit-tui\Cargo.toml --
```

# minefit

Mining coin and method comparison in the terminal, built from the `llmfit` TUI codebase.

The live snapshot now combines WhatToMine, Hashrate.no, and MiningPoolStats so the default feed covers 100+ tier-one coins on current public data, including BTC. Ranking defaults to your detected local CPU and GPU only.

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

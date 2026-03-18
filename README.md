<div align="center">
  <a href="https://github.com/Barkeydog/minefit">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/github/logo-dark.svg">
      <source media="(prefers-color-scheme: light)" srcset="assets/github/logo-light.svg">
      <img alt="minefit logo" src="assets/github/logo-dark.svg" width="78%">
    </picture>
  </a>
</div>

<div align="center">
  <h3>Terminal-first crypto mining comparison for the hardware you actually have.</h3>
</div>

<div align="center">
  <a href="https://github.com/Barkeydog/minefit/actions/workflows/ci.yml"><img src="https://github.com/Barkeydog/minefit/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://www.npmjs.com/package/minefit"><img src="https://img.shields.io/npm/v/minefit?color=cb3837" alt="npm version"></a>
  <a href="https://www.npmjs.com/package/minefit"><img src="https://img.shields.io/npm/dm/minefit?color=0f766e" alt="npm downloads per month"></a>
  <a href="./LICENSE"><img src="https://img.shields.io/badge/license-MIT-0f172a.svg" alt="MIT License"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-1.85%2B-f97316.svg" alt="Rust"></a>
  <a href="https://github.com/Barkeydog/minefit/stargazers"><img src="https://img.shields.io/github/stars/Barkeydog/minefit?style=flat" alt="GitHub stars"></a>
</div>

<div align="center">
  <a href="https://github.com/Barkeydog/minefit/releases"><img src="https://img.shields.io/github/v/release/Barkeydog/minefit?display_name=tag&color=2563eb" alt="GitHub release"></a>
  <a href="https://github.com/Barkeydog/minefit/pkgs/container/minefit"><img src="https://img.shields.io/badge/ghcr-minefit-111827" alt="GHCR image"></a>
  <a href="https://github.com/Barkeydog/minefit/releases"><img src="https://img.shields.io/badge/platforms-win%20%7C%20macOS%20%7C%20linux-0f766e" alt="Supported platforms"></a>
  <a href="https://github.com/Barkeydog/minefit"><img src="https://img.shields.io/badge/interfaces-TUI%20%7C%20CLI%20%7C%20JSON-7c3aed" alt="Interfaces"></a>
  <a href="https://github.com/Barkeydog/minefit/blob/main/README.md"><img src="https://img.shields.io/badge/install-npm%20preferred-c2410c" alt="Install method"></a>
</div>

<br>

`minefit` is a mining-focused fork of `llmfit` that turns a fast terminal UI into a live mining decision surface. It detects the local CPU and GPU, estimates electricity from the current location, pulls live coin and market data, and ranks coins and methods against real power drag instead of fantasy hashrates.

The goal is operational usefulness. `minefit` is built to answer a narrower question than a generic portfolio tracker: *what can this machine mine, what does power do to the economics, and what method looks best right now?*

> [!NOTE]
> The current default scope is the local system only. `minefit` detects the CPU and GPU on the current machine and ranks mining setups against that hardware automatically.

---

## Quick Start

![minefit install card](assets/github/install-card.svg)

## Why minefit

- Local-first by default. The app models the CPU and GPU on the current machine instead of asking the user to assemble a fake rig.
- Power-aware ranking. Electricity is part of the default math, including utility-aware California TOU modeling and U.S. state fallback.
- Live mining data. Rankings are fed by WhatToMine, Hashrate.no, MiningPoolStats, Coinbase spot, and discovery catalog enrichment.
- Multi-surface workflow. The same ranking engine is available as a full TUI, a classic terminal table, and a JSON output path.
- Realism over hype. Methods include fees, stale/reject drag, uptime assumptions, service costs, eligibility checks, and solo variance.

## What It Models

- Local CPU and GPU detection, including backend hints and memory context.
- Utility-aware electricity estimation from the current location, with explicit manual overrides.
- Tier-one mining rows backed by live telemetry and benchmarked algorithm support.
- Discovery coverage for the long tail of assets, so catalog assets do not vanish when feed quality drops.
- GPU, CPU, and ASIC-oriented techniques across pool, solo, hosted, and efficiency-focused strategies.
- Coin eligibility checks for backend fit, VRAM pressure, benchmark coverage, and algorithm support.
- Solo variance signals including p50 and p90 monthly outcomes plus zero-block risk.
- Persistent app state and cached startup snapshots under `~/.config/minefit/`.

## Ranking Model

`minefit` blends market opportunity with operational drag. A row score is influenced by:

- gross daily revenue
- electricity cost
- pool fees and stale-share drag
- runtime and uptime assumptions
- service cost for hosted strategies
- liquidity and confidence penalties
- trend and volatility adjustments
- fit between the coin, method, and available hardware

This means a row can appear with a negative net return if it is technically possible but economically weak. That is intentional. For example, BTC can show up on CPU or GPU through software SHA256 paths even though those rows are usually not viable in practice.

## Data Sources

`minefit` currently draws on a mix of live mining, benchmark, and market sources:

- [WhatToMine](https://whattomine.com/coins.json)
- [Hashrate.no](https://www.hashrate.no/)
- [MiningPoolStats](https://miningpoolstats.stream/)
- [Coinbase spot prices](https://www.coinbase.com/)
- [CoinPaprika discovery catalog](https://docs.coinpaprika.com/api-reference/coins/list-coins)

When a source is unavailable or rate-limited, cached snapshots are used so startup stays fast and the app degrades cleanly instead of failing hard.

[![Star History Chart](https://api.star-history.com/svg?repos=Barkeydog/minefit&type=Date)](https://www.star-history.com/#Barkeydog/minefit&Date)

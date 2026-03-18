# Ranking Model Diagram Experiments

This file explores alternatives to the current `Ranking Model` section in `README.md`.

Current problem:
- the text is accurate
- the list is clear
- but it reads like raw documentation instead of a quick mental model

The goal of these variants is to explain:
- what feeds the score
- why profitability is not the only factor
- why technically possible rows can still rank poorly

## Variant 1: Simple flowchart

```mermaid
flowchart LR
    A[Market Opportunity] --> E[Final Score]
    B[Power Cost] --> E
    C[Operational Drag] --> E
    D[Hardware Fit] --> E

    A1[Gross daily revenue] --> A
    A2[Trend and volatility] --> A
    A3[Liquidity and confidence] --> A

    B1[Electricity cost] --> B
    B2[Runtime and uptime] --> B

    C1[Pool fees] --> C
    C2[Stale and reject drag] --> C
    C3[Hosted service cost] --> C

    D1[Coin / method / hardware fit] --> D
```

Why it works:
- easiest to understand
- good README fit
- shows that score is multi-input, not just profit

Weakness:
- still a little abstract

## Variant 2: Positive vs negative forces

```mermaid
flowchart TB
    A[What pushes a setup up] --> E[Score]
    B[What pushes a setup down] --> E
    C[What determines viability] --> E

    A1[Gross revenue] --> A
    A2[Trend strength] --> A
    A3[Liquidity] --> A

    B1[Electricity cost] --> B
    B2[Pool fees] --> B
    B3[Stale and reject drag] --> B
    B4[Hosted service cost] --> B

    C1[Runtime and uptime assumptions] --> C
    C2[Confidence penalties] --> C
    C3[Hardware and method fit] --> C
```

Why it works:
- strong mental framing
- makes the tradeoffs obvious

Weakness:
- less literal than the underlying ranking pipeline

## Variant 3: Funnel diagram

```mermaid
flowchart TB
    A[All technically possible setups]
    B[Apply hardware and method fit]
    C[Apply revenue and power math]
    D[Apply fees, stale drag, and service cost]
    E[Apply liquidity, confidence, trend, and volatility]
    F[Final ranked results]

    A --> B --> C --> D --> E --> F
```

Suggested supporting sentence:

> A setup can survive the technical filter and still rank badly once power, fees, confidence, and market quality are applied.

Why it works:
- very clean
- explains why “possible” does not mean “good”

Weakness:
- hides the specific factors unless paired with short text

## Variant 4: Score equation map

```mermaid
flowchart LR
    A[Gross revenue] --> S[Score]
    B[Electricity cost] --> S
    C[Fees and stale drag] --> S
    D[Runtime and uptime] --> S
    E[Hosted service cost] --> S
    F[Liquidity and confidence] --> S
    G[Trend and volatility] --> S
    H[Hardware fit] --> S
```

Suggested supporting sentence:

> `minefit` scores a setup by balancing raw earnings against power, execution drag, market quality, and hardware fit.

Why it works:
- closest to the current list
- very compact

Weakness:
- visually flatter than the stronger concept diagrams

## Variant 5: Why BTC can appear but still rank badly

```mermaid
flowchart TB
    A[Technically possible]
    B[Economically weak]
    C[Still shown in results]

    A1[CPU or GPU can run software SHA256] --> A
    B1[Low output] --> B
    B2[Real power cost] --> B
    B3[Poor competitiveness vs ASICs] --> B

    A --> C
    B --> C
```

Suggested supporting sentence:

> `minefit` does not hide technically valid setups just because they are bad bets. A CPU or GPU BTC path can appear, but rank poorly for economic reasons.

Why it works:
- directly explains the most confusing example
- makes the design philosophy obvious

Weakness:
- too specific to replace the whole section by itself

## Recommendation

Best overall:
- **Variant 2** if you want the clearest conceptual explanation
- **Variant 3** if you want the cleanest README presentation

Best combined solution:
- use **Variant 3** as the diagram
- keep one short paragraph underneath explaining that technically possible rows can still rank badly

## Suggested replacement

```mermaid
flowchart TB
    A[All technically possible setups]
    B[Apply hardware and method fit]
    C[Apply revenue and power math]
    D[Apply fees, stale drag, and service cost]
    E[Apply liquidity, confidence, trend, and volatility]
    F[Final ranked results]

    A --> B --> C --> D --> E --> F
```

`minefit` blends market opportunity with operational drag. A setup can survive the technical filter and still rank badly once power, fees, confidence, and market quality are applied. That is intentional. For example, BTC can show up on CPU or GPU through software SHA256 paths even when those setups are not economically viable in practice.

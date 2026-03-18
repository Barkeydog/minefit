# Why minefit Formatting Variants

This file prototypes different formatting treatments for the same `Why minefit` content.

The wording stays effectively the same. Only presentation changes.

---

## Variant 1: Current bullet list

### Why minefit

- Local-first by default. The app models the CPU and GPU on the current machine instead of asking the user to assemble a fake rig.
- Power-aware ranking. Electricity is part of the default math, including utility-aware California TOU modeling and U.S. state fallback.
- Live mining data. Rankings are fed by [WhatToMine](https://whattomine.com/coins.json), [Hashrate.no](https://www.hashrate.no/), [MiningPoolStats](https://miningpoolstats.stream/), [Coinbase spot](https://www.coinbase.com/), and discovery catalog enrichment.
- Multi-surface workflow. The same ranking engine is available as a full TUI, a classic terminal table, and a JSON output path.
- Realism over hype. Methods include fees, stale/reject drag, uptime assumptions, service costs, eligibility checks, and solo variance.

Good:
- simple
- fast to scan

Weakness:
- visually generic

---

## Variant 2: Bold lead labels

### Why minefit

`Local-first by default`
The app models the CPU and GPU on the current machine instead of asking the user to assemble a fake rig.

`Power-aware ranking`
Electricity is part of the default math, including utility-aware California TOU modeling and U.S. state fallback.

`Live mining data`
Rankings are fed by [WhatToMine](https://whattomine.com/coins.json), [Hashrate.no](https://www.hashrate.no/), [MiningPoolStats](https://miningpoolstats.stream/), [Coinbase spot](https://www.coinbase.com/), and discovery catalog enrichment.

`Multi-surface workflow`
The same ranking engine is available as a full TUI, a classic terminal table, and a JSON output path.

`Realism over hype`
Methods include fees, stale/reject drag, uptime assumptions, service costs, eligibility checks, and solo variance.

Good:
- more premium
- reads like product writing

Weakness:
- taller section

---

## Variant 3: Two-column table

### Why minefit

| Principle | Why it matters |
| --- | --- |
| Local-first by default | The app models the CPU and GPU on the current machine instead of asking the user to assemble a fake rig. |
| Power-aware ranking | Electricity is part of the default math, including utility-aware California TOU modeling and U.S. state fallback. |
| Live mining data | Rankings are fed by [WhatToMine](https://whattomine.com/coins.json), [Hashrate.no](https://www.hashrate.no/), [MiningPoolStats](https://miningpoolstats.stream/), [Coinbase spot](https://www.coinbase.com/), and discovery catalog enrichment. |
| Multi-surface workflow | The same ranking engine is available as a full TUI, a classic terminal table, and a JSON output path. |
| Realism over hype | Methods include fees, stale/reject drag, uptime assumptions, service costs, eligibility checks, and solo variance. |

Good:
- structured
- very readable

Weakness:
- slightly more documentation-like

---

## Variant 4: Card-style subheadings

### Why minefit

#### Local-first by default
The app models the CPU and GPU on the current machine instead of asking the user to assemble a fake rig.

#### Power-aware ranking
Electricity is part of the default math, including utility-aware California TOU modeling and U.S. state fallback.

#### Live mining data
Rankings are fed by [WhatToMine](https://whattomine.com/coins.json), [Hashrate.no](https://www.hashrate.no/), [MiningPoolStats](https://miningpoolstats.stream/), [Coinbase spot](https://www.coinbase.com/), and discovery catalog enrichment.

#### Multi-surface workflow
The same ranking engine is available as a full TUI, a classic terminal table, and a JSON output path.

#### Realism over hype
Methods include fees, stale/reject drag, uptime assumptions, service costs, eligibility checks, and solo variance.

Good:
- strongest visual hierarchy
- feels deliberate

Weakness:
- a little too tall for this amount of copy

---

## Variant 5: Inline emphasis bullets

### Why minefit

- **Local-first by default:** The app models the CPU and GPU on the current machine instead of asking the user to assemble a fake rig.
- **Power-aware ranking:** Electricity is part of the default math, including utility-aware California TOU modeling and U.S. state fallback.
- **Live mining data:** Rankings are fed by [WhatToMine](https://whattomine.com/coins.json), [Hashrate.no](https://www.hashrate.no/), [MiningPoolStats](https://miningpoolstats.stream/), [Coinbase spot](https://www.coinbase.com/), and discovery catalog enrichment.
- **Multi-surface workflow:** The same ranking engine is available as a full TUI, a classic terminal table, and a JSON output path.
- **Realism over hype:** Methods include fees, stale/reject drag, uptime assumptions, service costs, eligibility checks, and solo variance.

Good:
- probably the best balance
- keeps the speed of bullets
- adds stronger visual rhythm

Weakness:
- less distinctive than tables or cards

---

## Recommendation

Best overall: **Variant 5**

Why:
- same content
- clearer visual structure
- does not take much more space
- still feels like a polished README instead of internal docs

Best if you want a more formal/product feel: **Variant 2**

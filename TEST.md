# Product Surface Experiments

This file explores alternatives to the current `Product Surface` section in `README.md`.

## Is the current section useful?

Short answer: **somewhat, but it can be better**.

What works:
- It tells the reader that `minefit` has three interfaces: TUI, CLI, and JSON.
- It gives concrete commands instead of abstract feature names.
- It is compact.

What does not work:
- `Product Surface` is vague. It reads like internal product language, not user-facing README language.
- The table explains interfaces, but not when a reader should choose one over another.
- The copy focuses on output format, not task intent.
- The first row is weaker than it should be because `minefit` alone is doing a lot of work and the description is slightly abstract.

The pattern that looks strongest in other projects is: **command first, intent second**.

Relevant examples:
- `rdme` uses a direct `Quick Start` with install, then immediate commands tied to specific tasks.
  Source: https://github.com/readmeio/rdme
- `docker-mcp-server` uses install plus “Try It Out” commands with plain descriptions of what each command is for.
  Source: https://github.com/0xshariq/docker-mcp-server
- `HuggingFaceModelDownloader` separates TUI-style behavior and JSON/CLI behavior more explicitly, which makes mode choice clearer.
  Source: https://github.com/bodaay/HuggingFaceModelDownloader

## Variant A: Rename only

This is the smallest improvement. Keep the structure, but use a clearer section name.

### Interfaces

| Command | What it is for |
| --- | --- |
| `minefit` | Full-screen terminal interface for browsing opportunities and inspecting why a setup ranks the way it does |
| `minefit --cli -n 12` | Fast terminal table for quick checks, SSH sessions, and shell workflows |
| `minefit --json -n 25` | Structured output for scripts, automation, and downstream analysis |

Why this is better:
- `Interfaces` is clearer than `Product Surface`.
- Very low-risk change.

Why this is still limited:
- It still feels a bit documentation-heavy rather than user-guiding.

## Variant B: Intent-first table

This is the most practical replacement.

### Choose Your Mode

| If you want to... | Run this |
| --- | --- |
| browse the best opportunities interactively | `minefit` |
| check the current ranking quickly in a shell | `minefit --cli -n 12` |
| feed results into scripts or other tools | `minefit --json -n 25` |

Why this is better:
- It starts with the user’s goal instead of the product’s structure.
- It answers the question “which one should I use?”
- It reads more naturally in a README.

Tradeoff:
- Slightly less explicit about TUI/CLI/JSON naming.

## Variant C: Command cards

This is closer to how polished CLI READMEs often present the first few commands.

### Common Ways to Use minefit

#### Interactive terminal view

```powershell
minefit
```

Best when you want to explore rankings, change focus quickly, and inspect details in context.

#### Quick shell snapshot

```powershell
minefit --cli -n 12
```

Best when you want a fast table in a terminal, over SSH, or in a quick manual workflow.

#### Structured output

```powershell
minefit --json -n 25
```

Best when you want to pipe results into scripts, dashboards, or other automation.

Why this is better:
- More polished and readable than a table.
- Gives each mode a little breathing room.
- Fits a tool with distinct usage styles.

Tradeoff:
- Takes more vertical space.

## Variant D: Try it out

This borrows more from the `docker-mcp-server` style.

### Try It Out

```powershell
# Launch the full terminal interface
minefit

# Print a quick ranking table
minefit --cli -n 12

# Emit structured JSON
minefit --json -n 25
```

Why this is better:
- Feels active and approachable.
- Very good for first-time readers.

Tradeoff:
- Slightly less polished than a clean table or command-card layout.

## Variant E: Minimal inline section

This is the leanest option.

### Modes

- `minefit` for the interactive TUI
- `minefit --cli -n 12` for quick terminal tables
- `minefit --json -n 25` for scripts and automation

Why this is better:
- Extremely compact.
- Easy to scan.

Tradeoff:
- Looks less premium than the stronger options above.

## Recommendation

Best overall: **Variant B: Intent-first table**

Why:
- It improves clarity the most with the least visual overhead.
- It feels more user-centered than `Product Surface`.
- It fits the rest of the README better than a heavier card layout.

Best if you want a more polished/productized feel: **Variant C: Command cards**

Why:
- It gives the section more presence.
- It reads like a stronger open-source product README.
- It works well if the README is meant to feel more deliberate and less reference-like.

## Suggested replacement

If I were replacing the current section on `main`, I would use this:

### Choose Your Mode

| If you want to... | Run this |
| --- | --- |
| browse the best opportunities interactively | `minefit` |
| check the current ranking quickly in a shell | `minefit --cli -n 12` |
| feed results into scripts or other tools | `minefit --json -n 25` |

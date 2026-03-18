# minefit API Status

`minefit` currently ships and maintains:

- the interactive terminal UI
- classic CLI table output
- JSON output for scripting and automation

The legacy upstream REST API documentation that used to live in this file no longer reflects the mining-focused product in this repository.

For machine-readable integration today, use:

```powershell
minefit --json -n 25
```

If a maintained HTTP API is added back to `minefit`, this document should be replaced with a product-specific contract rather than the old upstream `llmfit` endpoints.

# Security Policy

## Supported Version

Security fixes are applied to the current `main` branch.

## Reporting

If you find a security issue:

1. Do not open a public issue with exploit details.
2. Use GitHub private vulnerability reporting if it is available for this repository.
3. If private reporting is unavailable, open a minimal public issue asking for a private follow-up without disclosing sensitive details.

## Scope

Relevant issues include:

- command execution vulnerabilities
- unsafe shell invocation
- path traversal or arbitrary file write behavior
- credential leakage
- dependency vulnerabilities with practical impact
- network/API handling bugs that expose local machine information unexpectedly

# Test template library

The tracker includes reusable test-case templates and curated bundles. Templates are original,
short starting points authored for TestTracker; they are not copied client material.

## Public sources

Coverage is aligned with public frameworks and benchmarks:

- OWASP Web Security Testing Guide (WSTG)
- OWASP Application Security Verification Standard 5.0
- OWASP API Security Top 10 2023
- OWASP Mobile Application Security Verification Standard (MASVS/MASTG)
- OWASP Top 10 2025
- OWASP Top 10 for LLM Applications 2026
- CIS Benchmarks for Windows, Linux, macOS, Kubernetes, Docker, and cloud platforms
- NIST SP 800-115 and the Penetration Testing Execution Standard (PTES)
- Microsoft Active Directory security guidance and public SAP security baselines

References in each template point to a framework or weakness class, not to client-specific documents.

## Bundles

Built-in bundles cover:

- web core and web deep dive
- API core and GraphQL/gRPC/WebSocket
- mobile baseline
- cloud baseline
- Kubernetes and containers
- Windows and Active Directory
- Linux and macOS
- network exposure
- SAP baseline
- AI/LLM security
- BI and Power BI
- thick client
- secure code review
- full application assessment

Add a bundle from **Test tracker → From templates**. Existing tests are deduplicated by template ID.

## Import and export

Bundles export as `.tttemplates.json` and include:

```json
{
  "format": "testtracker-template-bundle",
  "formatVersion": 1,
  "contentHash": "BLAKE3...",
  "bundle": {
    "id": "team-web",
    "name": "Team web baseline",
    "description": "Shared team starting point",
    "version": "1.2.0"
  },
  "templates": [
    {
      "id": "team-web-auth",
      "domain": "web",
      "category": "Authentication",
      "title": "Authentication and session management",
      "approach": "Review login, logout, reset, session rotation, cookies, and lockout.",
      "expectedResult": "Sessions rotate and authentication controls work.",
      "references": "OWASP WSTG-ATHN, CWE-287"
    }
  ]
}
```

Imports validate the format version, size, content hash, template count, unique IDs, domains, and
text limits. Imported template IDs are remapped locally so they cannot overwrite built-ins. Importing
the same source bundle again updates that imported bundle.

Plain JSON is intentional so teams can review and version bundles in Git. The content hash checks
integrity, not authorship; only import bundles from trusted teams.

## Content hygiene

- No client names, internal hostnames, findings, evidence, or reference workbook text are included.
- Templates use synthetic examples only.
- Review imported bundles before adding them to an engagement.

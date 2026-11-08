# AI data policy

AI features are optional. The default per-engagement policy is local-only and the application
remains fully usable with AI disabled. This document describes the v1 implementation and the
controls still planned.

## Providers

Supported connection types:

- Local: Ollama or any OpenAI-compatible endpoint on localhost/127.0.0.1/::1.
- Model discovery: Ollama uses `/api/tags`; OpenAI-compatible and Anthropic providers use their
  model-list endpoints. Azure OpenAI requires the deployment name manually.
- Stored API keys are never reused against a changed provider URL or type. Editing the endpoint
  requires re-entering the key, and model discovery refuses to send a stored key to a new endpoint.
- OpenAI-compatible APIs.
- Anthropic Messages API.
- Azure OpenAI (full chat-completions endpoint).

AWS Bedrock is planned. Consumer subscriptions are not supported; use official APIs, cloud accounts,
or local models.

Most consumer AI subscriptions do not expose API access. Connect official APIs, cloud accounts, or
local models only. Do not integrate with undocumented or subscription-scraping endpoints.

## Per-engagement policy

Each engagement has an explicit AI policy:

| Policy           | Behavior                                                    |
| ---------------- | ----------------------------------------------------------- |
| `off`            | No prompts may include engagement data                      |
| `local_only`     | Only local model endpoints may be used                      |
| `approved_cloud` | Only providers on the organization allowlist, after consent |

The policy is enforced in Rust, not in the UI. A cloud call without an approved policy and a
server-side consent token is rejected.

## Consent flow

Before a prompt leaves the device, the UI shows the provider, model, policy, purpose, redaction
count, and the exact redacted payload.

The server stores the redacted prompt in memory with a one-time consent token that expires after five
minutes. Sending requires that token; the frontend cannot substitute a different payload. The token
is consumed on first use, cleared on vault lock, and the current policy and provider are re-checked at
send time.

The audit log records the provider, model, purpose, prompt hash, response hash, redaction count, and
token usage. Message content is not persisted by default.

## Redaction

The redaction pipeline runs in Rust before any provider request and replaces:

- Private key blocks, JWTs, bearer tokens, AWS access keys, and secret assignments.
- URLs with embedded credentials.
- Emails, IPv4 addresses, and common internal hostnames.

Users must review the preview; redaction is a control, not a guarantee. Entropy checks and
organization-specific rules are planned.

## Logging

- Audit event per preview, successful call, and failed call: provider, model, purpose, redaction
  count, prompt hash, response hash, and token usage.
- Full prompt/response retention is not implemented in v1; the default keeps hashes and metadata only.
- Provider keys are encrypted with the master key and never exposed to the webview.
- Usage and cost budgets are planned.

## Safety

- AI output is untrusted. Findings and report text are drafts until the user accepts them.
- The system prompt labels pasted content as untrusted data, but prompt injection remains a risk.
- Tool calls, MCP, and automatic data access are not implemented in v1.
- No automatic execution of generated scripts.
- Responses are displayed as plain text and are never auto-applied to findings or reports.

## MCP

MCP client/server support, tool calling, and streaming are planned. MCP servers will be untrusted:
capabilities will be shown, approved, and audited; writes will require confirmation.

## Non-goals

- Training models on engagement data.
- Sending evidence files without explicit selection.
- Silent background prompts.
- Telemetry or analytics containing engagement data.

# AI agent skills

TestTracker's AI is an in-app agent with server-built context and explicit write approval. It is not
a raw chat that receives arbitrary frontend payloads.

## Flow

1. The user opens an agent action (engagement summary, assumptions, test cases, test approach, or
   finding).
2. Rust loads the scoped engagement context, checks the provider policy, and builds the prompt.
3. The prompt is redacted and shown as a preview. The preview is the consent token.
4. The agent makes one provider call and must return a JSON object matching the skill schema.
5. The server validates the JSON and stores it as an in-memory proposal.
6. The user reviews the proposal and selects **Apply**. Nothing is written automatically.

Previews expire after 5 minutes and proposals after 15 minutes. Both are cleared on vault lock.

## Skills

| Skill                 | Output                                                     | Apply behavior                              |
| --------------------- | ---------------------------------------------------------- | ------------------------------------------- |
| `application_summary` | `{"value":"..."}`                                          | Updates the engagement summary              |
| `attack_surface`      | `{"value":"..."}`                                          | Updates the attack-surface field            |
| `assumptions`         | `{"values":["..."]}`                                       | Replaces the engagement assumptions         |
| `test_cases`          | `{"testCases":[{title,category,approach,expectedResult}]}` | Creates new tests at the end of the tracker |
| `test_approach`       | `{"approach":"..."}`                                       | Replaces the target test's approach         |
| `finding_draft`       | Structured finding fields                                  | Creates a finding linked to the target test |

Finding drafts include probability and impact (0-9); severity is recomputed server-side with the
approved matrix. Test cases are created as `not_started` and assigned to the current profile.

## Context

The context includes engagement metadata, scope, risks, assumptions, contacts, existing test
summaries, and existing finding summaries. Target-test skills add the full target test, linked
findings, and linked evidence metadata. Evidence **content** is not included.

The context is capped at 120 KB. The prompt is redacted with the same pipeline used by the AI chat
(email, IP, bearer tokens, secrets, private keys, JWTs, internal hosts). The preview shows exactly
what will be sent.

## Security

- Provider policy is enforced in Rust on preview and re-checked on run.
- The agent has no SQL, shell, filesystem, network, or plugin access. It returns JSON only.
- Proposals are validated and applied through domain repositories in a transaction.
- Every preview, proposal, applied action, and failure writes an audit event with hashes and
  metadata, not content.
- No tool calls are automatically executed; every write requires **Apply**.
- The current implementation performs one provider call per skill run. Multi-step tool loops and
  external MCP client support are planned. Separately, the local MCP bridge exposes read tools and
  draft-only proposals to external MCP clients; see `docs/mcp.md`.

## UI entry points

- Engagement overview: **Draft** on application summary, attack surface, and assumptions.
- Test tracker: **Generate tests** in the toolbar; **AI draft** on each row for test approach or a
  finding.
- Provider selection and policy are shown inside the agent dialog. Local-only remains the default.

## Assessment import

**Import assessment** is a separate flow from engagement-scoped agent skills. It extracts text from
a previous assessment (XLSX/XLSM, DOCX, PDF, Markdown, JSON, or text), builds a dedicated import
prompt, and asks the provider for one JSON draft. There is no engagement yet, so the per-engagement
AI policy does not apply; local providers are allowed, and non-local providers require an explicit
cloud-consent checkbox before the preview is created. The redacted preview is still shown and
consumed by the provider call. The returned draft is validated, held in memory, and only written
after **Create engagement**. See `docs/imports.md`.

## Engagement assistant

The engagement **Assistant** is the first tab in an engagement. It combines the agent context with a
bounded conversation and chooses one action from a server-approved catalog:

- application summary
- attack surface
- assumptions
- test cases
- test approach
- finding draft
- finding improvement
- no action / read-only answer

The model returns:

```json
{
  "message": "Short answer",
  "action": {
    "type": "finding_improve",
    "targetId": "finding-uuid",
    "payload": {}
  }
}
```

The server validates the action type, target ownership, payload shape, and engagement context. The
proposal remains in memory until the user selects **Apply**. The UI shows the detected action and
proposal payload.

### Focus and deep links

- Test tracker **Ask assistant** focuses a test case.
- Findings **Ask assistant** focuses a finding and can propose a finding improvement.
- Evidence **Analyze with AI** focuses one evidence item.
- Focus is passed in the URL and can be cleared in the assistant header.

### Evidence and images

Text is extracted from text-like evidence, PDFs, and DOCX files. PNG, JPEG, and WebP images can be
attached for a vision-capable provider. Images are decoded, resized to a maximum dimension of 2048
pixels, re-encoded to strip metadata, and capped at 4 MB. Redaction applies to text, not pixels.
Image content is sent only after the user selects **Analyze with AI** and approves the preview.

The audit log records the focus, provider, model, prompt/response hashes, token usage, redaction
count, action metadata, and image hashes. It does not store message content or image bytes.

## Reference files and tools

The Assistant can review files uploaded to the **Reference files** panel:

- Postman collections
- Burp XML exports
- HAR files
- Procmon CSV/XML exports
- OpenAPI/Swagger JSON or YAML
- GraphQL SDL
- Nmap XML
- HTTP/text/JSON/CSV/Markdown

Reference files are encrypted at rest and scoped to one engagement. Deterministic Rust parsers
produce compact summaries; the model reads details only through tools.

Tools are read-only. Every tool call pauses for an explicit decision:

- **Allow once**
- **Always allow this tool for this engagement**
- **Deny and stop**

Always-allowed tools are listed under **Tool permissions** and can be removed there. Tool calls,
decisions, arguments hashes, result hashes, and durations are audited. Tool results are redacted and
size-capped before being sent to a provider.

Agent runs are bounded to 6 provider rounds and 12 tool calls. Write actions still require the final
proposal Apply step.

## Ollama notes

Assistant and structured agent calls use Ollama JSON mode and a provider context-window budget. If a
local model still returns a malformed envelope, the assistant returns its message without applying
an action; use a larger/stronger model for reliable structured proposals.

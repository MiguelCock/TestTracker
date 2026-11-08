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
  MCP are planned.

## UI entry points

- Engagement overview: **Draft** on application summary, attack surface, and assumptions.
- Test tracker: **Generate tests** in the toolbar; **AI draft** on each row for test approach or a
  finding.
- Provider selection and policy are shown inside the agent dialog. Local-only remains the default.

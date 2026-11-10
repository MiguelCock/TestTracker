# MCP bridge

TestTracker includes an optional local **Model Context Protocol server**. It lets an MCP-capable
client read engagement data and submit draft changes to the desktop application. It is not an MCP
client: TestTracker does not connect to third-party MCP servers.

The bridge is off by default and must be started manually. A bridge token grants access to **all
engagements** in the unlocked vault.

## Setup

1. Unlock the vault.
2. Open **MCP** in the sidebar.
3. Choose a port and select **Start bridge**.
4. Reveal the bearer token and configure the MCP client with:

```json
{
  "mcpServers": {
    "testtracker": {
      "type": "http",
      "url": "http://127.0.0.1:37621/mcp",
      "headers": {
        "Authorization": "Bearer <token>"
      }
    }
  }
}
```

Client configuration formats vary. The required values are the Streamable HTTP endpoint and the
`Authorization: Bearer ...` header on every request.

The default endpoint is `http://127.0.0.1:37621/mcp`. For OpenCode, see `docs/opencode.md`. The server binds only to IPv4 loopback. The
port is stored in the encrypted database; the token is encrypted with the vault master key and is
only returned to the UI on explicit reveal.

## Quick examples

After starting the bridge and configuring a client, try prompts such as:

- "List my engagements and summarize the open critical findings."
- "For engagement `<id>`, propose five authorization test cases."
- "Read test case `<id>` and draft a finding only if the recorded result supports it."
- "Show the evidence linked to finding `<id>`."
- "Check the status of MCP approval `<id>`."

The client will call read tools immediately. `propose_*` calls return a pending approval ID; approve
or reject the draft under **MCP → Pending proposals**.

### Curl smoke test

```bash
TOKEN="<token>"
URL="http://127.0.0.1:37621/mcp"

curl -s "$URL" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"curl","version":"1.0"}}}'

curl -s "$URL" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'

curl -s "$URL" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_engagements","arguments":{}}}'
```

### Common errors

| Response                        | Meaning                                                                                            |
| ------------------------------- | -------------------------------------------------------------------------------------------------- |
| `401 Unauthorized`              | Missing or incorrect bearer token. Reveal the token again in TestTracker.                          |
| `403 Forbidden`                 | `Host` or `Origin` is not the loopback endpoint. Use `127.0.0.1`/`localhost` and no browser proxy. |
| `Connection refused`            | Bridge is stopped, the app is locked, or the port is different.                                    |
| `Approval not found or expired` | Proposals expire after 15 minutes and are cleared when the bridge stops.                           |

## Lifecycle

- Disabled until the user selects **Start**.
- Stops immediately when the vault locks, the session expires, the app exits, or the user selects
  **Stop**.
- MCP reads and proposals do not extend the auto-lock timer.
- Pending proposals are cleared when the server stops.
- Regenerating the token requires the bridge to be stopped.

## Tools

Read-only tools:

| Tool                     | Purpose                                               |
| ------------------------ | ----------------------------------------------------- |
| `list_engagements`       | Engagement summaries and counts                       |
| `get_engagement`         | Metadata, scope, risks, assumptions; contacts omitted |
| `list_test_cases`        | Paginated test summaries with optional filters        |
| `get_test_case`          | One test plus linked finding/evidence metadata        |
| `list_findings`          | Paginated findings with optional filters              |
| `get_finding`            | One finding plus linked test/evidence metadata        |
| `get_dashboard_metrics`  | Aggregate dashboard metrics                           |
| `get_engagement_context` | Sanitized context for drafting                        |

`list_engagements` returns `{ "engagements": [...], "count": N }`. The other list tools return
`{ "testCases": [...], ... }` and `{ "findings": [...], ... }` so that `structuredContent` is always
an object.

Draft-only tools:

| Tool                          | Proposal                          |
| ----------------------------- | --------------------------------- |
| `propose_application_summary` | Replace the application summary   |
| `propose_attack_surface`      | Replace the attack surface        |
| `propose_assumptions`         | Replace assumptions               |
| `propose_test_cases`          | Add test cases                    |
| `propose_test_approach`       | Replace one test approach         |
| `propose_finding`             | Create a finding linked to a test |
| `get_approval_status`         | Poll a pending/completed approval |

The bridge does not expose evidence file content, contacts, generated documents, audit history,
snapshots, approvals, sign-offs, AI providers, or deletion tools. Findings and test results are
readable for drafting, so treat the token as access to sensitive engagement data.

## Approval flow

1. A `propose_*` call validates identifiers, engagement ownership, text limits, and payload shape.
2. The proposal is stored in memory with a 15-minute expiry.
3. The tool returns a pending approval ID. **No database mutation occurs.**
4. The proposal appears under **MCP → Pending proposals**.
5. The user selects **Approve** or **Reject**.
6. Approval applies the proposal through the same domain logic as in-app agent proposals, in one
   transaction, and writes `mcp.approval.approved`.
7. The MCP client can poll `get_approval_status` for the result.

Limits: 20 pending proposals total, 5 per engagement. Completed status records are retained briefly
for polling and then removed. Proposals and statuses live only in memory.

## Security controls

- **Loopback only:** listener binds to `127.0.0.1`; no LAN or public interface.
- **Bearer authentication:** a 32-byte token is required on every request. The server compares
  token hashes in constant time.
- **Host validation:** the MCP transport accepts only the configured `127.0.0.1:<port>` and
  `localhost:<port>` authorities.
- **Origin validation:** browser origins are restricted to the loopback endpoint. Requests from
  other origins are rejected. No CORS headers are returned.
- **Token at rest:** encrypted with the vault master key and a profile-bound AAD. Audit events never
  contain the token.
- **Request limits:** 1 MB maximum body and bounded concurrent local access.
- **Human approval:** MCP clients cannot write directly, even with a valid token.
- **Audit:** server start/stop, token creation/reveal/regeneration, authentication failures (rate
  limited), tool calls, pending proposals, approvals, and rejections are recorded with metadata and
  hashes, not payload content.
- **No session extension:** bridge activity does not keep the vault unlocked.
- **Lock clears state:** the server stops and pending approvals are discarded on lock.

## Threat model and residual risk

The token is a bearer credential. Any local process or user that obtains it can read every
engagement while the bridge is running. A connected MCP client may be cloud-backed; TestTracker
controls local access, not what the client does with retrieved data. Use a trusted client, reveal the
token only when needed, stop the bridge when finished, and regenerate the token if it may have
leaked.

Prompt-injection content in engagement text can influence a connected model. Drafts are untrusted
and require review. The bridge does not auto-apply model output, execute tools inside TestTracker, or
send evidence files.

The server is a local attack surface: dependency vulnerabilities, parser bugs, HTTP handling, or the
loopback boundary can affect it. It is disabled by default and only runs while the vault is
unlocked.

## Non-goals

- Connecting TestTracker's assistant to external MCP servers.
- Direct MCP writes, deletes, evidence uploads, document generation, approvals, or sign-offs.
- Remote/LAN access, OAuth, multi-user authorization, or cloud-hosted deployment.
- Per-engagement allowlists in this version.
- Streaming tool execution or model sampling.

## Audit actions

`mcp.server.started`, `mcp.server.stopped`, `mcp.settings.updated`, `mcp.token.created`,
`mcp.token.revealed`, `mcp.token.regenerated`, `mcp.auth.failed`, `mcp.tool.called`,
`mcp.approval.pending`, `mcp.approval.approved`, `mcp.approval.rejected`.

## Tests

- Token validation and authentication failure handling.
- Proposal validation, pending limits, expiry, approve/reject transitions.
- Proposal cannot write before approval; approval creates records and preserves the audit chain.
- Vault lock stops the listener and clears pending proposals.
- Manual MCP smoke test with an MCP client or `curl` against `initialize`, `tools/list`, and
  `tools/call`.

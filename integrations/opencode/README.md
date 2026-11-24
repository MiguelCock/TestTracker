# TestTracker OpenCode pack

This pack connects OpenCode to a running TestTracker MCP bridge and provides
Plan and Build agents that use TestTracker's read and proposal tools.

## 1. Start TestTracker

1. Unlock the vault.
2. Open **MCP**.
3. Start the bridge.
4. Reveal the bearer token.

## 2. Configure the MCP server

Copy the `testtracker` block from `opencode.jsonc.example` into your OpenCode
configuration:

- Global: `~/.config/opencode/opencode.jsonc`
- Project: `./opencode.jsonc` or `./.opencode/opencode.jsonc`

Supply the token through the environment, not in the config:

```bash
export TESTTRACKER_MCP_TOKEN="<token>"
```

The default endpoint is `http://127.0.0.1:37621/mcp`.

## 3. Install the agents

Copy or symlink the `agents/` directory into one of:

```
~/.config/opencode/agents/
.opencode/agents/
```

Agents:

- `testtracker-plan` — read-only planning. Uses TestTracker read/file tools and
  produces a plan without applying changes.
- `testtracker-build` — uses TestTracker proposal tools. Every write becomes a
  pending change that must be approved in TestTracker.

Select the agent in OpenCode with `/agent testtracker-plan` or
`/agent testtracker-build`.

## 4. Suggested workflow

```text
/agent testtracker-plan
Review engagement <id>. Inspect the files, tests, and findings, then produce a
plan for the next testing block.

/agent testtracker-build
Implement the approved plan. Propose changes; do not apply them directly.
```

Read tools execute immediately. Proposal tools create pending changes under
**MCP → Pending proposals** in TestTracker.

## Notes

- The bridge only binds to loopback.
- It stops when the vault locks.
- The token grants read access to every engagement while the bridge runs.
- OpenCode never writes directly; TestTracker validates and applies proposals.

# OpenCode MCP connection

TestTracker can expose its local MCP bridge to OpenCode. The bridge must be running and the vault
unlocked.

## 1. Start the bridge

1. Unlock TestTracker.
2. Open **MCP**.
3. Select **Start bridge**.
4. Reveal the bearer token.

## 2. Configure OpenCode

The global configuration is `~/.config/opencode/opencode.json` (`%USERPROFILE%\.config\opencode\opencode.json`
on Windows). OpenCode V2 keeps server names under `mcp.servers`, not directly under `mcp`. Merge
`testtracker` into any existing entries:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "servers": {
      "testtracker": {
        "type": "remote",
        "url": "http://127.0.0.1:37621/mcp",
        "oauth": false,
        "headers": {
          "Authorization": "Bearer {env:TESTTRACKER_MCP_TOKEN}"
        }
      }
    }
  }
}
```

Do not paste the token directly into the config. Supply it through the environment of the shell
that starts OpenCode:

```bash
export TESTTRACKER_MCP_TOKEN="<token>"
opencode
```

Config edits apply without restarting, but the token is read from the environment when OpenCode
starts, so relaunch it from a shell with the current value.

## 3. Verify

```bash
opencode mcp list
```

A connected server appears as:

```text
✓ testtracker connected
http://127.0.0.1:37621/mcp
```

Tools are named `testtracker_<tool>`, for example `testtracker_list_engagements`.

## Example prompts

```text
Use testtracker_list_engagements.
Use testtracker_get_engagement for engagement <id>.
Use testtracker_list_findings for engagement <id> and summarize open critical findings.
Use testtracker_propose_test_cases to propose authorization tests for engagement <id>.
```

Proposal tools create pending approvals in TestTracker. Approve or reject them under
**MCP → Pending proposals**. Read tools run immediately.

## Troubleshooting

- `Connection refused`: start the bridge in TestTracker and keep the app unlocked.
- `401`: the environment token is missing or stale; reveal it again and restart OpenCode.
- `403`: use the loopback URL; do not put a proxy in front of the endpoint.
- The server stops when TestTracker locks or exits.

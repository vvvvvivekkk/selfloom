# Wiring selfloom into Claude Code

Claude Code reads MCP servers from a few places. Pick whichever fits.

## 1. One command (recommended)

Add selfloom as a user-scope MCP server so every project you open has it:

```bash
claude mcp add --scope user selfloom \
  /ABSOLUTE/PATH/TO/loom --vault /ABSOLUTE/PATH/TO/VAULT serve
```

Verify:

```bash
claude mcp list
```

You should see `selfloom` with its command. Open any project and type:

```
/mcp
```

to see selfloom's tools listed.

## 2. Per-project `.mcp.json`

Drop this at the root of a repo. It only applies when you run Claude Code inside that repo.

```json
{
  "mcpServers": {
    "selfloom": {
      "command": "/ABSOLUTE/PATH/TO/loom",
      "args": ["--vault", "/ABSOLUTE/PATH/TO/VAULT", "serve"]
    }
  }
}
```

## 3. The user-config file

If you prefer editing JSON directly, Claude Code's user-scope MCP servers live in `~/.config/claude-code/mcp.json` (or similar depending on your OS and Claude Code version). Same shape as above.

## Nudging Claude to actually use it

MCP tools are opt-in — the model decides when to call them. Give it a nudge by dropping something like this into your project's `CLAUDE.md`:

```markdown
# Memory

You have a `selfloom` MCP server attached. Before answering questions about
me, my preferences, past decisions, or ongoing projects, call its `search`
tool first. After every conversation, call `write_memory` to save anything
durable that came up — facts, rules I set, decisions I made, skills I learned.
Pick the right tier.
```

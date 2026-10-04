# selfloom

> Local-first memory for Claude Desktop, Claude Code, and any other MCP client.
> Markdown + git. Single tiny binary. Zero cloud.

**selfloom** is one small Rust binary that gives your AI tools a shared, persistent memory of you — stored as plain markdown files, versioned by git, searched by SQLite FTS5, and exposed over the Model Context Protocol. Point Claude Desktop and Claude Code at the same vault and whatever one learns, the other knows.

Think *GitLoom*, but local and open source. Think *Obsidian*, but written for AI agents instead of humans.

- **Local-first.** Vault is a folder on your disk. Nothing leaves your machine.
- **Human-readable.** Every memory is a markdown file you can open and edit.
- **Versioned.** Every write is a git commit — full history, forever.
- **Shared.** Same vault for Claude Desktop, Claude Code, Cursor, Zed, your own scripts.
- **Tiny.** ~1.8 MB release binary. No runtime, no daemon, no deps.

## Install

Requires Rust (`rustup`) and git. SQLite is bundled.

```bash
git clone https://github.com/vvvvvivekkk/selfloom
cd selfloom
cargo build --release
# binary lands at ./target/release/loom
# move it onto your PATH, e.g.:
sudo cp target/release/loom /usr/local/bin/
```

Homebrew tap and `cargo install selfloom` land in v1.0.

## Make a vault

```bash
loom --vault ~/brain init
export SELFLOOM_VAULT=~/brain      # so you can drop --vault from now on
```

That gives you:

```
~/brain/
├── .git/                      # every write becomes a commit
├── .selfloom/
│   ├── config.toml
│   └── index.sqlite           # FTS5 index
└── namespaces/
    └── default/
        ├── facts/
        ├── rules/
        ├── incidents/
        └── skills/
```

## Try it from the CLI

```bash
loom write --title "Prefers filter coffee" --tier rules \
  --tags "food,preference" \
  "Daily morning filter coffee, never instant."

loom search coffee
# → 0.500  namespaces/default/rules/2026-10-04-prefers-filter-coffee-xxx.md
#       Daily morning filter «coffee», never instant.
```

Open the file in any editor — it's just markdown with YAML frontmatter.

## Wire it into Claude Desktop

Edit (or create) your Claude Desktop config file:

- **macOS** `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows** `%APPDATA%\Claude\claude_desktop_config.json`
- **Linux** `~/.config/Claude/claude_desktop_config.json`

Add the `selfloom` entry under `mcpServers`:

```json
{
  "mcpServers": {
    "selfloom": {
      "command": "/usr/local/bin/loom",
      "args": ["--vault", "/Users/you/brain", "serve"]
    }
  }
}
```

Restart Claude Desktop. You'll see selfloom's tools in the tools picker. Full snippet in [`examples/claude-desktop.json`](examples/claude-desktop.json).

## Wire it into Claude Code

One command, user scope (every project gets it):

```bash
claude mcp add --scope user selfloom \
  /usr/local/bin/loom --vault /Users/you/brain serve

claude mcp list     # confirm
```

Open any project and type `/mcp` to see the tools. More options (per-project `.mcp.json`, direct user-config edit) in [`examples/claude-code.md`](examples/claude-code.md).

## Nudge the model to actually use it

MCP tools are opt-in — the model decides when to call them. Drop [`examples/CLAUDE.md.example`](examples/CLAUDE.md.example) into your project (or `~/.claude/CLAUDE.md` for global) so Claude knows to search before answering and write after conversations.

## MCP tools exposed

| tool              | what it does                                                 |
| ----------------- | ------------------------------------------------------------ |
| `search`          | BM25 full-text search over the vault, filter by tier / ns    |
| `read_memory`     | fetch one memory by path                                     |
| `write_memory`    | save a memory with title, body, tier, tags                   |
| `list_namespaces` | list vaults inside the vault                                 |

## About "autostart"

Claude Desktop and Claude Code spawn `loom serve` themselves over stdio when they launch. **You don't need to run it as a service.** Both apps share the vault because they both write to the same folder.

The `installers/` folder has launchd and systemd files for the day you want selfloom running as a daemon for other reasons (HTTP mode, Cursor over SSE, etc.). You can skip them for now.

## Architecture

```
Ingest:   conversation → write_memory → markdown → git commit → FTS5 index
Retrieve: query → BM25 (FTS5) → ranked results with snippets
```

v0.1 is BM25-only. v0.2 adds LLM-powered conversation extraction (so you don't have to call `write_memory` by hand). v0.3 adds local embeddings + hybrid retrieval. See [`docs/architecture.md`](docs/architecture.md) for the full plan.

## Status

**v0.1 is here.** Everything above actually runs. Expect bugs — open an issue.

- [x] markdown vault + per-write git commits
- [x] SQLite FTS5 index with snippets
- [x] stdio MCP server (initialize, tools/list, tools/call)
- [x] CLI: init, serve, write, read, search, reindex
- [ ] v0.2 — `ingest_conversation` tool with LLM extraction into tiers
- [ ] v0.3 — local embeddings + HNSW vector search + score fusion
- [ ] v0.4 — wiki-link graph + backlinks
- [ ] v0.5 — namespaces polish, HTTP mode, Docker image
- [ ] v1.0 — Homebrew tap, cargo install, demo video, launch

## Prior art

- [GitLoom](https://gitloom.cloud) — the cloud product that inspired this one. Credit for the tier model (`facts/rules/incidents/skills`) and git-backed-markdown idea. selfloom's wedge is local-first and open source.
- [Basic Memory](https://github.com/basicmachines-co/basic-memory) — Python MCP memory server. Different stack, similar spirit.
- [mem0](https://mem0.ai), [Letta](https://letta.com), [Zep](https://getzep.com) — hosted memory layers for agents.

## License

MIT — see [LICENSE](LICENSE).

---

Built by [@vvvvvivekkk](https://github.com/vvvvvivekkk).

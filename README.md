# selfloom

> Local-first memory for AI agents. Git-backed markdown. MCP out of the box. Zero cloud.

**selfloom** is a single-binary memory layer for coding agents and chat assistants. Point Claude Code, Cursor, or any MCP client at it, and your conversations turn into a vault of markdown notes you can grep, edit by hand, and `git log` through — forever, on your own machine.

Think *GitLoom*, but local. Think *Obsidian*, but written for AI agents instead of humans.

---

## Why

Agents forget. The usual answers are:

- **Cloud memory APIs** — fast to adopt, but your conversations and private thoughts end up on someone else's server.
- **Vector DBs** — opaque. You can't open a row and read it. You can't `git blame` a fact.
- **Big desktop apps** — Obsidian is 400+ MB of Electron. Logseq is heavier. Neither speaks MCP natively.

selfloom picks the stubborn third option: **plain markdown files, versioned by git, searched by SQLite FTS + local vectors, exposed through MCP.** The whole thing is one static binary. Your data lives in a folder you own.

## What's in the box

- **Markdown vault** — one file per memory, grouped into `facts/`, `rules/`, `incidents/`, `skills/` tiers (idea borrowed from GitLoom).
- **Git under the hood** — every write is a commit with provenance (who, when, from which conversation).
- **Hybrid retrieval** — BM25 (full-text), vector (local embeddings), and wiki-link graph, fused into one score.
- **MCP server** — stdio transport, drop into any MCP-aware client.
- **Bring your own LLM** — extraction can call Ollama locally, or Claude / OpenAI with your own key. Never ours.
- **Tiny** — target: < 50 MB RAM idle, < 100 ms search on 10k memories, single ~15 MB binary.

## Quickstart (planned)

```bash
# install
brew install selfloom           # or: cargo install selfloom

# make a vault
loom init ./brain

# start the MCP server
loom serve --vault ./brain

# or hook into Claude Code
# add to ~/.config/claude-code/mcp.json:
# {
#   "selfloom": { "command": "loom", "args": ["serve", "--vault", "/path/to/brain"] }
# }
```

Then just use your AI like normal. `ingest_conversation` fires automatically on each exchange; `search` fires when the agent needs context.

## MCP tools

| tool                   | what it does                                            |
| ---------------------- | ------------------------------------------------------- |
| `ingest_conversation`  | takes messages, extracts memories into the right tier  |
| `search`               | hybrid BM25 + vector + graph query                      |
| `read_memory`          | fetches one memory by path                              |
| `write_memory`         | writes a memory directly (bypasses extraction)          |
| `list_namespaces`      | lists vaults                                            |
| `create_namespace`     | creates a new vault                                     |
| `backlinks`            | returns memories that `[[wiki-link]]` to this one       |

## How it works

```
Ingest:   conversation → LLM extract → markdown write → git commit → index update
Retrieve: query → BM25 ∥ vector ∥ graph → score fusion → top-K with provenance
```

Full diagrams and design notes are in [`docs/architecture.md`](docs/architecture.md).

## Status

This repo is **in design**. Not usable yet. Watch the repo (or star it) if you want to be nudged when there's something to run.

## Roadmap

- [ ] **v0.1** — markdown vault + git + SQLite FTS5 + MCP `search`/`read`/`write`
- [ ] **v0.2** — `ingest_conversation` with LLM extraction into tiers
- [ ] **v0.3** — local embeddings (fastembed) + HNSW vector search + score fusion
- [ ] **v0.4** — wiki-link graph + backlinks tool
- [ ] **v0.5** — namespaces, auth for HTTP mode, Docker image
- [ ] **v1.0** — Homebrew tap, cargo install, demo video, launch

## Prior art

- [GitLoom](https://gitloom.cloud) — the cloud product that inspired this one. Credit where due; selfloom borrows the tier model and the git-backed markdown idea. The wedge here is local-first and open source.
- [Basic Memory](https://github.com/basicmachines-co/basic-memory) — Python-based MCP memory server. Similar spirit, different stack.
- [mem0](https://mem0.ai), [Letta](https://letta.com), [Zep](https://getzep.com) — hosted memory layers for agents.

## License

MIT. See [LICENSE](LICENSE).

---

Built by [@vvvvvivekkk](https://github.com/vvvvvivekkk).

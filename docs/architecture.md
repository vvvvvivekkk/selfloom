# selfloom architecture

Design notes for the single-binary, local-first memory layer.

## Goals

1. **Local-first.** The vault is a folder on disk. If the binary dies, nothing is lost.
2. **Human-readable.** Every memory is a plain markdown file. You can `cat` it, edit it in vim, grep it with ripgrep.
3. **Versioned.** Git under the hood. Every write is a commit; every memory has provenance.
4. **MCP-native.** The primary interface is a Model Context Protocol server, so any MCP client (Claude Code, Cursor, Zed, custom agents) can speak to it.
5. **Low footprint.** Target < 50 MB RAM idle, < 100 ms search on 10k memories, single binary < 20 MB.

## Non-goals

- Not a note-taking app for humans. (Use Obsidian or Logseq for that; selfloom is for agents.)
- Not a replacement for a vector DB at scale. (Target: millions of memories per user, not billions across tenants.)
- Not multi-tenant SaaS. (Each user runs their own binary. Hosting it for others is a separate project.)

## Vault layout

```
./brain/
  .selfloom/
    config.toml           # vault config
    index/
      fts.sqlite          # SQLite with FTS5 full-text index + metadata
      vectors.hnsw        # HNSW vector index on disk
  namespaces/
    default/
      .git/               # per-namespace git repo
      facts/
        2026-10-04-sony-a7iii-purchase-a3f2.md
      rules/
      incidents/
      skills/
    project-x/
      ...
```

## Memory file format

```markdown
---
id: a3f2b1c0
tier: facts
topic: camera-gear
tags: [camera, purchase, 2026]
created: 2026-10-04T17:30:00+08:00
confidence: 0.95
sources:
  - session: chat-2026-10-04-a
    turn: 3
---

User bought Sony A7III on 2026-10-04 for 142k INR at Fotocentre in
Bengaluru. Chose it over A7IV purely on price. Wants it ready for the
[[japan-trip-oct-2026]].
```

Frontmatter is YAML. Body is plain prose. Wiki-links (`[[slug]]`) build the graph.

## Flows

### Ingest

```
MCP client
    │  ingest_conversation(messages, namespace?)
    ▼
┌───────────────────┐
│  Extractor        │  calls user-configured LLM (Ollama / Claude / OpenAI)
│  (LLM-driven)     │  returns [{tier, title, body, tags, confidence}, ...]
└─────────┬─────────┘
          │
          ▼
┌───────────────────┐
│  Writer           │  slugify → write to namespaces/<ns>/<tier>/<slug>.md
└─────────┬─────────┘
          │
          ▼
┌───────────────────┐
│  Committer        │  git add + commit with provenance in commit message
└─────────┬─────────┘
          │
          ▼
┌───────────────────┐
│  Indexer          │  FTS5 upsert, embedding + HNSW insert, link graph update
└───────────────────┘
```

### Retrieve

```
MCP client
    │  search(q, namespace, filters)
    ▼
┌───────────────────────────────────┐
│   Three arms in parallel:         │
│   • BM25 via SQLite FTS5          │
│   • Vector via HNSW (local embed) │
│   • Graph expansion (link hops)   │
└──────────────┬────────────────────┘
               ▼
      Reciprocal rank fusion
               │
               ▼
      Filter by tier / tags / time
               │
               ▼
      Return top-K memories with
      provenance (commit hash, time)
```

## Tech stack (Rust)

| Concern         | Crate                      |
| --------------- | -------------------------- |
| MCP server      | `rmcp` (stdio transport)   |
| Full-text       | `rusqlite` + FTS5          |
| Embeddings      | `fastembed` (ONNX)         |
| Vector index    | `hnsw_rs`                  |
| Git ops         | `git2`                     |
| Markdown parse  | `pulldown-cmark`           |
| CLI             | `clap`                     |
| Config          | `serde` + `toml`           |
| Async runtime   | `tokio` (minimal features) |

Go is a reasonable fallback (bleve + go-git + fastembed-go + mcp-go) if Rust iteration speed becomes a problem.

## Model for choosing the right memory

Borrowed and adapted from GitLoom:

| Tier       | What belongs here                                          |
| ---------- | ---------------------------------------------------------- |
| facts      | Stable, true-right-now assertions about the user or world. |
| rules      | Preferences, policies, standing instructions.              |
| incidents  | Dated events, decisions, meetings.                         |
| skills     | How-to procedures, repeatable workflows.                   |

The extractor LLM is prompted to classify each memory it emits into one tier. Tiers map to directories, which keeps everything grep-able and lets `search` filter by tier.

## Open questions

- Do we need a daemon, or is `loom serve` per-vault enough?
- How to handle schema evolution of the memory frontmatter without breaking old vaults?
- Should extraction run inline (slow, simple) or background-queued (fast, more infra)?
- Does the HTTP mode get an auth layer in v1, or stay stdio-only?

## Launch checklist (v1.0)

- [ ] Homebrew formula
- [ ] `cargo install selfloom`
- [ ] 60-second demo video
- [ ] Submission to the official MCP server directory
- [ ] "Show HN" post
- [ ] r/LocalLLaMA + r/selfhosted announce

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;

use crate::memory::{parse_file, Memory};
use crate::vault::Vault;

pub struct Index {
    conn: Connection,
}

#[derive(Debug, Clone)]
pub struct Hit {
    pub path: String,
    pub namespace: String,
    pub tier: String,
    pub title: String,
    pub score: f64,
    pub snippet: String,
}

impl Index {
    pub fn open(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(db_path)
            .with_context(|| format!("open sqlite {}", db_path.display()))?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS memories (
              path       TEXT PRIMARY KEY,
              namespace  TEXT NOT NULL,
              tier       TEXT NOT NULL,
              title      TEXT NOT NULL,
              tags       TEXT NOT NULL,
              created    TEXT NOT NULL
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
              path UNINDEXED,
              title,
              body,
              tags,
              tokenize='porter unicode61'
            );
            "#,
        )?;
        Ok(Self { conn })
    }

    pub fn upsert(&mut self, mem: &Memory) -> Result<()> {
        self.conn.execute(
            "INSERT INTO memories(path, namespace, tier, title, tags, created)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(path) DO UPDATE SET
               namespace=excluded.namespace,
               tier=excluded.tier,
               title=excluded.title,
               tags=excluded.tags,
               created=excluded.created",
            params![
                &mem.relative_path,
                &mem.namespace,
                mem.tier.as_dir(),
                &mem.title,
                mem.tags.join(","),
                mem.created.to_rfc3339(),
            ],
        )?;
        // Replace in FTS too
        self.conn.execute(
            "DELETE FROM memories_fts WHERE path = ?1",
            params![&mem.relative_path],
        )?;
        self.conn.execute(
            "INSERT INTO memories_fts(path, title, body, tags) VALUES (?1, ?2, ?3, ?4)",
            params![&mem.relative_path, &mem.title, &mem.body, mem.tags.join(" ")],
        )?;
        Ok(())
    }

    pub fn search(
        &self,
        query: &str,
        limit: usize,
        namespace: Option<&str>,
        tier: Option<&str>,
    ) -> Result<Vec<Hit>> {
        let safe_query = fts_escape(query);
        let mut sql = String::from(
            r#"
            SELECT f.path,
                   m.namespace,
                   m.tier,
                   m.title,
                   bm25(memories_fts) AS rank,
                   snippet(memories_fts, 2, '«', '»', '…', 20) AS snip
              FROM memories_fts f
              JOIN memories    m ON m.path = f.path
             WHERE memories_fts MATCH ?1
            "#,
        );
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(safe_query)];
        if let Some(ns) = namespace {
            sql.push_str(" AND m.namespace = ?");
            sql.push_str(&(args.len() + 1).to_string());
            args.push(Box::new(ns.to_string()));
        }
        if let Some(t) = tier {
            sql.push_str(" AND m.tier = ?");
            sql.push_str(&(args.len() + 1).to_string());
            args.push(Box::new(t.to_string()));
        }
        sql.push_str(" ORDER BY rank ASC LIMIT ?");
        sql.push_str(&(args.len() + 1).to_string());
        args.push(Box::new(limit as i64));

        let params_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            Ok(Hit {
                path: row.get(0)?,
                namespace: row.get(1)?,
                tier: row.get(2)?,
                title: row.get(3)?,
                score: {
                    let r: f64 = row.get(4)?;
                    // bm25 returns negative-ish scores; invert for a 0..1-ish feel.
                    (1.0 / (1.0 + (-r).exp())).clamp(0.0, 1.0)
                },
                snippet: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Scan the vault, parse every markdown file, and (re)populate the index.
    pub fn rebuild(&mut self, vault: &Vault) -> Result<usize> {
        self.conn.execute("DELETE FROM memories", [])?;
        self.conn.execute("DELETE FROM memories_fts", [])?;
        let mut n = 0;
        for rel in vault.walk_memories()? {
            let src = vault.read(&rel.to_string_lossy())?;
            let parsed = parse_file(&src);
            let rel_s = rel.to_string_lossy().to_string();
            let ns = parsed
                .namespace
                .clone()
                .unwrap_or_else(|| guess_namespace(&rel_s));
            let tier = parsed
                .tier
                .clone()
                .unwrap_or_else(|| guess_tier(&rel_s));
            let title = parsed
                .title
                .clone()
                .unwrap_or_else(|| guess_title(&rel_s));
            let tags = parsed.tags.join(",");
            let created = parsed
                .created
                .clone()
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

            self.conn.execute(
                "INSERT INTO memories(path, namespace, tier, title, tags, created)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![&rel_s, &ns, &tier, &title, &tags, &created],
            )?;
            self.conn.execute(
                "INSERT INTO memories_fts(path, title, body, tags) VALUES (?1, ?2, ?3, ?4)",
                params![&rel_s, &title, &parsed.body, &parsed.tags.join(" ")],
            )?;
            n += 1;
        }
        Ok(n)
    }
}

fn guess_namespace(path: &str) -> String {
    // namespaces/<ns>/...
    let parts: Vec<_> = path.split('/').collect();
    if parts.len() >= 2 && parts[0] == "namespaces" {
        parts[1].to_string()
    } else {
        "default".to_string()
    }
}

fn guess_tier(path: &str) -> String {
    let parts: Vec<_> = path.split('/').collect();
    if parts.len() >= 3 && parts[0] == "namespaces" {
        parts[2].to_string()
    } else {
        "facts".to_string()
    }
}

fn guess_title(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string()
}

/// FTS5 is picky about operators in user input. For a tiny MVP, wrap each
/// token in quotes so colons / dashes / etc. don't blow up. A richer query
/// grammar can come later.
fn fts_escape(q: &str) -> String {
    q.split_whitespace()
        .map(|t| {
            let safe: String = t.chars().filter(|c| *c != '"').collect();
            format!("\"{safe}\"")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

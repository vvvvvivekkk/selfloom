use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::index::Index;
use crate::mcp;
use crate::memory::{Memory, Tier};
use crate::vault::Vault;

#[derive(Parser, Debug)]
#[command(name = "loom", version, about = "Local-first MCP memory layer")]
pub struct Cli {
    /// Path to the vault. Defaults to $SELFLOOM_VAULT or ~/.selfloom/vault
    #[arg(long, global = true)]
    pub vault: Option<PathBuf>,

    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// Create a new vault at the given path (or the default location)
    Init,

    /// Start the MCP server over stdio. This is how Claude Desktop and
    /// Claude Code talk to selfloom.
    Serve,

    /// Full-text search the vault
    Search {
        /// Query string
        query: String,
        /// Max results
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
        /// Namespace to restrict to
        #[arg(short, long)]
        namespace: Option<String>,
        /// Tier to restrict to: facts | rules | incidents | skills
        #[arg(short, long)]
        tier: Option<String>,
    },

    /// Read a memory file by relative path inside the vault
    Read {
        /// e.g. namespaces/default/facts/2026-10-04-foo-abcd.md
        path: String,
    },

    /// Write a memory into a tier directory
    Write {
        /// Memory body (plain markdown)
        body: String,
        /// Short title; also used to slugify the filename
        #[arg(short, long)]
        title: String,
        /// Tier: facts | rules | incidents | skills
        #[arg(short = 'T', long, default_value = "facts")]
        tier: String,
        /// Namespace. Defaults to "default".
        #[arg(short, long, default_value = "default")]
        namespace: String,
        /// Optional tags (comma-separated)
        #[arg(long)]
        tags: Option<String>,
    },

    /// List namespaces in the vault
    Namespaces,

    /// Reindex everything (useful after manual edits)
    Reindex,
}

pub fn run(args: Cli) -> Result<()> {
    let vault_path = resolve_vault_path(args.vault.clone())?;

    match args.cmd {
        Cmd::Init => {
            Vault::init(&vault_path)?;
            eprintln!("Vault ready at {}", vault_path.display());
            eprintln!();
            eprintln!("Next:");
            eprintln!("  export SELFLOOM_VAULT=\"{}\"", vault_path.display());
            eprintln!("  loom serve     # starts the MCP server");
            eprintln!();
            eprintln!("Or wire it into Claude Desktop / Claude Code — see examples/");
        }
        Cmd::Serve => {
            let vault = Vault::open(&vault_path)?;
            let index = Index::open(&vault.index_path())?;
            mcp::serve_stdio(vault, index)?;
        }
        Cmd::Search {
            query,
            limit,
            namespace,
            tier,
        } => {
            let vault = Vault::open(&vault_path)?;
            let index = Index::open(&vault.index_path())?;
            let hits = index.search(&query, limit, namespace.as_deref(), tier.as_deref())?;
            if hits.is_empty() {
                eprintln!("No matches.");
                return Ok(());
            }
            for h in hits {
                println!("{:.3}  {}", h.score, h.path);
                for line in h.snippet.lines() {
                    println!("    {line}");
                }
                println!();
            }
        }
        Cmd::Read { path } => {
            let vault = Vault::open(&vault_path)?;
            let body = vault.read(&path)?;
            print!("{body}");
        }
        Cmd::Write {
            body,
            title,
            tier,
            namespace,
            tags,
        } => {
            let vault = Vault::open(&vault_path)?;
            let mut index = Index::open(&vault.index_path())?;
            let tier_enum = Tier::parse(&tier)?;
            let tags_vec = tags
                .map(|s| {
                    s.split(',')
                        .map(|t| t.trim().to_string())
                        .filter(|t| !t.is_empty())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let mem = Memory::new(&namespace, tier_enum, &title, &body, tags_vec);
            let saved = vault.write(&mem)?;
            index.upsert(&saved)?;
            println!("Wrote {}", saved.relative_path);
        }
        Cmd::Namespaces => {
            let vault = Vault::open(&vault_path)?;
            for ns in vault.list_namespaces()? {
                println!("{ns}");
            }
        }
        Cmd::Reindex => {
            let vault = Vault::open(&vault_path)?;
            let mut index = Index::open(&vault.index_path())?;
            let n = index.rebuild(&vault)?;
            eprintln!("Indexed {n} memories.");
        }
    }
    Ok(())
}

fn resolve_vault_path(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    if let Ok(env) = std::env::var("SELFLOOM_VAULT") {
        return Ok(PathBuf::from(env));
    }
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow!("could not determine home directory"))?;
    Ok(home.join(".selfloom").join("vault"))
}

#[allow(dead_code)]
fn _ensure_vault_exists(p: &PathBuf) -> Result<()> {
    if !p.join(".selfloom").exists() {
        return Err(anyhow!(
            "no vault at {} — run `loom init` first",
            p.display()
        ))
        .context("vault not initialized");
    }
    Ok(())
}

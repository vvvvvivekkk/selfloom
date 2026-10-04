use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::memory::Memory;

/// A vault on disk. Layout:
///
///   <root>/
///     .selfloom/
///       index.sqlite
///       config.toml
///     .git/
///     namespaces/
///       default/
///         facts/ rules/ incidents/ skills/
pub struct Vault {
    pub root: PathBuf,
}

impl Vault {
    pub fn init(root: &Path) -> Result<Self> {
        fs::create_dir_all(root).with_context(|| format!("mkdir {}", root.display()))?;
        fs::create_dir_all(root.join(".selfloom"))?;
        fs::create_dir_all(root.join("namespaces").join("default"))?;
        for tier in ["facts", "rules", "incidents", "skills"] {
            fs::create_dir_all(root.join("namespaces").join("default").join(tier))?;
            // Keep empty dirs tracked by git with a .gitkeep.
            let keep = root
                .join("namespaces")
                .join("default")
                .join(tier)
                .join(".gitkeep");
            if !keep.exists() {
                fs::write(&keep, b"")?;
            }
        }

        // Minimal config file. Users rarely touch this.
        let cfg_path = root.join(".selfloom").join("config.toml");
        if !cfg_path.exists() {
            fs::write(
                &cfg_path,
                b"# selfloom vault config\nversion = 1\n",
            )?;
        }

        // .gitignore inside the vault — keep the index out of git.
        let gi = root.join(".gitignore");
        if !gi.exists() {
            fs::write(&gi, b".selfloom/index.sqlite\n.selfloom/index.sqlite-*\n")?;
        }

        // git init if needed, with a first commit so later commits don't fail.
        let git_dir = root.join(".git");
        if !git_dir.exists() {
            run_git(root, &["init", "--quiet", "--initial-branch=main"])
                .or_else(|_| run_git(root, &["init", "--quiet"]))?;
            // Make sure git has an identity — fall back to a vault-local one.
            let _ = run_git(root, &["config", "user.email", "selfloom@localhost"]);
            let _ = run_git(root, &["config", "user.name", "selfloom"]);
            run_git(root, &["add", "-A"])?;
            run_git(root, &["commit", "-m", "init: selfloom vault", "--quiet"])?;
        }

        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    pub fn open(root: &Path) -> Result<Self> {
        if !root.join(".selfloom").exists() {
            return Err(anyhow!(
                "no vault at {} — run `loom init` first",
                root.display()
            ));
        }
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    pub fn index_path(&self) -> PathBuf {
        self.root.join(".selfloom").join("index.sqlite")
    }

    /// Write a memory to disk, create the namespace/tier dirs if missing,
    /// commit the change. Returns the memory with its final `relative_path`.
    pub fn write(&self, mem: &Memory) -> Result<Memory> {
        let full = self.root.join(&mem.relative_path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&full, mem.render().as_bytes())?;

        // Commit. If nothing changed (identical write), git commit will fail;
        // ignore that case.
        run_git(&self.root, &["add", "--", &mem.relative_path])?;
        let msg = format!(
            "add({}): {}",
            mem.tier.as_dir(),
            mem.title.replace('\n', " ")
        );
        let _ = run_git(&self.root, &["commit", "-m", &msg, "--quiet"]);

        Ok(mem.clone())
    }

    pub fn read(&self, rel: &str) -> Result<String> {
        let p = self.root.join(rel);
        fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))
    }

    pub fn list_namespaces(&self) -> Result<Vec<String>> {
        let ns_dir = self.root.join("namespaces");
        if !ns_dir.exists() {
            return Ok(vec![]);
        }
        let mut out = Vec::new();
        for entry in fs::read_dir(ns_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                out.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        out.sort();
        Ok(out)
    }

    /// Walk the vault and return every memory file (relative paths).
    pub fn walk_memories(&self) -> Result<Vec<PathBuf>> {
        let ns_dir = self.root.join("namespaces");
        let mut out = Vec::new();
        for entry in walkdir::WalkDir::new(&ns_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let p = entry.path();
            if p.is_file() {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.ends_with(".md") && name != ".gitkeep" {
                    if let Ok(rel) = p.strip_prefix(&self.root) {
                        out.push(rel.to_path_buf());
                    }
                }
            }
        }
        Ok(out)
    }
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .with_context(|| format!("spawn git {:?}", args))?;
    if !status.success() {
        return Err(anyhow!("git {:?} exited with {status}", args));
    }
    Ok(())
}

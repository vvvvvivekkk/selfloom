use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// The four tiers a memory can live in. Borrowed from GitLoom's model.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Facts,
    Rules,
    Incidents,
    Skills,
}

impl Tier {
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "facts" | "fact" => Ok(Tier::Facts),
            "rules" | "rule" => Ok(Tier::Rules),
            "incidents" | "incident" | "event" | "events" => Ok(Tier::Incidents),
            "skills" | "skill" => Ok(Tier::Skills),
            other => Err(anyhow!("unknown tier: {other}")),
        }
    }

    pub fn as_dir(&self) -> &'static str {
        match self {
            Tier::Facts => "facts",
            Tier::Rules => "rules",
            Tier::Incidents => "incidents",
            Tier::Skills => "skills",
        }
    }

    #[allow(dead_code)]
    pub fn all() -> [Tier; 4] {
        [Tier::Facts, Tier::Rules, Tier::Incidents, Tier::Skills]
    }
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_dir())
    }
}

/// A memory in-flight. Once written, `relative_path` is populated.
#[derive(Debug, Clone)]
pub struct Memory {
    pub id: String,
    pub namespace: String,
    pub tier: Tier,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub created: DateTime<Utc>,
    pub relative_path: String, // relative to vault root
}

impl Memory {
    pub fn new(namespace: &str, tier: Tier, title: &str, body: &str, tags: Vec<String>) -> Self {
        let created = Utc::now();
        let id = short_id(&created, title);
        let slug = slugify(title);
        let date = created.format("%Y-%m-%d");
        let relative_path = format!(
            "namespaces/{}/{}/{}-{}-{}.md",
            namespace,
            tier.as_dir(),
            date,
            slug,
            id
        );
        Self {
            id,
            namespace: namespace.to_string(),
            tier,
            title: title.to_string(),
            body: body.to_string(),
            tags,
            created,
            relative_path,
        }
    }

    /// Full markdown with YAML frontmatter.
    pub fn render(&self) -> String {
        let tags_yaml = if self.tags.is_empty() {
            "[]".to_string()
        } else {
            format!(
                "[{}]",
                self.tags
                    .iter()
                    .map(|t| format!("\"{}\"", t.replace('"', "\\\"")))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        format!(
            "---\nid: {}\nnamespace: {}\ntier: {}\ntitle: \"{}\"\ntags: {}\ncreated: {}\n---\n\n{}\n",
            self.id,
            self.namespace,
            self.tier,
            self.title.replace('"', "\\\""),
            tags_yaml,
            self.created.to_rfc3339(),
            self.body.trim_end()
        )
    }
}

fn short_id(dt: &DateTime<Utc>, title: &str) -> String {
    // Deterministic-ish, short, filename-safe. Not cryptographic.
    let seed = format!("{}:{}", dt.timestamp_nanos_opt().unwrap_or_default(), title);
    let mut h: u64 = 1469598103934665603;
    for b in seed.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    format!("{:08x}", (h & 0xFFFFFFFF) as u32)
}

pub fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_dash = false;
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_end_matches('-');
    let truncated: String = trimmed.chars().take(50).collect();
    if truncated.is_empty() {
        "untitled".to_string()
    } else {
        truncated
    }
}

/// Very small frontmatter parser — reads what Memory::render wrote. Not a
/// general YAML parser; keeps the binary tiny.
#[derive(Debug, Clone, Default)]
pub struct ParsedFile {
    pub id: Option<String>,
    pub namespace: Option<String>,
    pub tier: Option<String>,
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub created: Option<String>,
    pub body: String,
}

pub fn parse_file(src: &str) -> ParsedFile {
    let mut out = ParsedFile::default();
    let mut rest = src;

    if let Some(stripped) = rest.strip_prefix("---\n") {
        if let Some(end) = stripped.find("\n---") {
            let frontmatter = &stripped[..end];
            for line in frontmatter.lines() {
                let line = line.trim();
                if let Some((k, v)) = line.split_once(':') {
                    let k = k.trim();
                    let v = v.trim().trim_matches('"');
                    match k {
                        "id" => out.id = Some(v.to_string()),
                        "namespace" => out.namespace = Some(v.to_string()),
                        "tier" => out.tier = Some(v.to_string()),
                        "title" => out.title = Some(v.to_string()),
                        "created" => out.created = Some(v.to_string()),
                        "tags" => {
                            // naive [a, b, c] parse
                            let inner = v.trim_start_matches('[').trim_end_matches(']');
                            out.tags = inner
                                .split(',')
                                .map(|t| t.trim().trim_matches('"').to_string())
                                .filter(|t| !t.is_empty())
                                .collect();
                        }
                        _ => {}
                    }
                }
            }
            // body is after "---\n" at end + 4
            rest = &stripped[end + 4..];
            if let Some(r) = rest.strip_prefix('\n') {
                rest = r;
            }
        }
    }
    out.body = rest.to_string();
    out
}

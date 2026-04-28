use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

/// A validated slug — path relative to wiki root, no extension.
///
/// Invariants enforced at construction:
/// - No `../` path traversal
/// - No file extension
/// - No leading `/`
/// - Non-empty
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Slug(String);

impl Slug {
    /// Derive a slug from a file path relative to wiki root.
    ///
    /// - `concepts/moe.md` → `concepts/moe`
    /// - `concepts/moe/index.md` → `concepts/moe`
    pub fn from_path(path: &Path, wiki_root: &Path) -> Result<Self> {
        let rel = path
            .strip_prefix(wiki_root)
            .map_err(|_| anyhow::anyhow!("path is not under wiki root"))?;
        let raw = if rel.file_name() == Some(std::ffi::OsStr::new("index.md")) {
            rel.parent()
                .ok_or_else(|| anyhow::anyhow!("index.md has no parent"))?
                .to_string_lossy()
                .into_owned()
        } else {
            rel.with_extension("").to_string_lossy().into_owned()
        };
        Self::try_from(raw.as_str())
    }

    /// Resolve this slug to a file path. Checks flat then bundle.
    ///
    /// 1. `<wiki_root>/<slug>.md`
    /// 2. `<wiki_root>/<slug>/index.md`
    pub fn resolve(&self, wiki_root: &Path) -> Result<PathBuf> {
        let flat = wiki_root.join(format!("{}.md", self.0));
        if flat.is_file() {
            return Ok(flat);
        }
        let bundle = wiki_root.join(&self.0).join("index.md");
        if bundle.is_file() {
            return Ok(bundle);
        }
        bail!("page not found for slug: {}", self.0)
    }

    /// Derive a display title from the last slug segment.
    ///
    /// `concepts/mixture-of-experts` → `Mixture of Experts`
    pub fn title(&self) -> String {
        let last = self.0.rsplit('/').next().unwrap_or(&self.0);
        title_case(last)
    }

    /// Return the raw slug string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for Slug {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self> {
        let s = s.trim();
        if s.is_empty() {
            bail!("slug cannot be empty");
        }
        if s.starts_with('/') {
            bail!("slug cannot start with /: {s}");
        }
        if s.contains("../") || s.contains("..\\") {
            bail!("slug cannot contain path traversal: {s}");
        }
        // Reject if the last segment has a file extension
        if let Some(last) = s.rsplit('/').next()
            && let Some(dot) = last.rfind('.')
        {
            let ext = &last[dot + 1..];
            if !ext.is_empty() {
                bail!("slug cannot have a file extension: {s}");
            }
        }
        Ok(Slug(s.to_string()))
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Slug {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A parsed `wiki://` URI or bare slug.
///
/// `wiki://research/concepts/moe` → wiki: Some("research"), slug: "concepts/moe"
/// `concepts/moe` → wiki: None, slug: "concepts/moe"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiUri {
    /// Candidate wiki name — None for bare slugs.
    /// At parse time this is a candidate; WikiUri::resolve checks
    /// whether it's a registered wiki name.
    pub wiki: Option<String>,
    /// The slug portion.
    pub slug: Slug,
}

impl WikiUri {
    /// Parse a string into a WikiUri. Accepts both `wiki://` URIs and bare slugs.
    pub fn parse(input: &str) -> Result<Self> {
        let input = input.trim();
        if let Some(stripped) = input.strip_prefix("wiki://") {
            if stripped.is_empty() {
                bail!("invalid wiki URI: {input}");
            }
            let parts: Vec<&str> = stripped.splitn(2, '/').collect();
            if parts.len() == 2 && !parts[1].is_empty() {
                // wiki://candidate/slug — candidate may be wiki name or first slug segment
                Ok(WikiUri {
                    wiki: Some(parts[0].to_string()),
                    slug: Slug::try_from(parts[1])?,
                })
            } else {
                // wiki://slug (no slash, or trailing slash)
                Ok(WikiUri {
                    wiki: None,
                    slug: Slug::try_from(stripped.trim_end_matches('/'))?,
                })
            }
        } else {
            // Bare slug
            Ok(WikiUri {
                wiki: None,
                slug: Slug::try_from(input)?,
            })
        }
    }

    /// Resolve a URI or bare slug against the global config.
    ///
    /// - `wiki://` URIs: try candidate wiki name, fall back to default wiki
    /// - Bare slugs: use `wiki_flag` or default wiki
    ///
    /// Returns `(WikiEntry, Slug)`.
    pub fn resolve(
        input: &str,
        wiki_flag: Option<&str>,
        global: &crate::config::GlobalConfig,
    ) -> Result<(crate::config::WikiEntry, Slug)> {
        use crate::spaces;

        if input.starts_with("wiki://") {
            let parsed = Self::parse(input)?;
            if let Some(ref name) = parsed.wiki {
                if let Ok(entry) = spaces::resolve_name(name, global) {
                    return Ok((entry, parsed.slug));
                }
                // Not a wiki name — treat as slug segment
                let full_slug = format!("{name}/{}", parsed.slug);
                let slug = Slug::try_from(full_slug.as_str())?;
                let default = &global.global.default_wiki;
                let entry = spaces::resolve_name(default, global)?;
                return Ok((entry, slug));
            }
            let default = &global.global.default_wiki;
            let entry = spaces::resolve_name(default, global)?;
            Ok((entry, parsed.slug))
        } else {
            let wiki_name = wiki_flag.unwrap_or(&global.global.default_wiki);
            let entry = spaces::resolve_name(wiki_name, global)?;
            let slug = Slug::try_from(input)?;
            Ok((entry, slug))
        }
    }
}

/// Result of slug vs asset resolution for wiki_content_read.
#[derive(Debug)]
pub enum ReadTarget {
    /// Slug resolved to a page.
    Page(PathBuf),
    /// Slug resolved to a co-located asset: (parent slug, filename).
    Asset(String, String),
}

/// Two-step resolution: try page first, then asset fallback.
///
/// 1. Try `slug.resolve()` → page
/// 2. If the last segment has a non-.md extension, split into parent slug + filename → asset
pub fn resolve_read_target(input: &str, wiki_root: &Path) -> Result<ReadTarget> {
    // Step 1: try as page (may fail if input has an extension)
    if let Ok(slug) = Slug::try_from(input)
        && let Ok(path) = slug.resolve(wiki_root)
    {
        return Ok(ReadTarget::Page(path));
    }

    // Step 2: check last segment for non-.md extension (asset)
    if let Some(pos) = input.rfind('/') {
        let filename = &input[pos + 1..];
        if let Some(dot) = filename.rfind('.') {
            let ext = &filename[dot + 1..];
            if !ext.is_empty() && ext != "md" {
                let parent_slug = &input[..pos];
                let path = wiki_root.join(parent_slug).join(filename);
                if path.is_file() {
                    return Ok(ReadTarget::Asset(
                        parent_slug.to_string(),
                        filename.to_string(),
                    ));
                }
                bail!("asset not found: {input}");
            }
        }
    }

    bail!("page not found: {input}")
}

fn title_case(segment: &str) -> String {
    segment
        .split('-')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    upper + c.as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

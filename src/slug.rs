#![allow(unreachable_pub)]
use std::fmt;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};

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
        // Slugs are always POSIX-style. Replace the OS path separator with `/`
        // so nested pages get forward-slash slugs on every platform.
        let raw = raw.replace(std::path::MAIN_SEPARATOR, "/");
        Self::try_from(raw.as_str())
    }

    /// Resolve this slug to a file path. Checks flat then bundle.
    ///
    /// 1. `<wiki_root>/<slug>.md`
    /// 2. `<wiki_root>/<slug>/index.md`
    ///
    /// The slug invariants prevent `..` traversal, but symlinks inside `wiki_root` are
    /// not followed through `canonicalize`. Callers that accept user-supplied slugs and
    /// perform write operations must canonicalize the resolved path and verify it is still
    /// under `wiki_root` (see `ingest.rs` for the pattern).
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

    /// Normalize this slug: lowercase all path segments.
    /// Returns a [`NormalizedSlug`] that can be safely compared to other normalized slugs.
    pub fn normalize(&self) -> NormalizedSlug {
        NormalizedSlug(self.0.to_lowercase())
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
        if std::path::Path::new(s)
            .components()
            .any(|c| c == Component::ParentDir)
        {
            bail!("slug cannot contain path traversal: {s}");
        }
        if s.split('/').any(|seg| seg.starts_with('.')) {
            bail!("slug cannot contain hidden components: {s}");
        }
        // Reject if the last segment has a file extension (including trailing dot).
        if let Some(last) = s.rsplit('/').next()
            && last.contains('.')
        {
            bail!("slug cannot have a file extension: {s}");
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

/// A slug that has been lowercased and validated.
///
/// Constructable only via [`Slug::normalize`] (for external callers) or
/// `NormalizedSlug::from_normalized` (for internal index reads where the
/// stored value is already known to be normalized).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct NormalizedSlug(String);

impl NormalizedSlug {
    /// Wrap a string that is already known to be normalized.
    /// For internal crate use only — bypasses the normalization step.
    pub(crate) fn from_normalized(s: String) -> Self {
        debug_assert!(
            s == s.to_lowercase(),
            "from_normalized called with non-normalized input: {s:?}"
        );
        NormalizedSlug(s)
    }

    /// Return the normalized slug as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NormalizedSlug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for NormalizedSlug {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl PartialEq<str> for NormalizedSlug {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for NormalizedSlug {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<String> for NormalizedSlug {
    fn eq(&self, other: &String) -> bool {
        self.0 == *other
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

fn default_wiki(global: &crate::config::GlobalConfig) -> anyhow::Result<&str> {
    global.global.default_wiki_opt().ok_or_else(|| {
        anyhow::anyhow!("no default wiki configured — run `llm-wiki spaces set-default <name>`")
    })
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
                let default = default_wiki(global)?;
                let entry = spaces::resolve_name(default, global)?;
                return Ok((entry, slug));
            }
            let default = default_wiki(global)?;
            let entry = spaces::resolve_name(default, global)?;
            Ok((entry, parsed.slug))
        } else {
            let wiki_name = wiki_flag
                .or_else(|| global.global.default_wiki_opt())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "no default wiki configured — run `llm-wiki spaces set-default <name>`"
                    )
                })?;
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
                // Validate parent_slug before any filesystem probe — prevents existence oracle
                // for paths outside wiki_root. Callers must not skip this; it is the only guard.
                Slug::try_from(parent_slug)
                    .with_context(|| format!("asset path has invalid parent slug: {input:?}"))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile;

    // ── Slug construction ─────────────────────────────────────────────────────

    #[test]
    fn slug_rejects_empty() {
        assert!(Slug::try_from("").is_err());
        assert!(Slug::try_from("   ").is_err());
    }

    #[test]
    fn slug_rejects_leading_slash() {
        assert!(Slug::try_from("/concepts/page").is_err());
    }

    #[test]
    fn slug_rejects_path_traversal() {
        assert!(Slug::try_from("../etc/passwd").is_err());
        assert!(Slug::try_from("concepts/../../etc/passwd").is_err());
    }

    #[test]
    fn slug_rejects_hidden_components() {
        assert!(Slug::try_from(".hidden/page").is_err());
        assert!(Slug::try_from("concepts/.dotfile").is_err());
    }

    #[test]
    fn slug_rejects_file_extension() {
        assert!(Slug::try_from("concepts/page.md").is_err());
        assert!(Slug::try_from("concepts/page.txt").is_err());
        // Trailing dot (empty extension) must also be rejected — "concepts/page."
        // previously bypassed the extension check because ext == "".
        assert!(Slug::try_from("concepts/page.").is_err());
        assert!(Slug::try_from("top-level.").is_err());
    }

    #[test]
    fn slug_accepts_valid_paths() {
        assert!(Slug::try_from("concepts/moe").is_ok());
        assert!(Slug::try_from("concepts/mixture-of-experts").is_ok());
        assert!(Slug::try_from("a/b/c").is_ok());
        assert!(Slug::try_from("top-level").is_ok());
    }

    // ── Normalization ─────────────────────────────────────────────────────────

    #[test]
    fn normalize_lowercases_all_segments() {
        let slug = Slug::try_from("Concepts/MOE").unwrap();
        assert_eq!(slug.normalize(), "concepts/moe");
    }

    #[test]
    fn normalize_already_lowercase_is_identity() {
        let slug = Slug::try_from("concepts/moe").unwrap();
        assert_eq!(slug.normalize(), "concepts/moe");
    }

    #[test]
    fn normalize_nested_path() {
        let slug = Slug::try_from("A/B/C-Deep").unwrap();
        assert_eq!(slug.normalize(), "a/b/c-deep");
    }

    // ── NormalizedSlug round-trips ────────────────────────────────────────────

    #[test]
    fn normalized_slug_display_and_as_str() {
        let slug = Slug::try_from("concepts/moe").unwrap().normalize();
        assert_eq!(slug.as_str(), "concepts/moe");
        assert_eq!(slug.to_string(), "concepts/moe");
    }

    #[test]
    fn normalized_slug_partial_eq_str() {
        let slug = Slug::try_from("concepts/moe").unwrap().normalize();
        assert_eq!(slug, "concepts/moe");
        assert_ne!(slug, "concepts/MOE");
    }

    #[test]
    fn normalized_slug_partial_eq_string() {
        let slug = Slug::try_from("concepts/moe").unwrap().normalize();
        assert_eq!(slug, String::from("concepts/moe"));
    }

    // ── resolve_read_target ───────────────────────────────────────────────────

    #[test]
    fn resolve_read_target_rejects_traversal_in_asset_parent() {
        let dir = tempfile::tempdir().unwrap();
        // Attempt: read an asset whose parent slug traverses out of wiki root
        let result = resolve_read_target("../etc/passwd.pub", dir.path());
        assert!(
            result.is_err(),
            "traversal asset path must be rejected before any filesystem probe"
        );
        let result2 = resolve_read_target("concepts/../../etc/passwd.pdf", dir.path());
        assert!(
            result2.is_err(),
            "multi-component traversal asset path must be rejected"
        );
    }

    #[test]
    fn resolve_read_target_valid_asset_not_found_returns_err() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve_read_target("concepts/diagram.png", dir.path());
        assert!(result.is_err(), "asset not on disk must return Err");
    }

    // ── from_normalized (internal) ────────────────────────────────────────────

    #[test]
    fn from_normalized_wraps_without_transformation() {
        // Simulates what search.rs does when reading slugs from the Tantivy index.
        // The index stores pre-normalized (already lowercase) slugs, so no
        // re-normalization is needed.
        let s = NormalizedSlug::from_normalized("concepts/moe".to_string());
        assert_eq!(s.as_str(), "concepts/moe");
    }
}

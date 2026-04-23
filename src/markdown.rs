use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::frontmatter;
use crate::slug::Slug;

/// Read a page by slug. Optionally strip frontmatter.
/// Appends a supersession notice if `superseded_by` is set.
pub fn read_page(slug: &Slug, wiki_root: &Path, no_frontmatter: bool) -> Result<String> {
    let path = slug.resolve(wiki_root)?;
    let content = std::fs::read_to_string(&path)?;

    let page = frontmatter::parse(&content);
    let notice = page
        .superseded_by()
        .map(|s| format!("\n> **Superseded** by [{s}](wiki://{s})\n"));

    if no_frontmatter {
        let body = &page.body;
        let mut out = body.to_string();
        if let Some(n) = notice {
            out.push_str(&n);
        }
        Ok(out)
    } else {
        let mut out = content;
        if let Some(n) = notice {
            out.push_str(&n);
        }
        Ok(out)
    }
}

/// Write content to a page path resolved from slug.
/// Creates parent directories if needed. Does not validate or commit.
pub fn write_page(slug: &str, content: &str, wiki_root: &Path) -> Result<PathBuf> {
    // Try to resolve existing page first
    if let Ok(s) = Slug::try_from(slug) {
        if let Ok(path) = s.resolve(wiki_root) {
            std::fs::write(&path, content)?;
            return Ok(path);
        }
    }

    // New file — write as flat page
    let path = wiki_root.join(format!("{slug}.md"));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, content)?;
    Ok(path)
}

/// List co-located assets of a bundle page.
pub fn list_assets(slug: &Slug, wiki_root: &Path) -> Result<Vec<String>> {
    let bundle_dir = wiki_root.join(slug.as_str());
    if !bundle_dir.is_dir() || !bundle_dir.join("index.md").is_file() {
        return Ok(Vec::new());
    }
    let mut assets = Vec::new();
    for entry in std::fs::read_dir(&bundle_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name != "index.md" && entry.file_type()?.is_file() {
            assets.push(format!("wiki://{slug}/{name}"));
        }
    }
    assets.sort();
    Ok(assets)
}

/// Read raw bytes of a co-located asset.
pub fn read_asset(slug: &Slug, filename: &str, wiki_root: &Path) -> Result<Vec<u8>> {
    let path = wiki_root.join(slug.as_str()).join(filename);
    if !path.is_file() {
        bail!("asset not found: {slug}/{filename}");
    }
    Ok(std::fs::read(&path)?)
}

/// Create a new page with scaffolded frontmatter.
///
/// - `name_override`: override the title (default: derived from slug)
/// - `type_override`: override the type (default: "page")
/// - `bundle`: create as folder + index.md instead of flat file
///
/// Auto-creates missing parent sections with `type: section`.
pub fn create_page(
    slug: &Slug,
    bundle: bool,
    wiki_root: &Path,
    name_override: Option<&str>,
    type_override: Option<&str>,
    body_template: Option<&str>,
) -> Result<PathBuf> {
    let slug_str = slug.as_str();

    // Auto-create parent sections
    let parts: Vec<&str> = slug_str.split('/').collect();
    if parts.len() > 1 {
        for i in 1..parts.len() {
            let parent_slug = parts[..i].join("/");
            let parent_dir = wiki_root.join(&parent_slug);
            if !parent_dir.exists() {
                std::fs::create_dir_all(&parent_dir)?;
                let parent_s = Slug::try_from(parent_slug.as_str())?;
                let fm = frontmatter::scaffold(&parent_s, true);
                let content = frontmatter::write(&fm, "");
                std::fs::write(parent_dir.join("index.md"), content)?;
            }
        }
    }

    let mut fm = frontmatter::scaffold(slug, false);
    if let Some(name) = name_override {
        fm.insert("title".into(), serde_yaml::Value::String(name.to_string()));
    }
    if let Some(t) = type_override {
        fm.insert("type".into(), serde_yaml::Value::String(t.to_string()));
    }
    let body = body_template.unwrap_or("");
    let content = frontmatter::write(&fm, body);

    let path = if bundle {
        let dir = wiki_root.join(slug_str);
        std::fs::create_dir_all(&dir)?;
        let p = dir.join("index.md");
        std::fs::write(&p, content)?;
        p
    } else {
        if let Some(parent) = wiki_root.join(slug_str).parent() {
            std::fs::create_dir_all(parent)?;
        }
        let p = wiki_root.join(format!("{slug_str}.md"));
        std::fs::write(&p, content)?;
        p
    };

    Ok(path)
}

/// Create a new section (directory + index.md with type: section).
pub fn create_section(
    slug: &Slug,
    wiki_root: &Path,
    body_template: Option<&str>,
) -> Result<PathBuf> {
    let dir = wiki_root.join(slug.as_str());
    std::fs::create_dir_all(&dir)?;

    let fm = frontmatter::scaffold(slug, true);
    let body = body_template.unwrap_or("");
    let content = frontmatter::write(&fm, body);
    let path = dir.join("index.md");
    std::fs::write(&path, content)?;
    Ok(path)
}

/// Promote a flat page to a bundle (move .md into folder/index.md).
pub fn promote_to_bundle(slug: &Slug, wiki_root: &Path) -> Result<()> {
    let flat = wiki_root.join(format!("{}.md", slug.as_str()));
    if !flat.is_file() {
        bail!("flat page not found for slug: {slug}");
    }
    let bundle_dir = wiki_root.join(slug.as_str());
    std::fs::create_dir_all(&bundle_dir)?;
    let dest = bundle_dir.join("index.md");
    std::fs::rename(&flat, &dest)?;
    Ok(())
}

/// Delete a page from disk. Handles both flat (.md) and bundle (slug/index.md) formats.
/// Returns true if a file was deleted, false if the page was not found.
pub fn delete_page(slug: &str, wiki_root: &Path) -> Result<bool> {
    // Try flat format: slug.md
    let flat_path = wiki_root.join(format!("{slug}.md"));
    if flat_path.exists() {
        std::fs::remove_file(&flat_path)?;
        return Ok(true);
    }

    // Try bundle format: slug/index.md
    let bundle_path = wiki_root.join(slug).join("index.md");
    if bundle_path.exists() {
        // Remove the entire bundle directory
        let bundle_dir = wiki_root.join(slug);
        std::fs::remove_dir_all(&bundle_dir)?;
        return Ok(true);
    }

    Ok(false)
}

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::frontmatter::{scaffold_frontmatter, write_frontmatter, PageFrontmatter};

pub fn slug_for(path: &Path, wiki_root: &Path) -> String {
    let rel = path.strip_prefix(wiki_root).unwrap();
    if rel.file_name() == Some(std::ffi::OsStr::new("index.md")) {
        rel.parent().unwrap().to_string_lossy().into()
    } else {
        rel.with_extension("").to_string_lossy().into()
    }
}

pub fn resolve_slug(slug: &str, wiki_root: &Path) -> Result<PathBuf> {
    let flat = wiki_root.join(format!("{slug}.md"));
    if flat.is_file() {
        return Ok(flat);
    }
    let bundle = wiki_root.join(slug).join("index.md");
    if bundle.is_file() {
        return Ok(bundle);
    }
    bail!("page not found for slug: {slug}")
}

pub fn read_page(slug: &str, wiki_root: &Path, no_frontmatter: bool) -> Result<String> {
    let path = resolve_slug(slug, wiki_root)?;
    let content = std::fs::read_to_string(&path)?;
    if no_frontmatter {
        if let Some(rest) = content.strip_prefix("---") {
            if let Some(close) = rest.find("\n---") {
                let after = &rest[close + 4..];
                let body = after
                    .strip_prefix("\r\n")
                    .or_else(|| after.strip_prefix('\n'))
                    .unwrap_or(after);
                return Ok(body.to_string());
            }
        }
    }
    Ok(content)
}

pub fn list_assets(slug: &str, wiki_root: &Path) -> Result<Vec<String>> {
    let bundle_dir = wiki_root.join(slug);
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

pub fn read_asset(slug: &str, filename: &str, wiki_root: &Path) -> Result<Vec<u8>> {
    let path = wiki_root.join(slug).join(filename);
    if !path.is_file() {
        bail!("asset not found: {slug}/{filename}");
    }
    Ok(std::fs::read(&path)?)
}

pub fn promote_to_bundle(slug: &str, wiki_root: &Path) -> Result<()> {
    let flat = wiki_root.join(format!("{slug}.md"));
    if !flat.is_file() {
        bail!("flat page not found for slug: {slug}");
    }
    let bundle_dir = wiki_root.join(slug);
    std::fs::create_dir_all(&bundle_dir)?;
    let dest = bundle_dir.join("index.md");
    std::fs::rename(&flat, &dest)?;
    Ok(())
}

pub fn create_page(slug: &str, bundle: bool, wiki_root: &Path) -> Result<PathBuf> {
    // Auto-create parent sections
    let parts: Vec<&str> = slug.split('/').collect();
    if parts.len() > 1 {
        for i in 1..parts.len() {
            let parent_slug = parts[..i].join("/");
            let parent_dir = wiki_root.join(&parent_slug);
            if !parent_dir.exists() {
                std::fs::create_dir_all(&parent_dir)?;
                let section_fm = PageFrontmatter {
                    title: title_case(parts[i - 1]),
                    summary: String::new(),
                    status: "draft".into(),
                    last_updated: chrono::Local::now().format("%Y-%m-%d").to_string(),
                    r#type: "section".into(),
                    ..Default::default()
                };
                let content = write_frontmatter(&section_fm, "");
                std::fs::write(parent_dir.join("index.md"), content)?;
            }
        }
    }

    let fm = scaffold_frontmatter(slug);
    let content = write_frontmatter(&fm, "");

    let path = if bundle {
        let dir = wiki_root.join(slug);
        std::fs::create_dir_all(&dir)?;
        let p = dir.join("index.md");
        std::fs::write(&p, content)?;
        p
    } else {
        if let Some(parent) = wiki_root.join(slug).parent() {
            std::fs::create_dir_all(parent)?;
        }
        let p = wiki_root.join(format!("{slug}.md"));
        std::fs::write(&p, content)?;
        p
    };

    Ok(path)
}

pub fn create_section(slug: &str, wiki_root: &Path) -> Result<PathBuf> {
    let dir = wiki_root.join(slug);
    std::fs::create_dir_all(&dir)?;

    let fm = PageFrontmatter {
        title: title_case(slug.rsplit('/').next().unwrap_or(slug)),
        summary: String::new(),
        status: "draft".into(),
        last_updated: chrono::Local::now().format("%Y-%m-%d").to_string(),
        r#type: "section".into(),
        ..Default::default()
    };
    let content = write_frontmatter(&fm, "");
    let path = dir.join("index.md");
    std::fs::write(&path, content)?;
    Ok(path)
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

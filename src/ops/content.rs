use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::config;
use crate::engine::EngineState;
use crate::git;
use crate::markdown;
use crate::slug::{resolve_read_target, ReadTarget, Slug, WikiUri};

pub enum ContentReadResult {
    Page(String),
    Assets(Vec<String>),
    Binary,
}

pub fn content_read(
    engine: &EngineState,
    uri: &str,
    wiki_flag: Option<&str>,
    no_frontmatter: bool,
    list_assets: bool,
) -> Result<ContentReadResult> {
    let (entry, slug) = WikiUri::resolve(uri, wiki_flag, &engine.config)?;
    let wiki_root = PathBuf::from(&entry.path).join("wiki");

    if list_assets {
        let assets = markdown::list_assets(&slug, &wiki_root)?;
        return Ok(ContentReadResult::Assets(assets));
    }

    match resolve_read_target(slug.as_str(), &wiki_root)? {
        ReadTarget::Page(_) => {
            let wiki_cfg = config::load_wiki(&PathBuf::from(&entry.path)).unwrap_or_default();
            let resolved = config::resolve(&engine.config, &wiki_cfg);
            let strip = no_frontmatter || resolved.read.no_frontmatter;
            let content = markdown::read_page(&slug, &wiki_root, strip)?;
            Ok(ContentReadResult::Page(content))
        }
        ReadTarget::Asset(parent_slug, filename) => {
            let parent = Slug::try_from(parent_slug.as_str())?;
            let bytes = markdown::read_asset(&parent, &filename, &wiki_root)?;
            match String::from_utf8(bytes) {
                Ok(text) => Ok(ContentReadResult::Page(text)),
                Err(_) => Ok(ContentReadResult::Binary),
            }
        }
    }
}

pub struct WriteResult {
    pub bytes_written: usize,
    pub path: PathBuf,
}

pub fn content_write(
    engine: &EngineState,
    uri: &str,
    wiki_flag: Option<&str>,
    content: &str,
) -> Result<WriteResult> {
    let (_entry, slug) = WikiUri::resolve(uri, wiki_flag, &engine.config)?;
    let wiki_root = PathBuf::from(&_entry.path).join("wiki");
    let path = markdown::write_page(slug.as_str(), content, &wiki_root)?;
    Ok(WriteResult {
        bytes_written: content.len(),
        path,
    })
}

pub fn content_new(
    engine: &EngineState,
    uri: &str,
    wiki_flag: Option<&str>,
    section: bool,
    bundle: bool,
    name: Option<&str>,
    type_: Option<&str>,
) -> Result<String> {
    let (entry, slug) = WikiUri::resolve(uri, wiki_flag, &engine.config)?;
    let wiki_root = PathBuf::from(&entry.path).join("wiki");

    if section {
        markdown::create_section(&slug, &wiki_root)?;
    } else {
        markdown::create_page(&slug, bundle, &wiki_root, name, type_)?;
    }
    Ok(format!("wiki://{}/{slug}", entry.name))
}

pub fn content_commit(
    engine: &EngineState,
    wiki_name: &str,
    slugs: &[String],
    all: bool,
    message: Option<&str>,
) -> Result<String> {
    let space = engine.space(wiki_name)?;

    if slugs.is_empty() && !all {
        bail!("specify slugs or --all");
    }

    if all {
        let msg = message.unwrap_or("commit: all");
        return git::commit(&space.repo_root, msg);
    }

    let mut paths = Vec::new();
    for s in slugs {
        let slug = Slug::try_from(s.as_str())?;
        let resolved = slug.resolve(&space.wiki_root)?;
        if resolved.file_name() == Some(std::ffi::OsStr::new("index.md")) {
            let bundle_dir = resolved.parent().unwrap();
            for entry in walkdir::WalkDir::new(bundle_dir)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.path().is_file() {
                    paths.push(entry.path().to_path_buf());
                }
            }
        } else {
            paths.push(resolved);
        }
    }
    let path_refs: Vec<&Path> = paths.iter().map(|p| p.as_path()).collect();
    let default_msg = format!("commit: {}", slugs.join(", "));
    let msg = message.unwrap_or(&default_msg);
    git::commit_paths(&space.repo_root, &path_refs, msg)
}

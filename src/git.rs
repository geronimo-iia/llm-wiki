use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use git2::{Delta, Repository, Signature};

pub fn init_repo(path: &Path) -> Result<()> {
    Repository::init(path)
        .with_context(|| format!("failed to init git repo at {}", path.display()))?;
    Ok(())
}

fn make_signature(repo: &Repository) -> Result<Signature<'_>> {
    repo.signature()
        .or_else(|_| Signature::now("llm-wiki", "llm-wiki@localhost"))
        .context("failed to create git signature")
}

/// Stage all files and commit. Returns empty string if nothing to commit.
pub fn commit(repo_root: &Path, message: &str) -> Result<String> {
    let repo = Repository::open(repo_root)
        .with_context(|| format!("failed to open repo at {}", repo_root.display()))?;

    let sig = make_signature(&repo)?;
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());

    // Skip if tree matches parent (nothing changed)
    if let Some(ref p) = parent {
        if p.tree_id() == tree_oid {
            return Ok(String::new());
        }
    }

    let parents: Vec<&git2::Commit> = parent.iter().collect();
    let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;
    Ok(oid.to_string())
}

/// Stage specific paths and commit. Returns empty string if nothing to commit.
pub fn commit_paths(repo_root: &Path, paths: &[&Path], message: &str) -> Result<String> {
    let repo = Repository::open(repo_root)
        .with_context(|| format!("failed to open repo at {}", repo_root.display()))?;

    let sig = make_signature(&repo)?;
    let mut index = repo.index()?;
    for path in paths {
        let rel = path.strip_prefix(repo_root).unwrap_or(path);
        index.add_path(rel)?;
    }
    index.write()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());

    if let Some(ref p) = parent {
        if p.tree_id() == tree_oid {
            return Ok(String::new());
        }
    }

    let parents: Vec<&git2::Commit> = parent.iter().collect();
    let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;
    Ok(oid.to_string())
}

/// Get current HEAD commit hash. Returns None if repo has no commits.
pub fn current_head(repo_root: &Path) -> Option<String> {
    let repo = Repository::open(repo_root).ok()?;
    let head = repo.head().ok()?.peel_to_commit().ok()?;
    Some(head.id().to_string())
}

// ── Change detection ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ChangedFile {
    pub path: PathBuf,
    pub status: Delta,
}

/// Detect changed `.md` files under `wiki/` in the working tree vs HEAD.
pub fn changed_wiki_files(repo_root: &Path, wiki_root: &Path) -> Result<Vec<ChangedFile>> {
    let repo = Repository::open(repo_root)
        .with_context(|| format!("failed to open repo at {}", repo_root.display()))?;
    let head_tree = repo
        .head()
        .and_then(|h| h.peel_to_tree())
        .context("no HEAD commit")?;
    let mut opts = git2::DiffOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    let diff = repo.diff_tree_to_workdir_with_index(Some(&head_tree), Some(&mut opts))?;
    let prefix = wiki_root
        .strip_prefix(repo_root)
        .unwrap_or(Path::new("wiki"));
    Ok(collect_md_changes(&diff, prefix))
}

/// Detect changed `.md` files under `wiki/` between a past commit and HEAD.
pub fn changed_since_commit(
    repo_root: &Path,
    wiki_root: &Path,
    from_commit: &str,
) -> Result<Vec<ChangedFile>> {
    let repo = Repository::open(repo_root)
        .with_context(|| format!("failed to open repo at {}", repo_root.display()))?;
    let from_oid = git2::Oid::from_str(from_commit)
        .with_context(|| format!("invalid commit hash: {from_commit}"))?;
    let from_tree = repo.find_commit(from_oid)?.tree()?;
    let head_tree = repo
        .head()
        .and_then(|h| h.peel_to_tree())
        .context("no HEAD commit")?;
    let diff = repo.diff_tree_to_tree(Some(&from_tree), Some(&head_tree), None)?;
    let prefix = wiki_root
        .strip_prefix(repo_root)
        .unwrap_or(Path::new("wiki"));
    Ok(collect_md_changes(&diff, prefix))
}

fn collect_md_changes(diff: &git2::Diff, wiki_prefix: &Path) -> Vec<ChangedFile> {
    let mut changes = Vec::new();
    diff.foreach(
        &mut |delta, _| {
            let path = delta.new_file().path().or_else(|| delta.old_file().path());
            if let Some(p) = path {
                if p.starts_with(wiki_prefix)
                    && p.extension().and_then(|e| e.to_str()) == Some("md")
                {
                    changes.push(ChangedFile {
                        path: p.to_path_buf(),
                        status: delta.status(),
                    });
                }
            }
            true
        },
        None,
        None,
        None,
    )
    .ok();
    changes
}

/// Collect all changed `.md` files by merging two git diffs:
/// - Working tree vs HEAD (uncommitted changes)
/// - `last_indexed_commit` vs HEAD (commits since last index update)
///
/// Working tree changes overwrite commit-based changes on duplicates.
pub fn collect_changed_files(
    repo_root: &Path,
    wiki_root: &Path,
    last_indexed_commit: Option<&str>,
) -> Result<HashMap<PathBuf, Delta>> {
    let mut changes = HashMap::new();

    // B: last indexed commit vs HEAD (insert first so A wins on duplicates)
    if let Some(from_hash) = last_indexed_commit {
        if let Ok(files) = changed_since_commit(repo_root, wiki_root, from_hash) {
            for f in files {
                changes.insert(f.path, f.status);
            }
        }
    }

    // A: working tree vs HEAD (overwrites B on duplicates)
    if let Ok(files) = changed_wiki_files(repo_root, wiki_root) {
        for f in files {
            changes.insert(f.path, f.status);
        }
    }

    Ok(changes)
}

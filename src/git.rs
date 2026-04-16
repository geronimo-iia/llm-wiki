use std::path::Path;

use anyhow::{Context, Result};
use git2::{Repository, Signature};

pub fn init_repo(path: &Path) -> Result<()> {
    Repository::init(path)
        .with_context(|| format!("failed to init git repo at {}", path.display()))?;
    Ok(())
}

pub fn commit(repo_root: &Path, message: &str) -> Result<String> {
    let repo = Repository::open(repo_root)
        .with_context(|| format!("failed to open repo at {}", repo_root.display()))?;

    let sig = Signature::now("llm-wiki", "wiki@localhost")?;
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();

    let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;
    Ok(oid.to_string())
}

pub fn commit_paths(repo_root: &Path, paths: &[&Path], message: &str) -> Result<String> {
    let repo = Repository::open(repo_root)
        .with_context(|| format!("failed to open repo at {}", repo_root.display()))?;

    let sig = Signature::now("llm-wiki", "wiki@localhost")?;
    let mut index = repo.index()?;
    for path in paths {
        let rel = path
            .strip_prefix(repo_root)
            .unwrap_or(path);
        index.add_path(rel)?;
    }
    index.write()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();

    let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;
    Ok(oid.to_string())
}

pub fn current_head(repo_root: &Path) -> Result<String> {
    let repo = Repository::open(repo_root)?;
    let head = repo.head()?.peel_to_commit()?;
    Ok(head.id().to_string())
}

pub fn diff_last(repo_root: &Path) -> Result<Vec<String>> {
    let repo = Repository::open(repo_root)?;
    let head = repo.head()?.peel_to_commit()?;

    let parent_tree = head.parent(0).ok().and_then(|p| p.tree().ok());
    let head_tree = head.tree()?;

    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&head_tree), None)?;

    let mut files = Vec::new();
    diff.foreach(
        &mut |delta, _| {
            if let Some(path) = delta.new_file().path() {
                files.push(path.to_string_lossy().into_owned());
            }
            true
        },
        None,
        None,
        None,
    )?;

    Ok(files)
}

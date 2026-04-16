use std::fs;

use llm_wiki::git;

#[test]
fn init_repo_creates_git_repository() {
    let dir = tempfile::tempdir().unwrap();
    git::init_repo(dir.path()).unwrap();
    assert!(dir.path().join(".git").exists());
}

#[test]
fn commit_creates_commit_and_returns_hash() {
    let dir = tempfile::tempdir().unwrap();
    git::init_repo(dir.path()).unwrap();
    fs::write(dir.path().join("test.txt"), "hello").unwrap();

    let hash = git::commit(dir.path(), "test commit").unwrap();
    assert!(!hash.is_empty());
    assert_eq!(hash.len(), 40); // SHA-1 hex
}

#[test]
fn current_head_returns_commit_hash() {
    let dir = tempfile::tempdir().unwrap();
    git::init_repo(dir.path()).unwrap();
    fs::write(dir.path().join("test.txt"), "hello").unwrap();
    git::commit(dir.path(), "initial").unwrap();

    let head = git::current_head(dir.path()).unwrap();
    assert_eq!(head.len(), 40);
}

#[test]
fn current_head_matches_commit_hash() {
    let dir = tempfile::tempdir().unwrap();
    git::init_repo(dir.path()).unwrap();
    fs::write(dir.path().join("test.txt"), "hello").unwrap();

    let commit_hash = git::commit(dir.path(), "initial").unwrap();
    let head_hash = git::current_head(dir.path()).unwrap();
    assert_eq!(commit_hash, head_hash);
}

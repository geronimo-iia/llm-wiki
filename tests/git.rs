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
    assert_eq!(hash.len(), 40);
}

#[test]
fn commit_empty_returns_empty_string() {
    let dir = tempfile::tempdir().unwrap();
    git::init_repo(dir.path()).unwrap();
    fs::write(dir.path().join("test.txt"), "hello").unwrap();
    git::commit(dir.path(), "first").unwrap();

    // Nothing changed — should be a no-op
    let hash = git::commit(dir.path(), "empty").unwrap();
    assert!(hash.is_empty());
}

#[test]
fn current_head_returns_commit_hash() {
    let dir = tempfile::tempdir().unwrap();
    git::init_repo(dir.path()).unwrap();
    fs::write(dir.path().join("test.txt"), "hello").unwrap();
    git::commit(dir.path(), "initial").unwrap();

    let head = git::current_head(dir.path());
    assert!(head.is_some());
    assert_eq!(head.unwrap().len(), 40);
}

#[test]
fn current_head_none_on_empty_repo() {
    let dir = tempfile::tempdir().unwrap();
    git::init_repo(dir.path()).unwrap();
    assert!(git::current_head(dir.path()).is_none());
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

#[test]
fn commit_paths_commits_only_specified_files() {
    let dir = tempfile::tempdir().unwrap();
    git::init_repo(dir.path()).unwrap();

    fs::write(dir.path().join("init.txt"), "init").unwrap();
    git::commit(dir.path(), "initial").unwrap();

    fs::write(dir.path().join("a.txt"), "aaa").unwrap();
    fs::write(dir.path().join("b.txt"), "bbb").unwrap();

    let hash =
        git::commit_paths(dir.path(), &[&dir.path().join("a.txt")], "commit a only").unwrap();
    assert_eq!(hash.len(), 40);
}

#[test]
fn commit_paths_empty_returns_empty_string() {
    let dir = tempfile::tempdir().unwrap();
    git::init_repo(dir.path()).unwrap();

    let a = dir.path().join("a.txt");
    fs::write(&a, "aaa").unwrap();
    git::commit_paths(dir.path(), &[a.as_path()], "first").unwrap();

    // Same file, same content — no-op
    let hash = git::commit_paths(dir.path(), &[a.as_path()], "empty").unwrap();
    assert!(hash.is_empty());
}

// ── changed_wiki_files ────────────────────────────────────────────────────────

#[test]
fn changed_wiki_files_detects_new_file() {
    let dir = tempfile::tempdir().unwrap();
    git::init_repo(dir.path()).unwrap();

    let wiki = dir.path().join("wiki");
    fs::create_dir_all(&wiki).unwrap();
    fs::write(wiki.join("init.md"), "---\ntitle: Init\n---\n").unwrap();
    git::commit(dir.path(), "initial").unwrap();

    fs::write(wiki.join("new-page.md"), "---\ntitle: New\n---\n").unwrap();

    let changes = git::changed_wiki_files(dir.path(), &wiki).unwrap();
    assert!(changes.iter().any(|c| c.path.ends_with("new-page.md")));
}

#[test]
fn changed_wiki_files_detects_modified_file() {
    let dir = tempfile::tempdir().unwrap();
    git::init_repo(dir.path()).unwrap();

    let wiki = dir.path().join("wiki");
    fs::create_dir_all(&wiki).unwrap();
    fs::write(wiki.join("page.md"), "---\ntitle: Old\n---\n").unwrap();
    git::commit(dir.path(), "initial").unwrap();

    fs::write(wiki.join("page.md"), "---\ntitle: New\n---\n").unwrap();

    let changes = git::changed_wiki_files(dir.path(), &wiki).unwrap();
    assert!(changes
        .iter()
        .any(|c| c.path.ends_with("page.md") && c.status == git2::Delta::Modified));
}

#[test]
fn changed_wiki_files_detects_deleted_file() {
    let dir = tempfile::tempdir().unwrap();
    git::init_repo(dir.path()).unwrap();

    let wiki = dir.path().join("wiki");
    fs::create_dir_all(&wiki).unwrap();
    fs::write(wiki.join("page.md"), "---\ntitle: Gone\n---\n").unwrap();
    git::commit(dir.path(), "initial").unwrap();

    fs::remove_file(wiki.join("page.md")).unwrap();

    let changes = git::changed_wiki_files(dir.path(), &wiki).unwrap();
    assert!(changes
        .iter()
        .any(|c| c.path.ends_with("page.md") && c.status == git2::Delta::Deleted));
}

#[test]
fn changed_wiki_files_ignores_non_md() {
    let dir = tempfile::tempdir().unwrap();
    git::init_repo(dir.path()).unwrap();

    let wiki = dir.path().join("wiki");
    fs::create_dir_all(&wiki).unwrap();
    fs::write(wiki.join("init.md"), "---\ntitle: Init\n---\n").unwrap();
    git::commit(dir.path(), "initial").unwrap();

    fs::write(wiki.join("image.png"), "fake-png").unwrap();

    let changes = git::changed_wiki_files(dir.path(), &wiki).unwrap();
    assert!(!changes.iter().any(|c| c.path.ends_with("image.png")));
}

#[test]
fn changed_wiki_files_ignores_files_outside_wiki() {
    let dir = tempfile::tempdir().unwrap();
    git::init_repo(dir.path()).unwrap();

    let wiki = dir.path().join("wiki");
    fs::create_dir_all(&wiki).unwrap();
    fs::write(wiki.join("init.md"), "---\ntitle: Init\n---\n").unwrap();
    git::commit(dir.path(), "initial").unwrap();

    fs::write(dir.path().join("README.md"), "# Hello").unwrap();

    let changes = git::changed_wiki_files(dir.path(), &wiki).unwrap();
    assert!(!changes.iter().any(|c| c.path.ends_with("README.md")));
}

// ── changed_since_commit ──────────────────────────────────────────────────────

#[test]
fn changed_since_commit_detects_gap() {
    let dir = tempfile::tempdir().unwrap();
    git::init_repo(dir.path()).unwrap();

    let wiki = dir.path().join("wiki");
    fs::create_dir_all(&wiki).unwrap();
    fs::write(wiki.join("page-a.md"), "---\ntitle: A\n---\n").unwrap();
    let first = git::commit(dir.path(), "first").unwrap();

    fs::write(wiki.join("page-b.md"), "---\ntitle: B\n---\n").unwrap();
    git::commit(dir.path(), "second").unwrap();

    let changes = git::changed_since_commit(dir.path(), &wiki, &first).unwrap();
    assert!(changes.iter().any(|c| c.path.ends_with("page-b.md")));
    assert!(!changes.iter().any(|c| c.path.ends_with("page-a.md")));
}

// ── collect_changed_files ─────────────────────────────────────────────────────

#[test]
fn collect_changed_files_detects_new_file() {
    let dir = tempfile::tempdir().unwrap();
    git::init_repo(dir.path()).unwrap();
    let wiki = dir.path().join("wiki");
    fs::create_dir_all(&wiki).unwrap();
    fs::write(dir.path().join("README.md"), "# test\n").unwrap();
    git::commit(dir.path(), "init").unwrap();

    fs::write(wiki.join("new.md"), "---\ntitle: New\n---\n").unwrap();

    let changes = git::collect_changed_files(dir.path(), &wiki, None).unwrap();
    assert!(!changes.is_empty());
}

#[test]
fn collect_changed_files_empty_when_clean() {
    let dir = tempfile::tempdir().unwrap();
    git::init_repo(dir.path()).unwrap();
    let wiki = dir.path().join("wiki");
    fs::create_dir_all(&wiki).unwrap();
    fs::write(wiki.join("foo.md"), "---\ntitle: Foo\n---\n").unwrap();
    git::commit(dir.path(), "add foo").unwrap();
    let head = git::current_head(dir.path()).unwrap();

    let changes = git::collect_changed_files(dir.path(), &wiki, Some(&head)).unwrap();
    assert!(changes.is_empty());
}

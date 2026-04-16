use llm_wiki::config::*;
use llm_wiki::spaces;

fn make_global(wikis: Vec<WikiEntry>, default: &str) -> GlobalConfig {
    GlobalConfig {
        global: GlobalSection {
            default_wiki: default.into(),
        },
        wikis,
        ..Default::default()
    }
}

fn make_entry(name: &str, path: &str) -> WikiEntry {
    WikiEntry {
        name: name.into(),
        path: path.into(),
        description: None,
        remote: None,
    }
}

#[test]
fn resolve_uri_parses_full_uri() {
    let global = make_global(vec![make_entry("research", "/tmp/research")], "research");
    let (entry, slug) = spaces::resolve_uri("wiki://research/concepts/foo", &global).unwrap();
    assert_eq!(entry.name, "research");
    assert_eq!(slug, "concepts/foo");
}

#[test]
fn resolve_uri_uses_default_wiki_for_short_uri() {
    let global = make_global(vec![make_entry("research", "/tmp/research")], "research");
    let (entry, slug) = spaces::resolve_uri("wiki://concepts/foo", &global).unwrap();
    assert_eq!(entry.name, "research");
    assert_eq!(slug, "concepts/foo");
}

#[test]
fn resolve_uri_returns_error_for_unknown_wiki() {
    let global = make_global(vec![make_entry("research", "/tmp/research")], "");
    let result = spaces::resolve_uri("wiki://unknown/slug", &global);
    assert!(result.is_err());
}

#[test]
fn register_appends_entry() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(&config_path, "[global]\ndefault_wiki = \"\"\n").unwrap();

    let entry = make_entry("test", "/tmp/test");
    spaces::register(entry, false, &config_path).unwrap();

    let config = load_global(&config_path).unwrap();
    assert_eq!(config.wikis.len(), 1);
    assert_eq!(config.wikis[0].name, "test");
}

#[test]
fn register_with_force_updates_existing() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(&config_path, "[global]\ndefault_wiki = \"\"\n").unwrap();

    let entry1 = make_entry("test", "/tmp/test1");
    spaces::register(entry1, false, &config_path).unwrap();

    let entry2 = make_entry("test", "/tmp/test2");
    spaces::register(entry2, true, &config_path).unwrap();

    let config = load_global(&config_path).unwrap();
    assert_eq!(config.wikis.len(), 1);
    assert_eq!(config.wikis[0].path, "/tmp/test2");
}

#[test]
fn register_errors_on_duplicate_without_force() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(&config_path, "[global]\ndefault_wiki = \"\"\n").unwrap();

    let entry = make_entry("test", "/tmp/test");
    spaces::register(entry.clone(), false, &config_path).unwrap();

    let result = spaces::register(entry, false, &config_path);
    assert!(result.is_err());
}

#[test]
fn remove_removes_entry() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(&config_path, "[global]\ndefault_wiki = \"\"\n").unwrap();

    let entry = make_entry("test", "/tmp/test");
    spaces::register(entry, false, &config_path).unwrap();
    spaces::remove("test", false, &config_path).unwrap();

    let config = load_global(&config_path).unwrap();
    assert!(config.wikis.is_empty());
}

#[test]
fn remove_with_delete_removes_directory() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(&config_path, "[global]\ndefault_wiki = \"\"\n").unwrap();

    let wiki_dir = dir.path().join("mywiki");
    std::fs::create_dir_all(&wiki_dir).unwrap();

    let entry = WikiEntry {
        name: "test".into(),
        path: wiki_dir.to_string_lossy().into(),
        description: None,
        remote: None,
    };
    spaces::register(entry, false, &config_path).unwrap();
    spaces::remove("test", true, &config_path).unwrap();

    assert!(!wiki_dir.exists());
}

#[test]
fn remove_errors_when_wiki_is_default() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(
        &config_path,
        "[global]\ndefault_wiki = \"test\"\n\n[[wikis]]\nname = \"test\"\npath = \"/tmp/test\"\n",
    )
    .unwrap();

    let result = spaces::remove("test", false, &config_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("default wiki"));
}

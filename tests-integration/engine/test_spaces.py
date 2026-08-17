def test_spaces_list_returns_both_wikis(wiki_env):
    result = wiki_env.run("spaces", "list")
    assert "research" in result.stdout
    assert "notes" in result.stdout


def test_spaces_list_shows_default_marker(wiki_env):
    result = wiki_env.run("spaces", "list")
    assert "* research" in result.stdout


def test_spaces_list_json_has_research_entry(wiki_env):
    data = wiki_env.json("spaces", "list")
    names = [w["name"] for w in data]
    assert "research" in names


def test_spaces_set_default(wiki_env):
    wiki_env.run("spaces", "set-default", "notes")
    result = wiki_env.run("spaces", "list")
    assert "* notes" in result.stdout
    config_result = wiki_env.run("config", "get", "global.default_wiki")
    assert "notes" in config_result.stdout

    wiki_env.run("spaces", "set-default", "research")
    result = wiki_env.run("spaces", "list")
    assert "* research" in result.stdout
    config_result = wiki_env.run("config", "get", "global.default_wiki")
    assert "research" in config_result.stdout


def test_spaces_register_creates_entry(wiki_env):
    register_dir = wiki_env.tmp / "wikis" / "register-test"
    (register_dir / "content").mkdir(parents=True)

    result = wiki_env.run(
        "spaces",
        "register",
        "--name",
        "register-test",
        "--wiki-root",
        "content",
        "--description",
        "integration test wiki",
        str(register_dir),
    )
    assert "register-test" in result.stdout


def test_spaces_register_creates_wiki_toml(wiki_env):
    register_dir = wiki_env.tmp / "wikis" / "register-test2"
    (register_dir / "content").mkdir(parents=True)
    wiki_env.run(
        "spaces",
        "register",
        "--name",
        "register-test2",
        "--wiki-root",
        "content",
        str(register_dir),
    )
    toml_text = (register_dir / "wiki.toml").read_text()
    assert "register-test2" in toml_text
    assert "wiki_root" in toml_text


def test_spaces_register_creates_dirs(wiki_env):
    register_dir = wiki_env.tmp / "wikis" / "register-test3"
    (register_dir / "content").mkdir(parents=True)
    wiki_env.run(
        "spaces",
        "register",
        "--name",
        "register-test3",
        "--wiki-root",
        "content",
        str(register_dir),
    )
    assert (register_dir / "inbox").is_dir()
    assert (register_dir / "schemas").is_dir()


def test_spaces_remove_unregisters(wiki_env):
    register_dir = wiki_env.tmp / "wikis" / "to-remove"
    (register_dir / "content").mkdir(parents=True)
    wiki_env.run(
        "spaces",
        "register",
        "--name",
        "to-remove",
        "--wiki-root",
        "content",
        str(register_dir),
    )
    result = wiki_env.run("spaces", "remove", "to-remove", "--delete")
    assert "Removed" in result.stdout
    result = wiki_env.run("spaces", "list")
    assert "to-remove" not in result.stdout

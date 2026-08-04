use std::path::Path;
use std::process::Command;

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_llm-wiki"))
}

fn setup_wiki_for_cli(dir: &Path) -> std::path::PathBuf {
    let config_path = dir.join("state").join("config.toml");
    let wiki_path = dir.join("test");

    llm_wiki::spaces::create(&wiki_path, "test", None, false, true, &config_path, None).unwrap();

    let wiki_root = wiki_path.join("wiki");
    std::fs::create_dir_all(wiki_root.join("concepts")).unwrap();
    std::fs::write(
        wiki_root.join("concepts/alpha.md"),
        "---\ntitle: \"Alpha\"\ntype: concept\nstatus: active\n---\nAlpha body.\n",
    )
    .unwrap();
    std::fs::write(
        wiki_root.join("concepts/beta.md"),
        "---\ntitle: \"Beta\"\ntype: concept\nstatus: active\n---\nSee [[concepts/alpha]].\n",
    )
    .unwrap();
    llm_wiki::git::commit(&wiki_path, "add pages").unwrap();

    config_path
}

#[test]
fn config_flag_overrides_default_path() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");
    // Empty TOML is a valid GlobalConfig (all fields have defaults)
    std::fs::write(&config, "").unwrap();

    let out = binary()
        .args(["--config", config.to_str().unwrap(), "spaces", "list"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn llm_wiki_config_env_var_overrides_default_path() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("env-config.toml");
    std::fs::write(&config, "").unwrap();

    let out = binary()
        .env("LLM_WIKI_CONFIG", config.to_str().unwrap())
        // Ensure HOME doesn't accidentally point to a real config
        .env("HOME", dir.path())
        .args(["spaces", "list"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn config_flag_takes_priority_over_env_var() {
    let dir = tempfile::tempdir().unwrap();
    let flag_config = dir.path().join("flag.toml");
    let env_config = dir.path().join("env.toml");
    std::fs::write(&flag_config, "").unwrap();
    std::fs::write(&env_config, "").unwrap();

    let out = binary()
        .args(["--config", flag_config.to_str().unwrap(), "spaces", "list"])
        .env("LLM_WIKI_CONFIG", env_config.to_str().unwrap())
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn graph_not_empty_after_index_rebuild_cli() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki_for_cli(dir.path());

    let rebuild = binary()
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "--wiki",
            "test",
            "index",
            "rebuild",
        ])
        .output()
        .unwrap();
    assert!(
        rebuild.status.success(),
        "index rebuild failed: {}",
        String::from_utf8_lossy(&rebuild.stderr)
    );

    let graph = binary()
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "--wiki",
            "test",
            "graph",
        ])
        .output()
        .unwrap();
    assert!(
        graph.status.success(),
        "graph failed: {}",
        String::from_utf8_lossy(&graph.stderr)
    );

    let stdout = String::from_utf8_lossy(&graph.stdout);
    assert!(
        stdout.contains("Alpha") || stdout.contains("Beta"),
        "graph output must contain page nodes after index rebuild (issue #112 regression): got:\n{stdout}"
    );
    assert!(
        !stdout.trim_end().eq("graph LR") && !stdout.trim_end().eq("digraph wiki {}"),
        "graph output must not be empty header only: got:\n{stdout}"
    );
}

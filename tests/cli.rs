use std::process::Command;

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_llm-wiki"))
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

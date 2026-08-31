import json
import shutil


def test_ingest_dry_run_inbox(wiki_env):
    result = wiki_env.run("ingest", "inbox/", "--dry-run", "--format", "json")
    data = json.loads(result.stdout)
    assert data["pages_validated"] > 0


def test_ingest_single_file_dry_run(wiki_env):
    result = wiki_env.run(
        "ingest", "inbox/01-paper-switch-transformer.md", "--dry-run", "--format", "json"
    )
    data = json.loads(result.stdout)
    assert data["pages_validated"] == 1


def test_ingest_real_file(wiki_env):
    src = wiki_env.inbox / "01-paper-switch-transformer.md"
    dst = wiki_env.inbox / "test-ingest.md"
    shutil.copy(src, dst)
    result = wiki_env.run("ingest", "inbox/test-ingest.md")
    assert "Ingested" in result.stdout


def test_ingest_redact_removes_secret(wiki_env):
    src = wiki_env.inbox / "03-note-with-secrets.md"
    dst = wiki_env.inbox / "secrets-test.md"
    shutil.copy(src, dst)
    wiki_env.run("ingest", "inbox/secrets-test.md", "--redact")
    content = dst.read_text(encoding="utf-8")
    assert "sk-ant-api03" not in content
    assert "REDACTED" in content

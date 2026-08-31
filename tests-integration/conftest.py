import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest

BINARY = os.environ.get("LLM_WIKI_BIN", "llm-wiki")
FIXTURES = Path(__file__).parent.parent / "tests" / "fixtures"


@pytest.fixture(scope="session")
def binary():
    b = shutil.which(BINARY) or BINARY
    result = subprocess.run([b, "--version"], capture_output=True, text=True, encoding="utf-8")
    assert result.returncode == 0, f"binary not found: {b!r} — set LLM_WIKI_BIN"
    return b


def _init_wiki(src: Path, dest: Path) -> None:
    shutil.copytree(src, dest)
    subprocess.run(["git", "-C", str(dest), "init", "-q"], check=True)
    subprocess.run(["git", "-C", str(dest), "add", "."], check=True)
    subprocess.run(
        [
            "git",
            "-C",
            str(dest),
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@test.com",
            "commit",
            "-qm",
            "init",
        ],
        check=True,
    )


class WikiEnv:
    def __init__(self, binary: str, config: Path, research: Path, notes: Path, tmp: Path):
        self.binary = binary
        self.config = config
        self.research = research  # repo root of research wiki
        self.notes = notes  # repo root of notes wiki
        self.tmp = tmp
        self.research_wiki = research / "wiki"  # page content root
        self.notes_wiki = notes / "wiki"
        self.inbox = research / "wiki" / "inbox"

    def run(self, *args: str, check: bool = True) -> subprocess.CompletedProcess:
        result = subprocess.run(
            [self.binary, "--config", str(self.config), *args],
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        if check:
            assert result.returncode == 0, (
                f"command failed: {list(args)}\nstdout: {result.stdout}\nstderr: {result.stderr}"
            )
        return result

    def json(self, *args: str) -> "dict | list":
        result = self.run(*args, "--format", "json")
        return json.loads(result.stdout)


@pytest.fixture()
def wiki_env(binary: str, tmp_path: Path) -> WikiEnv:
    config = tmp_path / "config.toml"
    research = tmp_path / "wikis" / "research"
    notes = tmp_path / "wikis" / "notes"

    _init_wiki(FIXTURES / "wikis" / "research", research)
    _init_wiki(FIXTURES / "wikis" / "notes", notes)

    # Copy inbox fixtures into the research wiki inbox
    inbox = research / "wiki" / "inbox"
    inbox.mkdir(parents=True, exist_ok=True)
    for f in (FIXTURES / "inbox").iterdir():
        shutil.copy(f, inbox / f.name)
    subprocess.run(["git", "-C", str(research), "add", "."], check=True)
    subprocess.run(
        [
            "git",
            "-C",
            str(research),
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@test.com",
            "commit",
            "-qm",
            "add inbox",
            "--allow-empty",
        ],
        check=True,
    )

    env = WikiEnv(binary=binary, config=config, research=research, notes=notes, tmp=tmp_path)

    # Register both wikis; research is the default
    env.run("spaces", "register", "--name", "research", str(research))
    env.run("spaces", "set-default", "research")
    env.run("spaces", "register", "--name", "notes", str(notes))

    return env

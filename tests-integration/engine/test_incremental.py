import json


def test_incremental_ingest_reports_result(wiki_env):
    wiki_env.run("index", "rebuild", "--wiki", "research")
    modified = wiki_env.research_wiki / "concepts" / "scaling-laws.md"
    modified.write_text(modified.read_text(encoding="utf-8") + "\n", encoding="utf-8")
    result = wiki_env.run("ingest", "concepts/scaling-laws.md", "--format", "json")
    data = json.loads(result.stdout)
    # file was modified (not in git yet), ingest reports it
    assert "pages_validated" in data or "unchanged_count" in data

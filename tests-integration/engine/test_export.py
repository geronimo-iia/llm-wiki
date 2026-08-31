import json


def test_export_llms_txt(wiki_env):
    wiki_env.run("index", "rebuild", "--wiki", "research")
    out = wiki_env.tmp / "export-llms.txt"
    wiki_env.run("export", "--path", str(out), "--wiki", "research")
    assert out.exists()
    assert "Mixture of Experts" in out.read_text(encoding="utf-8")


def test_export_json(wiki_env):
    wiki_env.run("index", "rebuild", "--wiki", "research")
    out = wiki_env.tmp / "export.json"
    wiki_env.run("export", "--path", str(out), "--format", "json", "--wiki", "research")
    assert out.exists()
    data = json.loads(out.read_text(encoding="utf-8"))
    assert isinstance(data, list)

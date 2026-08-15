import json

CUSTOM_SCHEMA = json.dumps(
    {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "test-custom",
        "type": "object",
        "x-wiki-types": {"test-custom": {"label": "Test Custom", "fields": []}},
    }
)


def test_schema_list(wiki_env):
    result = wiki_env.run("schema", "list")
    assert "concept" in result.stdout


def test_schema_show(wiki_env):
    result = wiki_env.run("schema", "show", "concept")
    assert "title" in result.stdout


def test_schema_validate(wiki_env):
    wiki_env.run("schema", "validate")


def test_schema_add_and_remove(wiki_env):
    schema_file = wiki_env.tmp / "test-custom.json"
    schema_file.write_text(CUSTOM_SCHEMA)

    result = wiki_env.run("schema", "add", "test-custom", str(schema_file))
    assert "copied" in result.stdout

    result = wiki_env.run("schema", "list")
    assert "test-custom" in result.stdout

    result = wiki_env.run("schema", "remove", "test-custom", "--delete")
    assert "schema file deleted: true" in result.stdout

    result = wiki_env.run("schema", "list")
    assert "test-custom" not in result.stdout

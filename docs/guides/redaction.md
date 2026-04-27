---
title: "Privacy Redaction"
summary: "How to use wiki_ingest redact: true to scrub secrets from page bodies before commit."
---

# Privacy Redaction

`wiki_ingest` can scan page bodies for known secrets and replace them with
placeholder strings before validation and git commit. Redaction is **opt-in**
and **lossy** — the original value is gone after the file is written.

## When to use `redact: true`

Use it when ingesting content from external sources that may contain secrets:

- Web clips and browser captures
- Session transcripts and meeting notes
- Raw notes pasted from external tools
- API documentation that may include example credentials

Do not use it for curated pages you've already reviewed — redaction adds
write overhead and is irreversible.

## Built-in patterns

Always active when `redact: true`. Defined in the engine; no configuration
required.

| Pattern name | What it matches |
|---|---|
| `github-pat` | GitHub personal access tokens (`ghp_…`) |
| `openai-key` | OpenAI API keys (`sk-…` 48 chars) |
| `anthropic-key` | Anthropic API keys (`sk-ant-…` 90+ chars) |
| `aws-access-key` | AWS access key IDs (`AKIA…`) |
| `bearer-token` | HTTP Bearer tokens (20+ chars) |
| `email` | Email addresses (RFC 5322 simplified) |

Replacements use the form `[REDACTED:pattern-name]`.

## Reading the redaction report

The JSON output includes a `redacted` field — a list per file:

```json
{
  "pages_validated": 1,
  "redacted": [
    {
      "slug": "inbox/transcript",
      "matches": [
        { "pattern_name": "github-pat", "line_number": 14 },
        { "pattern_name": "email",      "line_number": 27 }
      ]
    }
  ]
}
```

The report shows the slug, the pattern name, and the absolute line number
in the file (frontmatter + body). It never records the original value.

Text output shows one line per match:

```
redacted: inbox/transcript line 14 [github-pat]
redacted: inbox/transcript line 27 [email]
```

## Disable a built-in pattern per-wiki

Email addresses are legitimate content in a contacts wiki or people directory.
Disable specific patterns in `wiki.toml`:

```toml
[redact]
disable = ["email"]
```

An empty `[redact]` section or no section at all leaves all built-ins active.

## Add custom patterns

Internal hostnames, employee IDs, or project codes that should not leak:

```toml
[[redact.patterns]]
name        = "internal-hostname"
pattern     = "corp\\.internal\\.[a-z]+"
replacement = "[REDACTED:internal-hostname]"

[[redact.patterns]]
name        = "employee-id"
pattern     = "EMP-[0-9]{6}"
replacement = "[REDACTED:employee-id]"
```

Effective set = built-ins minus `disable` plus `[[redact.patterns]]`.

## Scope: body only

Redaction runs on the page body. Frontmatter is structured YAML; redacting
it would likely corrupt the document. Frontmatter redaction is a future
extension.

## Warning: redaction is lossy

The original value is permanently replaced in the file on disk and then
committed to git. There is no recovery path. Review the redaction report
after ingest to verify what was removed, and check that the replacement
placeholders do not break the meaning of the text.

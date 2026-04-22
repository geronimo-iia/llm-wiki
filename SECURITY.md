# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | ✓         |

## Reporting a Vulnerability

Please report security vulnerabilities by email to <jguibert@gmail.com>.

**Do not open a public issue.**

You should receive an acknowledgment within 48 hours. A fix will be
prioritized based on severity and released as a patch version.

## Scope

llm-wiki is a local-first tool. It does not make network requests,
store credentials, or run user-supplied code. The main attack surface is:

- Malicious Markdown files processed by the ingest pipeline
- SSE transport exposed on a network port (`llm-wiki serve --sse`)
- Dependencies (tantivy, git2, rmcp, agent-client-protocol)

Dependency vulnerabilities are tracked via `cargo audit` in CI and
Dependabot alerts.

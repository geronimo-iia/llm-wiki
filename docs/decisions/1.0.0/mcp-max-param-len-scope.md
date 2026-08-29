# `mcp_max_param_len` applies to short params only — content tools are exempt

## Decision

`mcp_max_param_len` guards short string parameters (slugs, queries, wiki names,
section names, format strings). Tools that accept a full page body (`wiki_content_write`,
`wiki_content_new`) are explicitly exempt from this limit; they enforce their own
size cap via `max_content_len`.

## Context

`mcp_max_param_len` (default 8192 bytes) was introduced as a broad safety guard
against oversized MCP tool arguments. It was applied uniformly to all string
parameters, including `content` in `wiki_content_write`.

This caused a conflict: `wiki_content_write` already enforces `max_content_len`
(default 10 MiB) at the write layer. Having a 8192-byte outer cap on `content`
made the tool unusable for any page larger than ~8 KB, which is a normal wiki page.

`wiki_content_new` has the same issue — it accepts an optional initial body.

## Decision rationale

Two separate concerns:

1. **Protocol safety** — prevent a malicious or buggy client from sending a 100 MB
   string as a wiki name or search query. `mcp_max_param_len` covers this.
2. **Content size policy** — operators decide the maximum page size their instance
   accepts. `max_content_len` covers this.

Applying `mcp_max_param_len` to `content` conflates the two and breaks the
intended `max_content_len` configuration. The fix is to skip the
`mcp_max_param_len` check for the `content` parameter in tools that have their own
content-size enforcement.

## Affected tools

| Tool | Exempt param | Own guard |
| ---- | ------------ | --------- |
| `wiki_content_write` | `content` | `max_content_len` (default 10 MiB) |
| `wiki_content_new` | `content` (initial body, optional) | same |

All other tools and all other parameters remain subject to `mcp_max_param_len`.

## Consequences

- Operators tuning `mcp_max_param_len` do not affect page-write capacity.
- Operators tuning `max_content_len` do not need to also raise `mcp_max_param_len`.
- The two limits are orthogonal and documented separately in the config reference.

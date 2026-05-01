# ACP Manual Test Matrix

Connect Zed to `llm-wiki serve --acp`. One session per block.

## research
| Prompt | Expected |
|--------|----------|
| `what is sparse routing?` | search tool call + read tool call + page body + slug list |
| `llm-wiki:research scaling laws` | same via explicit prefix |
| `llm-wiki:research zzz-no-match` | "No results found" message |

## lint
| Prompt | Expected |
|--------|----------|
| `llm-wiki:lint` | tool call + summary line + one line per finding |
| `llm-wiki:lint orphan` | only orphan findings |
| `llm-wiki:lint stale,broken-link` | stale + broken-link findings only |

## graph
| Prompt | Expected |
|--------|----------|
| `llm-wiki:graph` | tool call + "N nodes, M edges" + graph text |
| `llm-wiki:graph concepts/moe` | subgraph from that root |
| `llm-wiki:graph zzz-missing` | error message in tool call result |

## ingest
| Prompt | Expected |
|--------|----------|
| `llm-wiki:ingest` | tool call + summary (pages validated, commit) |
| `llm-wiki:ingest wiki/concepts/test.md` | ingest specific file |
| `llm-wiki:ingest /nonexistent` | tool call Failed + error text |

## use
| Prompt | Expected |
|--------|----------|
| `llm-wiki:use concepts/moe` | tool call Completed + full page markdown streamed |
| `llm-wiki:use` | "Usage: llm-wiki:use <slug>" |
| `llm-wiki:use zzz-missing` | tool call Failed + error text |

## help / unknown
| Prompt | Expected |
|--------|----------|
| `llm-wiki:help` | workflow listing |
| `llm-wiki:bogus` | "Unknown workflow" + workflow listing |
